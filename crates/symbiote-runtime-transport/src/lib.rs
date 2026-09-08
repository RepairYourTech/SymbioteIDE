//! Bounded Unix JSONL pipes. Environment isolation is not a security sandbox.
use nix::fcntl::{FcntlArg, OFlag, fcntl};
use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};
use std::ffi::OsString;
use std::fmt;
use std::io::{self, Read, Write};
use std::os::fd::AsFd;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, SyncSender, TryRecvError},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub mod rpc;

pub struct SpawnSpec {
    pub executable: PathBuf,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
    pub env: BTreeMap<OsString, OsString>,
}

#[derive(Clone, Copy, Debug)]
pub struct TransportLimits {
    pub max_frame_bytes: usize,
    pub frame_queue_capacity: usize,
    pub diagnostic_queue_capacity: usize,
    pub max_diagnostic_bytes: usize,
}
impl Default for TransportLimits {
    fn default() -> Self {
        Self {
            max_frame_bytes: 65_536,
            frame_queue_capacity: 32,
            diagnostic_queue_capacity: 32,
            max_diagnostic_bytes: 4096,
        }
    }
}
impl TransportLimits {
    fn validate(self) -> Result<Self> {
        if !(1..=1_048_576).contains(&self.max_frame_bytes)
            || !(1..=256).contains(&self.frame_queue_capacity)
            || !(1..=256).contains(&self.diagnostic_queue_capacity)
            || !(1..=65_536).contains(&self.max_diagnostic_bytes)
        {
            return Err(TransportError::InvalidLimits);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessExit {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GroupSignalStatus {
    Sent,
    Absent,
    LeaderAlreadyReaped,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DescendantCleanup {
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CancelReport {
    pub exit: ProcessExit,
    pub group_signal: GroupSignalStatus,
    pub descendant_cleanup: DescendantCleanup,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportError {
    InvalidLimits,
    InvalidExecutable,
    InvalidWorkingDirectory,
    Io {
        operation: &'static str,
        kind: io::ErrorKind,
    },
    MalformedFrame,
    FrameTooLarge,
    FrameQueueOverflow,
    UnterminatedFrame,
    PartialFrameWrite,
    DeadlineExceeded,
    ProcessExited(ProcessExit),
    StdoutClosed,
    Closed,
}
impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TransportError {}
pub type Result<T> = std::result::Result<T, TransportError>;
fn io_error(operation: &'static str, error: io::Error) -> TransportError {
    TransportError::Io {
        operation,
        kind: error.kind(),
    }
}

#[derive(Clone)]
pub struct Diagnostic {
    pub text: String,
    pub truncated: bool,
}
impl fmt::Debug for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Diagnostic")
            .field("bytes", &self.text.len())
            .field("truncated", &self.truncated)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, Default)]
pub struct DiagnosticBatch {
    pub entries: Vec<Diagnostic>,
    pub dropped: u64,
}
#[derive(Default)]
struct DiagnosticBuffer {
    entries: VecDeque<Diagnostic>,
    dropped: u64,
}
type Fault = Arc<Mutex<Option<TransportError>>>;
fn fail(fault: &Fault, error: TransportError) {
    let mut current = fault.lock().unwrap_or_else(|poison| poison.into_inner());
    if current.is_none() {
        *current = Some(error);
    }
}

pub struct JsonlTransport {
    child: Child,
    stdin: Option<ChildStdin>,
    frames: Receiver<Value>,
    fault: Fault,
    diagnostics: Arc<Mutex<DiagnosticBuffer>>,
    stop: Arc<AtomicBool>,
    stdout_eof: Arc<AtomicBool>,
    stderr_finished: Arc<AtomicBool>,
    readers: Vec<JoinHandle<()>>,
    limits: TransportLimits,
    exit: Option<ProcessExit>,
    closed: bool,
    group_signal: Option<GroupSignalStatus>,
}

impl JsonlTransport {
    pub fn spawn(spec: SpawnSpec, limits: TransportLimits) -> Result<Self> {
        let limits = limits.validate()?;
        if !spec.executable.is_absolute() || !spec.executable.is_file() {
            return Err(TransportError::InvalidExecutable);
        }
        if !spec.cwd.is_absolute() || !spec.cwd.is_dir() {
            return Err(TransportError::InvalidWorkingDirectory);
        }
        let mut child = Command::new(spec.executable)
            .args(spec.args)
            .current_dir(spec.cwd)
            .env_clear()
            .envs(spec.env)
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| io_error("spawn", e))?;
        let pipes = (child.stdin.take(), child.stdout.take(), child.stderr.take());
        let (Some(stdin), Some(stdout), Some(stderr)) = pipes else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(TransportError::Closed);
        };
        if nonblocking(&stdin)
            .and_then(|_| nonblocking(&stdout))
            .and_then(|_| nonblocking(&stderr))
            .is_err()
        {
            let _ = killpg(Pid::from_raw(child.id() as i32), Signal::SIGKILL);
            let _ = child.kill();
            let _ = child.wait();
            return Err(TransportError::Io {
                operation: "configure nonblocking pipes",
                kind: io::ErrorKind::Other,
            });
        }
        let (sender, frames) = mpsc::sync_channel(limits.frame_queue_capacity);
        let fault = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let stdout_eof = Arc::new(AtomicBool::new(false));
        let diagnostics = Arc::new(Mutex::new(DiagnosticBuffer::default()));
        let stderr_finished = Arc::new(AtomicBool::new(false));
        let err_finished = stderr_finished.clone();
        let out_stop = stop.clone();
        let out_fault = fault.clone();
        let out_eof = stdout_eof.clone();
        let err_stop = stop.clone();
        let err_fault = fault.clone();
        let err_buffer = diagnostics.clone();
        let mut transport = Self {
            child,
            stdin: Some(stdin),
            frames,
            fault,
            diagnostics,
            stop,
            stdout_eof,
            stderr_finished,
            readers: vec![],
            limits,
            exit: None,
            closed: false,
            group_signal: None,
        };
        let out = thread::Builder::new()
            .name("jsonl-stdout".into())
            .spawn(move || {
                read_stdout(
                    stdout,
                    sender,
                    out_stop,
                    out_fault,
                    out_eof,
                    limits.max_frame_bytes,
                )
            })
            .map_err(|e| io_error("spawn stdout reader", e))?;
        transport.readers.push(out);
        let err = thread::Builder::new()
            .name("jsonl-stderr".into())
            .spawn(move || {
                read_stderr(stderr, err_stop, err_fault, err_buffer, limits);
                err_finished.store(true, Ordering::Release);
            })
            .map_err(|e| io_error("spawn stderr reader", e))?;
        transport.readers.push(err);
        Ok(transport)
    }
    pub fn child_id(&self) -> u32 {
        self.child.id()
    }
    pub fn failure(&self) -> Option<TransportError> {
        self.fault
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
    pub fn diagnostics_finished(&self) -> bool {
        self.stderr_finished.load(Ordering::Acquire)
    }
    pub fn send(&mut self, value: &Value, timeout: Duration) -> Result<()> {
        self.check_open()?;
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or(TransportError::DeadlineExceeded)?;
        let mut writer = BoundedFrame {
            bytes: Vec::new(),
            limit: self.limits.max_frame_bytes,
            deadline,
            error: None,
        };
        if serde_json::to_writer(&mut writer, value).is_err() {
            return Err(writer.error.unwrap_or(TransportError::MalformedFrame));
        }
        let mut frame = writer.bytes;
        frame.push(b'\n');
        let mut offset = 0;
        while offset < frame.len() {
            self.check_open()?;
            if Instant::now() >= deadline {
                if offset > 0 {
                    fail(&self.fault, TransportError::PartialFrameWrite);
                    return Err(TransportError::PartialFrameWrite);
                }
                return Err(TransportError::DeadlineExceeded);
            }
            match self
                .stdin
                .as_mut()
                .ok_or(TransportError::Closed)?
                .write(&frame[offset..])
            {
                Ok(0) => {
                    fail(&self.fault, TransportError::Closed);
                    return Err(TransportError::Closed);
                }
                Ok(written) => offset += written,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2))
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => {
                    let error = io_error("write stdin", error);
                    fail(&self.fault, error.clone());
                    return Err(error);
                }
            }
        }
        Ok(())
    }
    pub fn recv(&mut self, timeout: Duration) -> Result<Value> {
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or(TransportError::DeadlineExceeded)?;
        loop {
            // Observe EOF before inspecting the fault/queue it publishes. If
            // EOF arrives later, the next iteration must inspect them again.
            let stdout_eof = self.stdout_eof.load(Ordering::Acquire);
            self.check_open()?;
            match self.frames.try_recv() {
                Ok(frame) => return Ok(frame),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
            }
            if stdout_eof {
                return match self.try_wait()? {
                    Some(exit) => Err(TransportError::ProcessExited(exit)),
                    None => Err(TransportError::StdoutClosed),
                };
            }
            if Instant::now() >= deadline {
                return Err(TransportError::DeadlineExceeded);
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
    /// Reaps the directly owned child. After reaping, cancellation cannot safely
    /// signal its historical group ID; descendant cleanup stays explicitly unknown.
    pub fn try_wait(&mut self) -> Result<Option<ProcessExit>> {
        if self.exit.is_none() {
            if let Some(status) = self
                .child
                .try_wait()
                .map_err(|e| io_error("poll child", e))?
            {
                self.exit = Some(ProcessExit {
                    code: status.code(),
                    signal: status.signal(),
                });
            }
        }
        Ok(self.exit.clone())
    }
    pub fn diagnostics(&mut self) -> DiagnosticBatch {
        let mut buffer = self
            .diagnostics
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        DiagnosticBatch {
            entries: buffer.entries.drain(..).collect(),
            dropped: std::mem::take(&mut buffer.dropped),
        }
    }
    /// Hard cancellation, not a promise that escaped descendants were terminated.
    /// The unreaped owned Child pins PID reuse while its initial group is signalled.
    pub fn cancel(&mut self, timeout: Duration) -> Result<CancelReport> {
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or(TransportError::DeadlineExceeded)?;
        self.closed = true;
        self.stop.store(true, Ordering::Release);
        self.stdin.take();
        if self.exit.is_none() {
            let pid =
                Pid::from_raw(i32::try_from(self.child.id()).map_err(|_| TransportError::Closed)?);
            let outcome = match killpg(pid, Signal::SIGKILL) {
                Ok(()) => GroupSignalStatus::Sent,
                Err(nix::errno::Errno::ESRCH) => GroupSignalStatus::Absent,
                Err(_) => GroupSignalStatus::Failed,
            };
            self.group_signal.get_or_insert(outcome);
            // Also target the directly owned child if it moved out of its group.
            let _ = self.child.kill();
        } else {
            self.group_signal
                .get_or_insert(GroupSignalStatus::LeaderAlreadyReaped);
        }
        while let Some(reader) = self.readers.last() {
            if reader.is_finished() {
                if let Some(reader) = self.readers.pop() {
                    let _ = reader.join();
                }
            } else if Instant::now() >= deadline {
                return Err(TransportError::DeadlineExceeded);
            } else {
                thread::sleep(Duration::from_millis(2));
            }
        }
        loop {
            if let Some(exit) = self.try_wait()? {
                return Ok(CancelReport {
                    exit,
                    group_signal: self
                        .group_signal
                        .clone()
                        .unwrap_or(GroupSignalStatus::LeaderAlreadyReaped),
                    descendant_cleanup: DescendantCleanup::Unknown,
                });
            }
            if Instant::now() >= deadline {
                return Err(TransportError::DeadlineExceeded);
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
    fn check_open(&self) -> Result<()> {
        if self.closed {
            return Err(TransportError::Closed);
        }
        if let Some(error) = self
            .fault
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
        {
            return Err(error);
        }
        Ok(())
    }
}
impl Drop for JsonlTransport {
    fn drop(&mut self) {
        let _ = self.cancel(Duration::from_millis(500));
    }
}

struct BoundedFrame {
    bytes: Vec<u8>,
    limit: usize,
    deadline: Instant,
    error: Option<TransportError>,
}
impl Write for BoundedFrame {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if Instant::now() >= self.deadline {
            self.error = Some(TransportError::DeadlineExceeded);
            return Err(io::Error::other("serialization deadline"));
        }
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            self.error = Some(TransportError::FrameTooLarge);
            return Err(io::Error::other("frame limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn nonblocking(fd: &impl AsFd) -> nix::Result<()> {
    let flags = OFlag::from_bits_truncate(fcntl(fd, FcntlArg::F_GETFL)?);
    fcntl(fd, FcntlArg::F_SETFL(flags | OFlag::O_NONBLOCK))?;
    Ok(())
}
fn read_stdout(
    mut pipe: impl Read,
    sender: SyncSender<Value>,
    stop: Arc<AtomicBool>,
    fault: Fault,
    eof: Arc<AtomicBool>,
    limit: usize,
) {
    let mut frame = Vec::with_capacity(limit.min(4096));
    let mut bytes = [0; 4096];
    while !stop.load(Ordering::Acquire) {
        match pipe.read(&mut bytes) {
            Ok(0) => {
                if !frame.is_empty() {
                    fail(&fault, TransportError::UnterminatedFrame);
                }
                eof.store(true, Ordering::Release);
                return;
            }
            Ok(count) => {
                for byte in &bytes[..count] {
                    if *byte == b'\n' {
                        if frame.last() == Some(&b'\r') {
                            frame.pop();
                        }
                        let value = match serde_json::from_slice::<StrictValue>(&frame) {
                            Ok(value) => value.0,
                            Err(_) => {
                                fail(&fault, TransportError::MalformedFrame);
                                return;
                            }
                        };
                        if sender.try_send(value).is_err() {
                            fail(&fault, TransportError::FrameQueueOverflow);
                            return;
                        }
                        frame.clear();
                    } else if frame.len() == limit {
                        fail(&fault, TransportError::FrameTooLarge);
                        return;
                    } else {
                        frame.push(*byte);
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2))
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                fail(&fault, io_error("read stdout", error));
                return;
            }
        }
    }
}
fn read_stderr(
    mut pipe: impl Read,
    stop: Arc<AtomicBool>,
    fault: Fault,
    buffer: Arc<Mutex<DiagnosticBuffer>>,
    limits: TransportLimits,
) {
    let mut line = Vec::with_capacity(limits.max_diagnostic_bytes.min(4096));
    let mut truncated = false;
    let mut bytes = [0; 4096];
    while !stop.load(Ordering::Acquire) {
        match pipe.read(&mut bytes) {
            Ok(0) => {
                if !line.is_empty() || truncated {
                    push_diagnostic(
                        &buffer,
                        &line,
                        truncated,
                        limits.diagnostic_queue_capacity,
                        limits.max_diagnostic_bytes,
                    );
                }
                return;
            }
            Ok(count) => {
                for byte in &bytes[..count] {
                    if *byte == b'\n' {
                        push_diagnostic(
                            &buffer,
                            &line,
                            truncated,
                            limits.diagnostic_queue_capacity,
                            limits.max_diagnostic_bytes,
                        );
                        line.clear();
                        truncated = false;
                    } else if line.len() < limits.max_diagnostic_bytes {
                        line.push(*byte);
                    } else {
                        truncated = true;
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2))
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                fail(&fault, io_error("read stderr", error));
                return;
            }
        }
    }
}
fn push_diagnostic(
    buffer: &Mutex<DiagnosticBuffer>,
    bytes: &[u8],
    truncated: bool,
    capacity: usize,
    byte_limit: usize,
) {
    let mut buffer = buffer.lock().unwrap_or_else(|poison| poison.into_inner());
    if buffer.entries.len() == capacity {
        buffer.entries.pop_front();
        buffer.dropped = buffer.dropped.saturating_add(1);
    }
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    let expanded = text.len() > byte_limit;
    if expanded {
        let mut boundary = byte_limit;
        while !text.is_char_boundary(boundary) {
            boundary -= 1;
        }
        text.truncate(boundary);
    }
    buffer.entries.push_back(Diagnostic {
        text,
        truncated: truncated || expanded,
    });
}

/// serde_json::Value normally collapses duplicate object keys. Preserve protocol
/// meaning by rejecting them at every nesting level before JSON-RPC sees a Value.
struct StrictValue(Value);
impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct StrictVisitor;
        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = StrictValue;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("JSON with unique object fields")
            }
            fn visit_bool<E: serde::de::Error>(
                self,
                v: bool,
            ) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(Value::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(v.into())))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(v.into())))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| StrictValue(Value::Number(n)))
                    .ok_or_else(|| E::custom("invalid JSON number"))
            }
            fn visit_str<E: serde::de::Error>(
                self,
                v: &str,
            ) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(Value::String(v.into())))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(Value::String(v)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<StrictValue>()? {
                    values.push(value.0);
                }
                Ok(StrictValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom("duplicate JSON field"));
                    }
                    values.insert(key, map.next_value::<StrictValue>()?.0);
                }
                Ok(StrictValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(StrictVisitor)
    }
}
