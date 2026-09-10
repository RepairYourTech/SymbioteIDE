//! Bounded Unix-socket client framing for the workflow driver. One
//! request per connection (the daemon's wire discipline), same-UID peer
//! check both directions, newline-framed requests, bounded responses. The
//! daemon-side counterpart is `symbiote-host::transport`; the bounds are
//! protocol constants.
use nix::sys::socket::getsockopt;
use nix::sys::socket::sockopt::PeerCredentials;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};
use symbiote_protocol::MAX_RESPONSE_BYTES;

pub const SOCKET_NAME: &str = "host.sock";
const FRAME_LIMIT: usize = 64 * 1024;
const DEADLINE: Duration = Duration::from_secs(30);

fn invalid(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

/// The peer must be owned by the same OS user — the same authenticated
/// local-owner policy the daemon enforces on its side of the socket.
fn authenticate(stream: &UnixStream) -> std::io::Result<()> {
    let credential = getsockopt(stream, PeerCredentials)?;
    if credential.uid() != nix::unistd::geteuid().as_raw() {
        return Err(invalid("local peer user does not own this Host"));
    }
    stream.set_read_timeout(Some(DEADLINE))?;
    stream.set_write_timeout(Some(DEADLINE))?;
    Ok(())
}

fn read_response(mut stream: &UnixStream) -> std::io::Result<Vec<u8>> {
    let deadline = Instant::now() + DEADLINE;
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| io_timeout("response deadline exceeded"))?;
        stream.set_read_timeout(Some(remaining))?;
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&buffer[..count]);
        if bytes.len() > MAX_RESPONSE_BYTES + 1 {
            return Err(invalid("response exceeds client bound"));
        }
    }
}

fn io_timeout(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::TimedOut, message)
}

/// One bounded exchange: connect, authenticate, write the single JSON
/// frame, read the bounded response. `Err` = transport failure (the
/// command's disposition is unknown; the SDK's recover replays it).
pub fn exchange(directory: &Path, request: &[u8]) -> std::io::Result<Vec<u8>> {
    if request.len() > FRAME_LIMIT || request.contains(&b'\n') {
        return Err(invalid("request must be a single bounded JSON frame"));
    }
    let mut stream = UnixStream::connect(directory.join(SOCKET_NAME))?;
    authenticate(&stream)?;
    stream.write_all(request)?;
    stream.write_all(b"\n")?;
    read_response(&stream)
}
