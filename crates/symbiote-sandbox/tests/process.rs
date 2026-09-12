#![cfg(target_os = "linux")]
use std::{
    collections::BTreeSet,
    fs::{self, DirBuilder},
    os::unix::{
        fs::{DirBuilderExt, symlink},
        net::UnixListener,
    },
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use symbiote_domain::*;
use symbiote_runtime_transport::{DescendantCleanup, TransportLimits};
use symbiote_sandbox::*;
use symbiote_trust::*;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    owns_directory: bool,
    base: PathBuf,
    worktree: PathBuf,
    protected: Vec<PathBuf>,
    bound_bytes: u64,
}
impl Fixture {
    fn new() -> Self {
        let base = loop {
            let p = std::env::temp_dir().join(format!(
                "symbiote-sandbox-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match DirBuilder::new().mode(0o700).create(&p) {
                Ok(()) => break p,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("{e}"),
            }
        };
        let worktree = base.join("worktree");
        let host = base.join("host");
        for path in [&worktree, &host] {
            DirBuilder::new().mode(0o700).create(path).unwrap();
        }
        Self {
            owns_directory: true,
            base,
            worktree,
            protected: vec![host],
            // The address-space ceiling every launch in these tests carries
            // unless a test narrows it: roomy enough for the tools under test.
            bound_bytes: 1 << 30,
        }
    }
    fn consent(&self, profile: Profile, args: &[String]) -> ResourceConsent {
        let project = ProjectId::new("project").unwrap();
        let root = RootId::new("root").unwrap();
        ResourceConsent {
            id: CommandId::new("consent").unwrap(),
            user_id: UserId::new("user").unwrap(),
            issued_at: Timestamp(1),
            expires_at: Timestamp(100),
            revoked_at: None,
            snapshot: ResourceSnapshot {
                project_id: project.clone(),
                role_id: RoleId::new("role").unwrap(),
                profile_id: RuntimeProfileId::new("runtime").unwrap(),
                host_id: HostId::new("host").unwrap(),
                resource_ref: "sandbox-command".into(),
                fingerprint: fingerprint_command(
                    &root,
                    &self.worktree,
                    profile,
                    "/usr/bin/python3",
                    args,
                )
                .unwrap(),
                access: AccessSnapshot {
                    project_id: project,
                    roots: BTreeSet::from([root]),
                    grants: BTreeSet::from([
                        Permission::ReadRoot,
                        Permission::ExecuteProcess,
                        Permission::MutateStream,
                    ]),
                    policy_revision: Revision(1),
                },
            },
        }
    }
    fn request<'a>(
        &'a self,
        consent: &'a ResourceConsent,
        profile: Profile,
        args: &'a [String],
    ) -> LaunchRequest<'a> {
        LaunchRequest {
            helper_path: std::path::Path::new(env!("CARGO_BIN_EXE_symbiote-sandbox-launch")),
            consent,
            snapshot: &consent.snapshot,
            policy: &consent.snapshot.access,
            host: &consent.snapshot.host_id,
            at: Timestamp(10),
            root_id: consent.snapshot.access.roots.first().unwrap(),
            worktree: &self.worktree,
            protected_paths: &self.protected,
            profile,
            program: "/usr/bin/python3",
            args,
            limits: TransportLimits::default(),
            address_space_bytes: self.bound_bytes,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if self.owns_directory {
            // Cleanup must not turn an original setup failure into a double panic.
            let _ = fs::remove_dir_all(&self.base);
        }
    }
}

fn launch_ready(request: LaunchRequest<'_>) -> SandboxProcess {
    launch(request).unwrap_or_else(|error| {
        if let Some(setup) = error.setup_failure() {
            for line in setup.diagnostics() {
                eprintln!("sandbox setup stderr: {line}");
            }
            if setup.truncated() {
                eprintln!("sandbox setup stderr truncated");
            }
        }
        panic!("sandbox setup failed: {error}");
    })
}

