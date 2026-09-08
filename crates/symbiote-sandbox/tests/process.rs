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
    base: PathBuf,
    worktree: PathBuf,
    protected: Vec<PathBuf>,
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
            base,
            worktree,
            protected: vec![host],
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
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
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
    let mut child = launch(fixture.request(&consent, Profile::ReadOnly, &args)).unwrap();
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
    let mut child = launch(fixture.request(&consent, Profile::WorktreeWrite, &args)).unwrap();
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
    let mut child = launch(fixture.request(&consent, Profile::ReadOnly, &args)).unwrap();
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
        worktree: base.join("worktree"),
        protected: vec![base.join("host")],
        base,
    };
    let args=vec!["-c".into(),"import os,time\nif os.fork()==0:\n os.setsid()\n deadline=time.monotonic()+10\n while time.monotonic()<deadline:\n  with open('/workspace/owner-heartbeat','ab') as f:f.write(b'x')\n  time.sleep(.01)\n os._exit(0)\ntime.sleep(10)".into()];
    let consent = fixture.consent(Profile::WorktreeWrite, &args);
    let _child = launch(fixture.request(&consent, Profile::WorktreeWrite, &args)).unwrap();
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
