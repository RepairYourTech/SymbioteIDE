//! Stable installation identity, private to this Host state directory.
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
use symbiote_domain::HostId;

fn invalid() -> std::io::Error {
    std::io::Error::other("invalid private Host identity")
}

pub(crate) fn load(directory: &Path) -> std::io::Result<HostId> {
    let path = directory.join("host-id");
    let existing = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
        .open(&path);
    match existing {
        Ok(mut file) => {
            let meta = file.metadata()?;
            if !meta.is_file()
                || meta.nlink() != 1
                || meta.uid() != nix::unistd::geteuid().as_raw()
                || meta.mode() & 0o777 != 0o600
                || meta.len() != 37
            {
                return Err(invalid());
            }
            let mut content = String::new();
            Read::by_ref(&mut file)
                .take(38)
                .read_to_string(&mut content)?;
            if content.len() != 37
                || !content.starts_with("host_")
                || !content[5..]
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(invalid());
            }
            HostId::new(content).map_err(|_| invalid())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut random = [0u8; 16];
            File::open("/dev/urandom")?.read_exact(&mut random)?;
            let id = format!(
                "host_{}",
                random
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
            let temporary = directory.join(format!(".{id}.new"));
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temporary)?;
            file.write_all(id.as_bytes())?;
            file.sync_all()?;
            // Caller holds the exclusive private directory Host lock.
            fs::rename(&temporary, &path)?;
            File::open(directory)?.sync_all()?;
            HostId::new(id).map_err(|_| invalid())
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::fs::{PermissionsExt, symlink},
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    #[test]
    fn private_identity_is_stable_and_rejects_symlinks_modes_and_malformed_content() {
        let directory = std::env::temp_dir().join(format!(
            "symbiote-identity-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        crate::transport::private_directory(&directory).unwrap();
        let id = load(&directory).unwrap();
        assert_eq!(load(&directory).unwrap(), id);
        let path = directory.join("host-id");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load(&directory).is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&path, b"not-a-host").unwrap();
        assert!(load(&directory).is_err());
        fs::remove_file(&path).unwrap();
        symlink(directory.join("missing"), &path).unwrap();
        assert!(load(&directory).is_err());
        assert!(
            fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        fs::remove_file(&path).unwrap();
        nix::unistd::mkfifo(
            &path,
            nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
        )
        .unwrap();
        assert!(load(&directory).is_err());
        fs::remove_dir_all(directory).unwrap();
    }
}