#[test]
fn actual_hostile_process_cannot_read_host_credentials_socket_or_network() {
    let fixture = Fixture::new();
    let credential = fixture.protected[0].join("credential");
    fs::write(&credential, b"fixture-only-secret").unwrap();
    let socket = fixture.protected[0].join("control.sock");
    let _listener = UnixListener::bind(&socket).unwrap();
    let tcp = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    fs::write(fixture.worktree.join("input"), b"readable").unwrap();
    let script = r#"import json,os,socket,sys,subprocess
def denied(f):
 try: f(); return False
 except OSError: return True
def tcp():
 s=socket.socket();s.settimeout(.2);s.connect(('127.0.0.1',int(sys.argv[3])))
def unix():
 s=socket.socket(socket.AF_UNIX);s.connect(sys.argv[2])
result={'credential_denied':denied(lambda:open(sys.argv[1]).read()),'socket_denied':denied(unix),'network_denied':denied(tcp),'write_denied':denied(lambda:open('/workspace/output','w')),'usr_write_denied':denied(lambda:open('/usr/symbiote-test','w')),'input':open('/workspace/input').read(),'home':os.environ.get('HOME'),'env_keys':sorted(os.environ),'nested_userns_denied':subprocess.run(['/usr/bin/unshare','-Ur','/usr/bin/true'],stderr=subprocess.DEVNULL).returncode!=0}
print(json.dumps(result),flush=True)
"#;
    let args = vec![
        "-c".into(),
        script.into(),
        credential.to_str().unwrap().into(),
        socket.to_str().unwrap().into(),
        tcp.local_addr().unwrap().port().to_string(),
    ];
    let consent = fixture.consent(Profile::ReadOnly, &args);
    let mut child = launch_ready(fixture.request(&consent, Profile::ReadOnly, &args));
    let report = child.recv(Duration::from_secs(5)).unwrap();
    for key in [
        "credential_denied",
        "socket_denied",
        "network_denied",
        "write_denied",
        "usr_write_denied",
        "nested_userns_denied",
    ] {
        assert_eq!(report[key], true, "{key}");
    }
    assert_eq!(report["input"], "readable");
    assert_eq!(report["home"], "/home/agent");
    let keys = report["env_keys"].as_array().unwrap();
    assert!(keys.iter().all(|k| matches!(
        k.as_str(),
        Some("HOME" | "PATH" | "TMPDIR" | "PWD" | "LC_CTYPE" | "SHLVL" | "_")
    )));
    assert!(!fixture.worktree.join("output").exists());
}

/// The declared memory bound is a real kernel ceiling on the process the
/// dispatch runs, not a number recorded beside it: the same reservation that
/// fails under a narrow bound succeeds under a roomy one. The reserved pages
/// are never touched, so the proof costs address space, not memory.
#[test]
fn the_declared_memory_bound_binds_the_sandboxed_process() {
    let script = r#"import json,resource,sys
size=int(sys.argv[1])
soft,hard=resource.getrlimit(resource.RLIMIT_AS)
try:
    block=bytearray(size)
    granted=len(block)
    del block
except MemoryError:
    granted=None
print(json.dumps({'soft':soft,'granted':granted}),flush=True)
"#;
    let demand: u64 = 256 << 20;
    let request = vec!["-c".into(), script.into(), demand.to_string()];
    // A narrow declared bound: the process carries exactly what the dispatch
    // declared, and an allocation past it fails rather than being granted.
    let mut narrow = Fixture::new();
    narrow.bound_bytes = 128 << 20;
    let consent = narrow.consent(Profile::ReadOnly, &request);
    let mut child = launch_ready(narrow.request(&consent, Profile::ReadOnly, &request));
    let observed = child.recv(Duration::from_secs(5)).unwrap();
    assert_eq!(
        observed["soft"],
        serde_json::json!(128_u64 << 20),
        "the declared bound is the process's own limit: {observed}"
    );
    assert_eq!(observed["granted"], serde_json::Value::Null, "{observed}");
    // The same demand under a roomy declared bound succeeds: the refusal above
    // is the declared limit, not the machine's memory.
    let mut roomy = Fixture::new();
    roomy.bound_bytes = 512 << 20;
    let consent = roomy.consent(Profile::ReadOnly, &request);
    let mut child = launch_ready(roomy.request(&consent, Profile::ReadOnly, &request));
    let observed = child.recv(Duration::from_secs(5)).unwrap();
    assert_eq!(
        observed["soft"],
        serde_json::json!(512_u64 << 20),
        "{observed}"
    );
    assert_eq!(observed["granted"], serde_json::json!(demand), "{observed}");
}

