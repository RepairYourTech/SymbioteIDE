//! Linux-only destructive-lifetime probe. It only launches this executable's fixed
//! workload, never caller-specified commands. This is not the production Host.
use nix::fcntl::{Flock, FlockArg};
use nix::sys::signal::{Signal, killpg};
use nix::unistd::{Pid, geteuid};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Group {
    pid: u32,
    start_ticks: u64,
    token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    sequence: u64,
    kind: String,
    detail: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u32,
    active: Option<Group>,
    events: Vec<Event>,
}

impl Journal {
    fn record(&mut self, kind: &str, detail: impl Into<String>) {
        self.events.push(Event {
            sequence: self.events.len() as u64 + 1,
            kind: kind.into(),
            detail: detail.into(),
        });
    }

    fn read(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self {
                version: 1,
                ..Self::default()
            });
        }
        if !fs::symlink_metadata(path)?.is_file() {
            return Err("journal must be a regular file".into());
        }
        let journal: Self = serde_json::from_reader(File::open(path)?)?;
        if journal.version != 1
            || journal
                .events
                .iter()
                .enumerate()
                .any(|(i, e)| e.sequence != i as u64 + 1)
        {
            return Err("unsupported journal or noncontiguous event cursor".into());
        }
        Ok(journal)
    }

    fn save(&self, path: &Path) -> Result<()> {
        let temporary = path.with_extension(format!("next-{}", std::process::id()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
        serde_json::to_writer(&mut file, self)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        File::open(path.parent().ok_or("journal has no directory")?)?.sync_all()?;
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Start { fault: Option<Fault> },
    Status {},
    Events { after: u64 },
    Cancel {},
    Shutdown {},
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Fault {
    IgnoreGuardianEof,
}

#[derive(Serialize)]
struct Response {
    ok: bool,
    status: String,
    cursor: u64,
    group: Option<Group>,
    events: Vec<Event>,
    error: Option<String>,
}

struct Running {
    child: Child,
    // Keeping this open authorizes lifetime independently of every client socket.
    lease: ChildStdin,
}

struct Host {
    journal: Journal,
    journal_path: PathBuf,
    running: Option<Running>,
    recovery_blocked: bool,
}

struct ProcessIdentity {
    state: char,
    group: u32,
    start_ticks: u64,
}

fn identity(pid: u32) -> io::Result<ProcessIdentity> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    let (_, tail) = stat
        .rsplit_once(')')
        .ok_or_else(|| io::Error::other("bad proc stat"))?;
    let fields: Vec<_> = tail.split_whitespace().collect();
    let parse = |index: usize| -> io::Result<u64> {
        fields
            .get(index)
            .ok_or_else(|| io::Error::other("short proc stat"))?
            .parse()
            .map_err(io::Error::other)
    };
    Ok(ProcessIdentity {
        state: fields
            .first()
            .and_then(|s| s.chars().next())
            .ok_or_else(|| io::Error::other("missing state"))?,
        group: u32::try_from(parse(2)?).map_err(io::Error::other)?,
        start_ticks: parse(19)?,
    })
}

fn has_token(pid: u32, token: &str) -> bool {
    fs::read(format!("/proc/{pid}/cmdline")).is_ok_and(|bytes| {
        let args: Vec<_> = bytes.split(|b| *b == 0).collect();
        args.get(1) == Some(&b"workload".as_slice()) && args.get(3) == Some(&token.as_bytes())
    })
}

fn live_members(group: &Group) -> Result<Vec<u32>> {
    let mut members = Vec::new();
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if let Ok(process) = identity(pid) {
            if process.group == group.pid
                && process.state != 'Z'
                && process.state != 'X'
                && has_token(pid, &group.token)
            {
                members.push(pid);
            }
        }
    }
    Ok(members)
}

fn wait_empty(group: &Group) -> Result<bool> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if live_members(group)?.is_empty() {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

impl Host {
    fn open(journal_path: PathBuf) -> Result<Self> {
        let mut journal = Journal::read(&journal_path)?;
        let mut recovery_blocked = false;
        if let Some(group) = journal.active.clone() {
            // Never signal journal PIDs: the old process could have exited and its
            // PID been reused. The workload's pipe guardian handles daemon death.
            if wait_empty(&group)? {
                journal.active = None;
                journal.record(
                    "recovery_confirmed_exit",
                    "no live owned workload members; no PID was signalled",
                );
            } else {
                recovery_blocked = true;
                journal.record(
                    "recovery_required",
                    "owned workload survived guardian deadline; no blind kill attempted",
                );
            }
        }
        journal.record("host_started", "private local protocol ready");
        journal.save(&journal_path)?;
        Ok(Self {
            journal,
            journal_path,
            running: None,
            recovery_blocked,
        })
    }

    fn response(&self, error: Option<String>, events: Vec<Event>) -> Response {
        Response {
            ok: error.is_none(),
            status: if self.recovery_blocked {
                "recovery_required"
            } else if self.running.is_some() {
                "running"
            } else {
                "idle"
            }
            .into(),
            cursor: self.journal.events.len() as u64,
            group: self.journal.active.clone(),
            events,
            error,
        }
    }

    fn start(&mut self, fault: Option<Fault>) -> Result<()> {
        if self.running.is_some() || self.journal.active.is_some() || self.recovery_blocked {
            return Err("workload already active or recovery required".into());
        }
        let token = format!("probe-{}-{}", std::process::id(), self.journal.events.len());
        let role = if fault.is_some() {
            "root-no-guardian"
        } else {
            "root"
        };
        let mut child = Command::new(std::env::current_exe()?)
            .args(["workload", role, &token])
            .arg(
                self.journal_path
                    .parent()
                    .ok_or("journal directory missing")?
                    .join(format!("stop-{token}")),
            )
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let group = Group {
            pid: child.id(),
            start_ticks: identity(child.id())?.start_ticks,
            token,
        };
        let mut lease = child.stdin.take().ok_or("workload lease missing")?;
        self.journal.active = Some(group.clone());
        self.journal.record(
            "workload_started",
            format!("owned process group {}", group.pid),
        );
        // Persist the process identity BEFORE authorizing descendants. EOF at any
        // earlier crash makes the root exit without launching them.
        self.journal.save(&self.journal_path)?;
        lease.write_all(b"GO\n")?;
        lease.flush()?;
        self.running = Some(Running { child, lease });
        let deadline = Instant::now() + Duration::from_secs(3);
        while live_members(&group)?.len() != 3 {
            if Instant::now() >= deadline {
                return Err("fixed workload did not produce three owned processes".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        self.journal
            .record("workload_ready", "root, child and grandchild observed");
        self.journal.save(&self.journal_path)?;
        Ok(())
    }

    fn cancel(&mut self) -> Result<()> {
        if self.recovery_blocked {
            return Err("recovery required; no journal PID may be signalled".into());
        }
        let Some(mut running) = self.running.take() else {
            return Err("no active workload".into());
        };
        let group = self
            .journal
            .active
            .clone()
            .ok_or("active identity missing")?;
        let root_matches = identity(group.pid).is_ok_and(|p| {
            p.start_ticks == group.start_ticks && p.group == group.pid && p.state != 'Z'
        }) && has_token(group.pid, &group.token)
            && running.child.try_wait()?.is_none();
        if root_matches {
            // The owned unreaped Child pins PID reuse during this operation. It
            // was created as process-group leader; only that verified group dies.
            killpg(Pid::from_raw(i32::try_from(group.pid)?), Signal::SIGTERM)?;
        }
        drop(running.lease);
        let exited = wait_empty(&group)?;
        if exited {
            running.child.wait()?;
            self.journal.active = None;
            self.journal.record(
                "workload_cancelled",
                "owned process group has no live members",
            );
        } else {
            self.recovery_blocked = true;
            self.journal
                .record("recovery_required", "cancellation did not prove group exit");
        }
        self.journal.save(&self.journal_path)?;
        if exited {
            Ok(())
        } else {
            Err("owned processes survived; recovery required".into())
        }
    }

    fn handle(&mut self, request: Request) -> (Response, bool) {
        let mut shutdown = false;
        let result = match request {
            Request::Start { fault } => self.start(fault),
            Request::Cancel {} => self.cancel(),
            Request::Status {} => Ok(()),
            Request::Events { after } => {
                if after > self.journal.events.len() as u64 {
                    return (
                        self.response(Some("cursor exceeds journal".into()), vec![]),
                        false,
                    );
                }
                let events = self
                    .journal
                    .events
                    .iter()
                    .filter(|e| e.sequence > after)
                    .cloned()
                    .collect();
                return (self.response(None, events), false);
            }
            Request::Shutdown {} => {
                let result = if self.running.is_some() {
                    self.cancel()
                } else {
                    Ok(())
                };
                if result.is_ok() {
                    self.journal.record("host_stopped", "explicit shutdown");
                    shutdown = true;
                    self.journal.save(&self.journal_path)
                } else {
                    result
                }
            }
        };
        (
            self.response(result.err().map(|error| error.to_string()), vec![]),
            shutdown,
        )
    }
}

fn workload(role: &str, token: &str, stop_marker: Option<&str>) -> Result<()> {
    match role {
        "root" | "root-no-guardian" => {
            let mut stdin = io::stdin().lock();
            let mut line = String::new();
            if stdin.read_line(&mut line)? == 0 || line != "GO\n" {
                return Ok(());
            }
            let _child = Command::new(std::env::current_exe()?)
                .args(["workload", "child", token])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            // This FD is held only by the daemon, not by any GUI/client. If the
            // daemon crashes, EOF ends this entire group, including grandchildren.
            line.clear();
            let _ = stdin.read_line(&mut line);
            if role == "root-no-guardian" {
                let marker =
                    stop_marker.ok_or("fault workload requires test-owned stop control")?;
                while fs::read(marker).ok().as_deref() != Some(b"stop") {
                    thread::sleep(Duration::from_millis(10));
                }
            }
            // The fault test requests cleanup through its private file control.
            // The still-live group leader signals itself: no restored or scanned
            // PID is used, so process identity cannot be recycled in between.
            killpg(
                Pid::from_raw(i32::try_from(std::process::id())?),
                Signal::SIGTERM,
            )?;
        }
        "child" => {
            let _grandchild = Command::new(std::env::current_exe()?)
                .args(["workload", "grandchild", token])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            loop {
                thread::sleep(Duration::from_secs(60));
            }
        }
        "grandchild" => loop {
            thread::sleep(Duration::from_secs(60));
        },
        _ => return Err("unknown fixed workload role".into()),
    }
    Ok(())
}

fn serve(directory: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(directory)?;
    if !metadata.is_dir() || metadata.uid() != geteuid().as_raw() || metadata.mode() & 0o077 != 0 {
        return Err("probe directory must be owned by current user with mode 0700".into());
    }
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory.join("host.lock"))?;
    let _lock = Flock::lock(lock, FlockArg::LockExclusiveNonblock).map_err(|(_, e)| e)?;
    let socket = directory.join("host.sock");
    if let Ok(metadata) = fs::symlink_metadata(&socket) {
        if !metadata.file_type().is_socket() || metadata.uid() != geteuid().as_raw() {
            return Err("refusing to replace non-owned socket path".into());
        }
        fs::remove_file(&socket)?;
    }
    let listener = UnixListener::bind(&socket)?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))?;
    let mut host = Host::open(directory.join("journal.json"))?;
    for stream in listener.incoming() {
        let mut stream = stream?;
        stream.set_read_timeout(Some(Duration::from_millis(500)))?;
        stream.set_write_timeout(Some(Duration::from_millis(500)))?;
        let mut line = String::new();
        // The fixed protocol accepts one short request per connection. Client EOF
        // has no relationship to the workload's separate daemon-held lifetime FD.
        let read = BufReader::new(&stream).take(4097).read_line(&mut line);
        let (response, shutdown) = match read {
            Ok(0) => continue,
            Ok(_) if line.len() <= 4096 && line.ends_with('\n') => {
                match serde_json::from_str::<Request>(&line) {
                    Ok(request) => host.handle(request),
                    Err(error) => (host.response(Some(error.to_string()), vec![]), false),
                }
            }
            _ => (
                host.response(
                    Some("request exceeds 4096 bytes, timed out or is incomplete".into()),
                    vec![],
                ),
                false,
            ),
        };
        let _ = serde_json::to_writer(&mut stream, &response);
        let _ = stream.write_all(b"\n");
        if shutdown {
            break;
        }
    }
    fs::remove_file(socket)?;
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("serve") if args.len() == 3 => serve(Path::new(&args[2])),
        Some("workload") if matches!(args.len(), 4 | 5) => {
            workload(&args[2], &args[3], args.get(4).map(String::as_str))
        }
        _ => Err("usage: symbiote-host-lifetime-probe serve PRIVATE_DIRECTORY".into()),
    }
}
