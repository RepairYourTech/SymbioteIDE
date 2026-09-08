#![cfg(target_os = "linux")]
use std::{
    collections::BTreeMap,
    fs::{self, DirBuilder},
    os::unix::fs::{DirBuilderExt, PermissionsExt, symlink},
    path::PathBuf,
    sync::{
        Arc, Barrier,
        atomic::{AtomicU64, Ordering},
    },
};
use symbiote_projection::generation::{GenerationError, publish, verify};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct PrivateRoot(PathBuf);
impl PrivateRoot {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "symbiote-generation-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("private test directory creation failed: {error}"),
            }
        }
    }
}
impl Drop for PrivateRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn files() -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([
        ("config.toml".into(), b"model = 'fixture'\n".to_vec()),
        ("AGENTS.md".into(), b"Test instructions".to_vec()),
    ])
}

#[test]
fn published_generation_verifies_and_has_private_permissions() {
    let root = PrivateRoot::new();
    let receipt = publish(&root.0, "generation-1", &files()).unwrap();
    assert_eq!(receipt, verify(&root.0, "generation-1").unwrap());
    assert_eq!(receipt.files["config.toml"].bytes, 18);
    assert!(!format!("{receipt:?}").contains("generation-1"));
    assert!(!format!("{receipt:?}").contains("config.toml"));
    assert_eq!(
        fs::metadata(root.0.join("generation-1"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o700
    );
    for name in ["config.toml", "AGENTS.md", ".manifest.json", ".ready"] {
        assert_eq!(
            fs::metadata(root.0.join("generation-1").join(name))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o600
        );
    }
    assert_eq!(
        publish(&root.0, "generation-1", &files()),
        Err(GenerationError::AlreadyExists)
    );
    assert_eq!(receipt, verify(&root.0, "generation-1").unwrap());
}

#[test]
fn concurrent_publish_has_one_winner_and_never_mixes_files() {
    let root = PrivateRoot::new();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|n| {
            let path = root.0.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let files = BTreeMap::from([("config.toml".into(), vec![n; 1000])]);
                barrier.wait();
                publish(&path, "race", &files)
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == Err(GenerationError::AlreadyExists))
            .count(),
        1
    );
    assert_eq!(
        verify(&root.0, "race").unwrap(),
        results.into_iter().find_map(Result::ok).unwrap()
    );
}

#[test]
fn partial_publications_remain_incomplete_and_are_preserved() {
    let root = PrivateRoot::new();
    publish(&root.0, "partial", &files()).unwrap();
    fs::remove_file(root.0.join("partial/.ready")).unwrap();
    assert_eq!(verify(&root.0, "partial"), Err(GenerationError::Incomplete));
    assert_eq!(
        publish(&root.0, "partial", &files()),
        Err(GenerationError::AlreadyExists)
    );
    assert!(root.0.join("partial/config.toml").exists());
    DirBuilder::new()
        .mode(0o700)
        .create(root.0.join("empty"))
        .unwrap();
    assert_eq!(verify(&root.0, "empty"), Err(GenerationError::Incomplete));
}

#[test]
fn content_manifest_marker_and_entry_set_tampering_is_rejected() {
    let root = PrivateRoot::new();
    for generation in [
        "content",
        "manifest",
        "marker",
        "extra",
        "missing",
        "directory",
    ] {
        publish(&root.0, generation, &files()).unwrap();
    }
    fs::write(root.0.join("content/config.toml"), b"model = 'changed'\n").unwrap();
    fs::write(root.0.join("manifest/.manifest.json"), b"{}").unwrap();
    fs::write(root.0.join("marker/.ready"), b"0".repeat(64)).unwrap();
    fs::write(root.0.join("extra/user-owned.txt"), b"preserve me").unwrap();
    fs::remove_file(root.0.join("missing/config.toml")).unwrap();
    fs::remove_file(root.0.join("directory/config.toml")).unwrap();
    DirBuilder::new()
        .mode(0o700)
        .create(root.0.join("directory/config.toml"))
        .unwrap();
    for generation in [
        "content",
        "manifest",
        "marker",
        "extra",
        "missing",
        "directory",
    ] {
        assert!(verify(&root.0, generation).is_err());
    }
    assert_eq!(
        fs::read(root.0.join("extra/user-owned.txt")).unwrap(),
        b"preserve me"
    );
}

#[test]
fn unsafe_paths_symlinks_hardlinks_and_permissions_are_rejected() {
    let root = PrivateRoot::new();
    for name in [
        "",
        "..",
        "../escape",
        "a/b",
        ".ready",
        "/absolute",
        "bad\0name",
    ] {
        assert_eq!(
            publish(&root.0, name, &files()),
            Err(GenerationError::InvalidInput)
        );
        assert_eq!(
            publish(
                &root.0,
                "invalid-file",
                &BTreeMap::from([(name.into(), vec![])])
            ),
            Err(GenerationError::InvalidInput)
        );
    }
    assert!(!root.0.join("invalid-file").exists());
    let alias = root.0.join("alias");
    symlink(&root.0, &alias).unwrap();
    assert_eq!(
        publish(&alias, "new", &files()),
        Err(GenerationError::UnsafePath)
    );
    assert_eq!(
        publish(&alias.join("child"), "new", &files()),
        Err(GenerationError::UnsafePath)
    );
    publish(&root.0, "linked", &files()).unwrap();
    fs::remove_file(root.0.join("linked/config.toml")).unwrap();
    symlink("AGENTS.md", root.0.join("linked/config.toml")).unwrap();
    assert!(verify(&root.0, "linked").is_err());
    publish(&root.0, "hardlink", &files()).unwrap();
    fs::hard_link(
        root.0.join("hardlink/config.toml"),
        root.0.join("second-link"),
    )
    .unwrap();
    assert_eq!(
        verify(&root.0, "hardlink"),
        Err(GenerationError::UnsafePath)
    );
    publish(&root.0, "permissions", &files()).unwrap();
    fs::set_permissions(
        root.0.join("permissions/config.toml"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert_eq!(
        verify(&root.0, "permissions"),
        Err(GenerationError::UnsafePath)
    );
    fs::set_permissions(&root.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        publish(&root.0, "public-root", &files()),
        Err(GenerationError::UnsafePath)
    );
}

#[test]
fn limits_are_rejected_before_creating_a_generation() {
    let root = PrivateRoot::new();
    let too_many = (0..17).map(|n| (format!("f{n}"), vec![])).collect();
    let too_large = BTreeMap::from([("config.toml".into(), vec![0; 1_048_577])]);
    let too_total = (0..5)
        .map(|n| (format!("f{n}"), vec![0; 1_048_576]))
        .collect();
    for files in [too_many, too_large, too_total] {
        assert_eq!(
            publish(&root.0, "oversized", &files),
            Err(GenerationError::BoundsExceeded)
        );
    }
    assert!(!root.0.join("oversized").exists());
}

#[test]
fn unsupported_fifo_does_not_block_and_generation_symlink_is_never_followed() {
    let root = PrivateRoot::new();
    publish(&root.0, "fifo", &files()).unwrap();
    fs::remove_file(root.0.join("fifo/config.toml")).unwrap();
    nix::unistd::mkfifo(
        &root.0.join("fifo/config.toml"),
        nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
    )
    .unwrap();
    assert_eq!(verify(&root.0, "fifo"), Err(GenerationError::UnsafePath));
    symlink("fifo", root.0.join("generation-alias")).unwrap();
    assert_eq!(
        verify(&root.0, "generation-alias"),
        Err(GenerationError::UnsafePath)
    );
    assert_eq!(
        publish(&root.0, "generation-alias", &files()),
        Err(GenerationError::AlreadyExists)
    );
    assert!(root.0.join("generation-alias").is_symlink());
}