/// A bound the launcher cannot express refuses the launch: there is no path
/// that starts a dispatch's process without the ceiling it declared.
#[test]
fn a_bound_the_launcher_cannot_express_is_refused_before_any_process_starts() {
    let mut fixture = Fixture::new();
    fixture.bound_bytes = 0;
    let args = vec!["-c".into(), "print('never runs')".into()];
    let consent = fixture.consent(Profile::ReadOnly, &args);
    assert!(matches!(
        launch(fixture.request(&consent, Profile::ReadOnly, &args)),
        Err(SandboxError::UnrepresentableLimit)
    ));
}

#[test]
fn writes_are_explicit_and_sets_id_descendant_stops_after_cancellation() {
    let fixture = Fixture::new();
    let script = r#"import json,os,time
pid=os.fork()
if pid==0:
 os.setsid()
 deadline=time.monotonic()+10
 while time.monotonic()<deadline:
  with open('/workspace/heartbeat','ab') as f:f.write(b'x')
  time.sleep(.01)
 os._exit(0)
print(json.dumps({'started':True}),flush=True)
time.sleep(10)
"#;
    let args = vec!["-c".into(), script.into()];
    let consent = fixture.consent(Profile::WorktreeWrite, &args);
    let mut child = launch_ready(fixture.request(&consent, Profile::WorktreeWrite, &args));
    assert_eq!(child.recv(Duration::from_secs(5)).unwrap()["started"], true);
    let deadline = Instant::now() + Duration::from_secs(3);
    while !fs::metadata(fixture.worktree.join("heartbeat")).is_ok_and(|m| m.len() > 0) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    let result = child.cancel(Duration::from_secs(3)).unwrap();
    assert_eq!(result.descendant_cleanup, DescendantCleanup::Unknown);
    std::thread::sleep(Duration::from_millis(50));
    let before = fs::metadata(fixture.worktree.join("heartbeat"))
        .unwrap()
        .len();
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(
        fs::metadata(fixture.worktree.join("heartbeat"))
            .unwrap()
            .len(),
        before
    );
}

#[test]
fn socket_hardlink_symlink_and_protected_overlap_refused_before_spawn() {
    let fixture = Fixture::new();
    let args = vec!["-c".into(), "print('{}')".into()];
    let consent = fixture.consent(Profile::ReadOnly, &args);
    let socket = fixture.worktree.join("socket");
    let listener = UnixListener::bind(&socket).unwrap();
    assert!(matches!(
        launch(fixture.request(&consent, Profile::ReadOnly, &args)),
        Err(SandboxError::UnsupportedEntry)
    ));
    drop(listener);
    fs::remove_file(socket).unwrap();
    let secret = fixture.protected[0].join("secret");
    fs::write(&secret, b"fixture").unwrap();
    let alias = fixture.worktree.join("alias");
    fs::hard_link(&secret, &alias).unwrap();
    assert!(matches!(
        launch(fixture.request(&consent, Profile::ReadOnly, &args)),
        Err(SandboxError::UnsupportedEntry)
    ));
    fs::remove_file(&alias).unwrap();
    symlink(&secret, &alias).unwrap();
    assert!(launch(fixture.request(&consent, Profile::ReadOnly, &args)).is_err());
    fs::remove_file(alias).unwrap();
    let protected = vec![fixture.base.clone()];
    let mut request = fixture.request(&consent, Profile::ReadOnly, &args);
    request.protected_paths = &protected;
    assert!(matches!(
        launch(request),
        Err(SandboxError::ProtectedOverlap)
    ));
    let protected = vec![PathBuf::from("/usr")];
    let mut request = fixture.request(&consent, Profile::ReadOnly, &args);
    request.protected_paths = &protected;
    assert!(matches!(
        launch(request),
        Err(SandboxError::ProtectedOverlap)
    ));
}

