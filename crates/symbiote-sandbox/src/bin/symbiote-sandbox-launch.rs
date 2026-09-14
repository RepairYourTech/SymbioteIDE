//! Trusted, single-threaded descriptor boundary before bubblewrap. No untrusted
//! code executes until every inherited descriptor other than stdio is closed.

// The record of the sources this build compiled, written by the build script
// into OUT_DIR and included here so it travels inside the binary. A proof can
// then tell whether the launcher it is about to drive was built from the
// sources under test, without a record file that a later build could refresh
// on its own. It is a `static` in the binary's root because a `static` in the
// library is dropped when this crate links it without referencing it. See
// `symbiote-source-stamp`, which names this file.
include!(concat!(env!("OUT_DIR"), "/source_record.rs"));

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
    // Read, not merely declared: a static the binary never reads is one the
    // compiler or linker may drop, and Rust 1.85 does drop this one (`#[used]`
    // did not keep it, and the desktop proofs refused its daemon for carrying
    // no record at all). A record that can be dropped cannot bind a binary to
    // the sources a proof drives it against.
    std::hint::black_box(SOURCE_RECORD);
    if run().is_err() {
        eprintln!("sandbox launcher unavailable");
        std::process::exit(126);
    }
}
