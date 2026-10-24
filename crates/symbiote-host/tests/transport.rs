use std::fs;
use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use symbiote_host::transport::{FRAME_LIMIT, LocalListener, private_directory, read_frame};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "symbiote-ipc-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(!path.exists());
        private_directory(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn shared_directory_and_symlink_are_refused_without_permission_changes() {
    let directory = Directory::new();
    fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(private_directory(&directory.0).is_err());
    assert_eq!(
        fs::metadata(&directory.0).unwrap().permissions().mode() & 0o777,
        0o755
    );
    fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o700)).unwrap();
    let alias = directory.0.join("alias");
    symlink(&directory.0, &alias).unwrap();
    assert!(private_directory(&alias).is_err());
}

#[test]
fn exclusive_lock_preserves_live_socket_and_stale_socket_recovers() {
    let directory = Directory::new();
    let first = LocalListener::bind(&directory.0).unwrap();
    assert!(LocalListener::bind(&directory.0).is_err());
    assert!(UnixStream::connect(directory.0.join("host.sock")).is_ok());
    drop(first);
    let stale = std::os::unix::net::UnixListener::bind(directory.0.join("host.sock")).unwrap();
    drop(stale);
    let recovered = LocalListener::bind(&directory.0).unwrap();
    assert_eq!(
        fs::metadata(directory.0.join("host.sock"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    drop(recovered);
}

/// A second Host on a live state directory is refused by name. The safety property — the first
/// Host's socket survives — is the sibling case's; this one is the diagnostic an operator reads
/// when a unit is already running and the binary is started by hand. `EAGAIN: Try again` names
/// neither the state directory nor the cause, while every other refusal in this module names both,
/// and a beta operator meets this one by accident rather than by intent.
#[test]
fn a_second_host_is_refused_by_the_lock_it_could_not_take() {
    let directory = Directory::new();
    let first = LocalListener::bind(&directory.0).unwrap();
    let error = match LocalListener::bind(&directory.0) {
        Ok(_) => panic!("a second Host took the operator lock the first Host holds"),
        Err(error) => error,
    };
    assert_eq!(
        error.kind(),
        std::io::ErrorKind::AddrInUse,
        "a caller that distinguishes this refusal from a permission refusal loses that: {error}"
    );
    let said = error.to_string();
    let lock = directory.0.join("host.lock").display().to_string();
    assert!(
        said.contains(&lock),
        "the refusal does not name the lock it could not take, so an operator is left with the \
         errno alone: {said}"
    );
    assert!(
        said.contains("another Host already holds"),
        "the refusal does not say the lock is already held, so it reads as a transient error \
         rather than as another Host serving this directory: {said}"
    );
    drop(first);
}

#[test]
fn lock_symlink_and_non_socket_path_are_preserved() {
    let directory = Directory::new();
    fs::write(directory.0.join("sentinel"), b"keep").unwrap();
    symlink(directory.0.join("sentinel"), directory.0.join("host.lock")).unwrap();
    assert!(LocalListener::bind(&directory.0).is_err());
    assert_eq!(fs::read(directory.0.join("sentinel")).unwrap(), b"keep");
    fs::remove_file(directory.0.join("host.lock")).unwrap();
    fs::write(directory.0.join("host.sock"), b"not a socket").unwrap();
    assert!(LocalListener::bind(&directory.0).is_err());
    assert_eq!(
        fs::read(directory.0.join("host.sock")).unwrap(),
        b"not a socket"
    );
}

#[test]
fn incomplete_oversized_and_valid_frames_have_bounded_dispositions() {
    for (bytes, expected) in [
        (b"{}\n".to_vec(), true),
        (b"{}".to_vec(), false),
        (vec![b'x'; FRAME_LIMIT + 1], false),
    ] {
        let (mut sender, receiver) = UnixStream::pair().unwrap();
        let send = std::thread::spawn(move || {
            let _ = sender.write_all(&bytes);
        });
        assert_eq!(read_frame(&receiver).is_ok(), expected);
        drop(receiver);
        send.join().unwrap();
    }
}