#[test]
fn changed_expired_revoked_or_underprivileged_consent_denies_launch() {
    let fixture = Fixture::new();
    let args = vec!["-c".into(), "print('{}')".into()];
    let consent = fixture.consent(Profile::ReadOnly, &args);
    let changed = vec!["-c".into(), "print('changed')".into()];
    assert!(matches!(
        launch(fixture.request(&consent, Profile::ReadOnly, &changed)),
        Err(SandboxError::ConsentDenied)
    ));
    assert!(matches!(
        launch(fixture.request(&consent, Profile::WorktreeWrite, &args)),
        Err(SandboxError::ConsentDenied)
    ));
    let mut expired = consent.clone();
    expired.expires_at = Timestamp(9);
    assert!(matches!(
        launch(fixture.request(&expired, Profile::ReadOnly, &args)),
        Err(SandboxError::ConsentDenied)
    ));
    let mut revoked = consent.clone();
    revoked.revoked_at = Some(Timestamp(5));
    assert!(matches!(
        launch(fixture.request(&revoked, Profile::ReadOnly, &args)),
        Err(SandboxError::ConsentDenied)
    ));
    let mut denied = consent.clone();
    denied
        .snapshot
        .access
        .grants
        .remove(&Permission::ExecuteProcess);
    assert!(matches!(
        launch(fixture.request(&denied, Profile::ReadOnly, &args)),
        Err(SandboxError::PermissionDenied)
    ));
    let mut write = fixture.consent(Profile::WorktreeWrite, &args);
    write
        .snapshot
        .access
        .grants
        .remove(&Permission::MutateStream);
    assert!(matches!(
        launch(fixture.request(&write, Profile::WorktreeWrite, &args)),
        Err(SandboxError::PermissionDenied)
    ));
}

#[test]
fn inherited_descriptor_boundary_child() {
    if std::env::var_os("SYMBIOTE_FD_TEST_CHILD").is_none() {
        return;
    }
    use nix::fcntl::{FcntlArg, FdFlag, fcntl};
    use std::{io::Write, os::fd::AsRawFd};
    let (read, write) = nix::unistd::pipe().unwrap();
    nix::unistd::write(&write, b"fixture-pipe").unwrap();
    let (socket, mut peer) = std::os::unix::net::UnixStream::pair().unwrap();
    peer.write_all(b"fixture-socket").unwrap();
    // Isolated test process only: deliberately create inheritable capabilities
    // to prove that the helper removes them, never changing the test runner's FDs.
    for fd in [&read as &dyn std::os::fd::AsFd, &socket] {
        fcntl(fd, FcntlArg::F_SETFD(FdFlag::empty())).unwrap();
    }
    let fixture = Fixture::new();
    let script = "import os,sys,json\ndef closed(fd):\n try:os.fstat(int(fd));return False\n except OSError:return True\nprint(json.dumps({'closed':all(closed(x) for x in sys.argv[1:])}),flush=True)";
    let args = vec![
        "-c".into(),
        script.into(),
        read.as_raw_fd().to_string(),
        socket.as_raw_fd().to_string(),
    ];
    let consent = fixture.consent(Profile::ReadOnly, &args);
    let mut child = launch_ready(fixture.request(&consent, Profile::ReadOnly, &args));
    assert_eq!(child.recv(Duration::from_secs(5)).unwrap()["closed"], true);
}

