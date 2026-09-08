//! Private local IPC. The OS user is the trust boundary, not a client-supplied identity.
use nix::fcntl::{Flock, FlockArg, OFlag};
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use nix::unistd::geteuid;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const FRAME_LIMIT: usize = 65_536;

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}

/// Refuses shared or symlink state directories. Never changes an existing directory's permissions.
pub fn private_directory(path: &Path) -> io::Result<()> {
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.uid() != geteuid().as_raw() || metadata.mode() & 0o077 != 0 {
        return Err(invalid(
            "state directory must be owned by this user, private, and not a symlink",
        ));
    }
    Ok(())
}

pub struct LocalListener {
    pub listener: UnixListener,
    path: PathBuf,
    _lock: Flock<File>,
}

impl LocalListener {
    pub fn bind(directory: &Path) -> io::Result<Self> {
        private_directory(directory)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(OFlag::O_NOFOLLOW.bits())
            .open(directory.join("host.lock"))?;
        let metadata = lock.metadata()?;
        if !metadata.is_file()
            || metadata.uid() != geteuid().as_raw()
            || metadata.mode() & 0o077 != 0
        {
            return Err(invalid("host lock is not a private owned file"));
        }
        let lock = Flock::lock(lock, FlockArg::LockExclusiveNonblock)
            .map_err(|(_, error)| io::Error::new(io::ErrorKind::AddrInUse, error))?;
        let path = directory.join("host.sock");
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_socket() && meta.uid() == geteuid().as_raw() => {
                fs::remove_file(&path)?
            }
            Ok(_) => {
                return Err(invalid(
                    "refusing to replace a non-socket or foreign socket",
                ));
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let listener = UnixListener::bind(&path)?;
        // The private parent is the access boundary even before socket chmod.
        fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o600))?;
        Ok(Self {
            listener,
            path,
            _lock: lock,
        })
    }

    pub fn authenticate(stream: &UnixStream) -> io::Result<()> {
        let credential = getsockopt(stream, PeerCredentials).map_err(io::Error::from)?;
        if credential.uid() != geteuid().as_raw() {
            return Err(invalid("local peer user does not own this Host"));
        }
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        Ok(())
    }
}

impl Drop for LocalListener {
    fn drop(&mut self) {
        // The exclusive lock remains held until after this drop body.
        let _ = fs::remove_file(&self.path);
    }
}

pub fn read_frame(stream: &UnixStream) -> io::Result<Vec<u8>> {
    read_frame_until(stream, Instant::now() + Duration::from_secs(2))
}

pub fn write_response(stream: &UnixStream, bytes: &[u8]) -> io::Result<()> {
    write_until(stream, bytes, Instant::now() + Duration::from_secs(2))
}

fn write_until(mut stream: &UnixStream, mut bytes: &[u8], deadline: Instant) -> io::Result<()> {
    while !bytes.is_empty() {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "response deadline exceeded"))?;
        stream.set_write_timeout(Some(remaining))?;
        match stream.write(bytes)? {
            0 => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "response disconnected",
                ));
            }
            written => bytes = &bytes[written..],
        }
    }
    Ok(())
}

fn read_frame_until(mut stream: &UnixStream, deadline: Instant) -> io::Result<Vec<u8>> {
    let mut frame = Vec::new();
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "request deadline exceeded"))?;
        stream.set_read_timeout(Some(remaining))?;
        let mut buffer = [0; 1024];
        let count = stream.read(&mut buffer)?;
        if count == 0 || frame.len() + count > FRAME_LIMIT + 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request requires a bounded newline-delimited frame",
            ));
        }
        frame.extend_from_slice(&buffer[..count]);
        if let Some(position) = frame.iter().position(|byte| *byte == b'\n') {
            if position != frame.len() - 1 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "one request per connection is required",
                ));
            }
            frame.pop();
            return Ok(frame);
        }
        if frame.len() > FRAME_LIMIT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request exceeds frame bound",
            ));
        }
    }
}

pub fn exchange(directory: &Path, request: &[u8]) -> io::Result<Vec<u8>> {
    private_directory(directory)?;
    if request.len() > FRAME_LIMIT || request.contains(&b'\n') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "request must be a single bounded JSON frame",
        ));
    }
    let mut stream = UnixStream::connect(directory.join("host.sock"))?;
    LocalListener::authenticate(&stream)?;
    stream.write_all(request)?;
    stream.write_all(b"\n")?;
    // Responses contain bounded pages, but can exceed one request frame.
    let mut bytes = Vec::new();
    stream
        .take((symbiote_protocol::MAX_RESPONSE_BYTES + 2) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > symbiote_protocol::MAX_RESPONSE_BYTES + 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "response exceeds client bound",
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trickled_bytes_do_not_extend_the_absolute_deadline() {
        let (mut sender, receiver) = UnixStream::pair().unwrap();
        let started = Instant::now();
        let writer = std::thread::spawn(move || {
            for _ in 0..20 {
                if sender.write_all(b" ").is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        assert!(read_frame_until(&receiver, started + Duration::from_millis(100)).is_err());
        assert!(started.elapsed() < Duration::from_millis(350));
        drop(receiver);
        writer.join().unwrap();
    }

    #[test]
    fn slow_response_consumer_does_not_extend_write_deadline() {
        let (sender, mut receiver) = UnixStream::pair().unwrap();
        nix::sys::socket::setsockopt(&sender, nix::sys::socket::sockopt::SndBuf, &4096).unwrap();
        let reader = std::thread::spawn(move || {
            let mut bytes = [0; 512];
            for _ in 0..20 {
                if receiver.read(&mut bytes).unwrap_or(0) == 0 {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let started = Instant::now();
        assert!(
            write_until(
                &sender,
                &vec![0; 1024 * 1024],
                started + Duration::from_millis(100)
            )
            .is_err()
        );
        assert!(started.elapsed() < Duration::from_millis(350));
        drop(sender);
        reader.join().unwrap();
    }
}
