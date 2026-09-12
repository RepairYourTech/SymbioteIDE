//! Trusted, single-threaded descriptor boundary before bubblewrap. No untrusted
//! code executes until every inherited descriptor other than stdio is closed.

/// The id of the source record this build wrote, embedded in the binary so a
/// proof can tell which record produced the launcher it is about to drive
/// rather than trusting a record a failed compile may have refreshed. It lives
/// in the binary's root because a static in the library is dropped when this
/// crate links it without referencing it. See `symbiote-source-stamp`.
#[used]
#[doc(hidden)]
static SOURCE_RECORD: &str = env!("SYMBIOTE_SOURCE_RECORD");

use std::os::unix::{ffi::OsStrExt, process::CommandExt};

fn run() -> Result<(), ()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() > 256 || args.iter().map(|a| a.as_bytes().len()).sum::<usize>() > 262_144 {
        return Err(());
    }
    let mut descriptors = Vec::new();
    for entry in std::fs::read_dir("/proc/self/fd").map_err(|_| ())? {
        let entry = entry.map_err(|_| ())?;
        let fd = entry
            .file_name()
            .to_str()
            .ok_or(())?
            .parse::<i32>()
            .map_err(|_| ())?;
        if fd > 2 {
            descriptors.push(fd);
        }
        if descriptors.len() > 65_536 {
            return Err(());
        }
    }
    // read_dir's own descriptor is closed before this loop. This process has
    // no other threads or signal handlers that allocate descriptors.
    for fd in descriptors {
        match nix::unistd::close(fd) {
            Ok(()) | Err(nix::errno::Errno::EBADF) => {}
            Err(_) => return Err(()),
        }
    }
    let _error = std::process::Command::new("/usr/bin/bwrap")
        .args(args)
        .env_clear()
        .exec();
    Err(())
}
fn main() {
    if run().is_err() {
        eprintln!("sandbox launcher unavailable");
        std::process::exit(126);
    }
}