#[test]
fn inherited_pipe_and_host_socket_are_closed_before_untrusted_exec() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "inherited_descriptor_boundary_child",
            "--nocapture",
        ])
        .env("SYMBIOTE_FD_TEST_CHILD", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn oversized_commands_and_untrusted_ancestors_are_rejected() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let root = RootId::new("root").unwrap();
    assert!(
        fingerprint_command(
            &root,
            &fixture.worktree,
            Profile::ReadOnly,
            &format!("/usr/bin/{}", "x".repeat(257)),
            &[]
        )
        .is_err()
    );
    assert!(
        fingerprint_command(
            &root,
            &PathBuf::from(format!("/{}", "x".repeat(4096))),
            Profile::ReadOnly,
            "/usr/bin/python3",
            &[]
        )
        .is_err()
    );
    let args = vec!["-c".into(), "print('{}')".into()];
    let consent = fixture.consent(Profile::ReadOnly, &args);
    fs::set_permissions(&fixture.base, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(matches!(
        launch(fixture.request(&consent, Profile::ReadOnly, &args)),
        Err(SandboxError::UnsafePath)
    ));
}

#[test]
fn sandbox_owner_death_child() {
    let Some(base) = std::env::var_os("SYMBIOTE_OWNER_DEATH_BASE") else {
        return;
    };
    let base = PathBuf::from(base);
    let fixture = Fixture {
        owns_directory: false,
        worktree: base.join("worktree"),
        protected: vec![base.join("host")],
        base,
        bound_bytes: 1 << 30,
    };
    let args=vec!["-c".into(),"import os,time\nif os.fork()==0:\n os.setsid()\n deadline=time.monotonic()+10\n while time.monotonic()<deadline:\n  with open('/workspace/owner-heartbeat','ab') as f:f.write(b'x')\n  time.sleep(.01)\n os._exit(0)\ntime.sleep(10)".into()];
    let consent = fixture.consent(Profile::WorktreeWrite, &args);
    let _child = launch_ready(fixture.request(&consent, Profile::WorktreeWrite, &args));
    std::thread::sleep(Duration::from_secs(60));
}

#[test]
fn setsid_descendant_heartbeat_stops_after_launcher_owner_dies() {
    struct Owner(std::process::Child);
    impl Drop for Owner {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let fixture = Fixture::new();
    let mut owner = Owner(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "sandbox_owner_death_child", "--nocapture"])
            .env("SYMBIOTE_OWNER_DEATH_BASE", &fixture.base)
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    let heartbeat = fixture.worktree.join("owner-heartbeat");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !fs::metadata(&heartbeat).is_ok_and(|m| m.len() > 0) {
        assert!(
            owner.0.try_wait().unwrap().is_none(),
            "sandbox owner exited before heartbeat"
        );
        assert!(Instant::now() < deadline, "owner death fixture timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
    owner.0.kill().unwrap();
    owner.0.wait().unwrap();
    std::thread::sleep(Duration::from_millis(100));
    let before = fs::metadata(&heartbeat).unwrap().len();
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(fs::metadata(&heartbeat).unwrap().len(), before);
}

#[test]
fn setup_failure_stderr_is_bounded_explicit_and_debug_redacted() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    // Deliberate trusted fixture helper that fails before executing any workload.
    let helper = fixture.base.join("failing-helper");
    fs::write(
        &helper,
        "#!/usr/bin/sh\nprintf 'fixture-private-detail\\n' >&2\nexit 42\n",
    )
    .unwrap();
    fs::set_permissions(&helper, fs::Permissions::from_mode(0o700)).unwrap();
    let args = vec!["-c".into(), "print('{}')".into()];
    let consent = fixture.consent(Profile::ReadOnly, &args);
    let mut request = fixture.request(&consent, Profile::ReadOnly, &args);
    request.helper_path = &helper;
    let error = match launch(request) {
        Err(error) => error,
        Ok(_) => panic!("failing helper was accepted"),
    };
    let setup = error.setup_failure().expect("structured setup failure");
    assert!(
        setup
            .diagnostics()
            .iter()
            .any(|line| line.contains("fixture-private-detail"))
    );
    assert!(!format!("{error:?}").contains("fixture-private-detail"));
    assert!(!format!("{error}").contains("fixture-private-detail"));
    assert!(setup.diagnostics().len() <= 8);
    assert!(setup.diagnostics().iter().all(|line| line.len() <= 1024));
    assert!(!setup.truncated());
    assert!(fixture.worktree.exists());
    fs::write(
        &helper,
        "#!/usr/bin/sh\nprintf '%02000d\\n' 0 >&2\nexit 42\n",
    )
    .unwrap();
    let mut request = fixture.request(&consent, Profile::ReadOnly, &args);
    request.helper_path = &helper;
    let error = match launch(request) {
        Err(error) => error,
        Ok(_) => panic!("failing helper was accepted"),
    };
    let setup = error.setup_failure().unwrap();
    assert!(setup.truncated());
    assert_eq!(setup.diagnostics()[0].len(), 1024);
}

/// The mount boundary as an errno table, for both compositions the sandbox
/// supports: `WorktreeWrite`, which is what a dispatch carrying the mutation
/// grant runs the external lane's harness under, and `ReadOnly`, which is what
/// a contract without that grant runs under. `/workspace` is the dispatch's
/// reserved worktree, the one Host surface a run may change. Everything else
/// the mount table leaves reachable inside is either absent (the sandbox root
/// is a fresh tmpfs, so no Host directory, `/etc` or `/var` exists at all —
/// errno 2) or read-only (`/usr` is a read-only bind, the root is remounted
/// read-only, and the device tree is the sandbox's own tmpfs remounted
/// read-only — errno 30). Read-only `/dev` closes the alias threat class the
/// worktree preflight exists for: a file, directory, symlink, Unix socket and
/// device node are each attempted inside it and must be refused with the errno
/// the kernel returned. `/tmp` and `/home/agent` are the sandbox's own writable
/// tmpfs mounts, not the Host's: the test asserts a run may write there and
/// that the write never reaches the Host path of the same name. `/dev/shm` is
/// the named exception to the read-only device tree — its own writable tmpfs,
/// proven by a forked second process observing what this one wrote — and
/// `/dev/pts` is the other, a separate writable devpts mount where
/// pseudo-terminal allocation must still succeed. Under `ReadOnly` the same
/// table holds minus the worktree, which joins the refusing side, so a dispatch
/// with no mutation grant gets no Host write at all.
#[test]
fn the_mount_boundary_confines_host_writes_to_the_reserved_worktree() {
    let script = r#"import json,os,socket,stat,sys
host_worktree,host_protected,shadow=sys.argv[1],sys.argv[2],sys.argv[3]
tmp_before=len(os.listdir('/tmp'))
def mutation(fn):
    try:
        fn()
        return 'allowed'
    except OSError as error:
        return error.errno
def write_file(path):
    def run():
        with open(path,'w') as handle:
            handle.write('probe')
    return run
def shm_second_process_observation():
    path='/dev/shm/'+shadow
    try:
        with open(path,'w') as handle:
            handle.write('shared-memory')
    except OSError as error:
        return 'errno %d'%error.errno
    read,write=os.pipe()
    pid=os.fork()
    if pid==0:
        os.close(read)
        try:
            with open(path) as handle:
                seen=handle.read()
        except OSError as error:
            seen='errno %d'%error.errno
        os.write(write,seen.encode())
        os._exit(0)
    os.close(write)
    seen=b''
    while True:
        part=os.read(read,64)
        if not part:
            break
        seen+=part
    os.close(read)
    os.waitpid(pid,0)
    return seen.decode()
probes={
 'reserved_worktree':write_file('/workspace/probe.txt'),
 'worktree_traversal':write_file('/workspace/../probe.txt'),
 'sandbox_root':write_file('/symbiote-probe'),
 'sandbox_usr':write_file('/usr/symbiote-probe'),
 'sandbox_etc':write_file('/etc/symbiote-probe'),
 'sandbox_var':write_file('/var/symbiote-probe'),
 'host_worktree':write_file(host_worktree+'/probe.txt'),
 'host_protected':write_file(host_protected+'/probe.txt'),
 'sandbox_tmp':write_file('/tmp/'+shadow),
 'sandbox_home':write_file('/home/agent/'+shadow),
 'sandbox_dev':write_file('/dev/'+shadow),
 'dev_mkdir':lambda:os.mkdir('/dev/'+shadow),
 'dev_symlink':lambda:os.symlink('/workspace','/dev/'+shadow),
 'dev_socket':lambda:socket.socket(socket.AF_UNIX).bind('/dev/'+shadow),
 'dev_node':lambda:os.mknod('/dev/'+shadow,0o600|stat.S_IFCHR),
}
def device_nodes():
    try:
        with open('/dev/null','w') as handle:
            handle.write('probe')
        with open('/dev/urandom','rb') as handle:
            handle.read(4)
        return 'usable'
    except OSError as error:
        return error.errno
def pty_allocation():
    try:
        import pty
        master,slave=pty.openpty()
        os.close(master)
        os.close(slave)
        return 'allocated'
    except OSError as error:
        return error.errno if error.errno is not None else 'refused'
json.dump({'cwd':os.getcwd(),'tmp_before':tmp_before,'shm_observed':shm_second_process_observation(),'device_nodes':device_nodes(),'pty':pty_allocation(),'probes':{name:mutation(fn) for name,fn in probes.items()}},sys.stdout)
sys.stdout.write('\n')
sys.stdout.flush()
"#;
    for (profile, reserved_worktree) in [
        (Profile::WorktreeWrite, serde_json::json!("allowed")),
        (Profile::ReadOnly, serde_json::json!(30)),
    ] {
        let fixture = Fixture::new();
        // A read-only run models a contract that carries NO mutation grant: the
        // grant is absent from the consent itself, not merely left unused.
        let shadow = format!(
            "{}.probe",
            fixture.base.file_name().unwrap().to_string_lossy()
        );
        let args = vec![
            "-c".to_owned(),
            script.to_owned(),
            fixture.worktree.display().to_string(),
            fixture.protected[0].display().to_string(),
            shadow.clone(),
        ];
        let mut consent = fixture.consent(profile, &args);
        if profile == Profile::ReadOnly {
            consent
                .snapshot
                .access
                .grants
                .remove(&Permission::MutateStream);
        }
        let mut child = launch_ready(fixture.request(&consent, profile, &args));
        let observed = child.recv(Duration::from_secs(10)).unwrap();
        assert_eq!(observed["cwd"], "/workspace", "{profile:?}");
        assert_eq!(
            observed["tmp_before"],
            serde_json::json!(0),
            "the sandbox's /tmp is its own and empty at start: {observed}"
        );
        let probes = &observed["probes"];
        assert_eq!(
            probes["reserved_worktree"], reserved_worktree,
            "{profile:?}: the reserved worktree is writable exactly under the mutation grant: {observed}"
        );
        // Read-only surfaces: the read-only `/usr` bind, the fresh root and the
        // device tree (`/dev` is remounted read-only after `--dev`).
        for target in [
            "worktree_traversal",
            "sandbox_root",
            "sandbox_usr",
            "sandbox_dev",
        ] {
            assert_eq!(
                probes[target],
                serde_json::json!(30),
                "{profile:?} {target} must be read-only: {observed}"
            );
        }
        // The alias threat class read-only `/dev` exists to close: no
        // directory, symlink, Unix socket or device node may be created in the
        // device tree, each refused with the errno the kernel returned.
        for target in ["dev_mkdir", "dev_symlink", "dev_socket", "dev_node"] {
            assert_eq!(
                probes[target],
                serde_json::json!(30),
                "{profile:?} {target} must be refused in the device tree: {observed}"
            );
        }
        // Absent surfaces: nothing of the Host tree, `/etc` or `/var` exists
        // inside the sandbox root at all.
        for target in [
            "sandbox_etc",
            "sandbox_var",
            "host_worktree",
            "host_protected",
        ] {
            assert_eq!(
                probes[target],
                serde_json::json!(2),
                "{profile:?} {target} must be invisible inside the sandbox: {observed}"
            );
        }
        // The sandbox's own writable tmpfs mounts, not the Host's.
        for target in ["sandbox_tmp", "sandbox_home"] {
            assert_eq!(
                probes[target], "allowed",
                "{profile:?} {target} is the sandbox's own mount: {observed}"
            );
        }
        // A read-only `/dev` does not take the device nodes with it: they are
        // separate mounts.
        assert_eq!(
            observed["device_nodes"], "usable",
            "{profile:?}: device nodes under a read-only /dev: {observed}"
        );
        // `/dev/pts` is the named exception: a separate writable devpts mount
        // where pseudo-terminal allocation must still succeed.
        assert_eq!(
            observed["pty"], "allocated",
            "{profile:?}: /dev/pts pty allocation under a read-only /dev: {observed}"
        );
        // Shared memory is a real cross-process surface: a forked second
        // process reads back what this one wrote through `/dev/shm`.
        assert_eq!(
            observed["shm_observed"], "shared-memory",
            "{profile:?}: a second process must observe the /dev/shm write: {observed}"
        );
        assert!(
            !std::path::Path::new("/tmp").join(&shadow).exists()
                && !std::path::Path::new("/home/agent").join(&shadow).exists()
                && !std::path::Path::new("/dev").join(&shadow).exists()
                && !std::path::Path::new("/dev/shm").join(&shadow).exists(),
            "{profile:?}: a write inside the sandbox's own /tmp, /home, /dev or /dev/shm reached the Host"
        );
        // Under the writable profile the file the probe wrote through
        // `/workspace` is the Host's own file in the reserved worktree; under
        // the read-only profile the worktree has no new file at all.
        let produced = fixture.worktree.join("probe.txt");
        match profile {
            Profile::WorktreeWrite => assert_eq!(
                fs::read_to_string(&produced).unwrap(),
                "probe",
                "the writable worktree write lands on the Host"
            ),
            Profile::ReadOnly => assert!(
                !produced.exists(),
                "a contract with no mutation grant wrote into its worktree"
            ),
        }
    }
}
