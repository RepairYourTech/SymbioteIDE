//! Immutable generation publication. Hashes detect drift, not malicious same-user rewriting.
//! No activation, in-place replacement, cleanup or runtime launch is performed here.
use nix::{
    dir::Dir,
    fcntl::{OFlag, open, openat},
    sys::stat::{Mode, SFlag, fstat, mkdirat},
    unistd::geteuid,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fs::File,
    io::{Read, Write},
    os::fd::AsFd,
    path::{Component, Path, PathBuf},
};

const MAX_FILE: usize = 1_048_576;
const MAX_TOTAL: usize = 4 * MAX_FILE;
const MAX_FILES: usize = 16;
const MAX_MANIFEST: usize = 16_384;
const MANIFEST: &str = ".manifest.json";
const READY: &str = ".ready";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerationError {
    InvalidInput,
    UnsafePath,
    AlreadyExists,
    Incomplete,
    IntegrityMismatch,
    BoundsExceeded,
    Io,
}
impl fmt::Display for GenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "projection generation rejected: {self:?}")
    }
}
impl std::error::Error for GenerationError {}
pub type Result<T> = std::result::Result<T, GenerationError>;

#[derive(Clone, PartialEq, Eq)]
pub struct FileReceipt {
    pub bytes: u64,
    pub sha256: String,
}
impl fmt::Debug for FileReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileReceipt")
            .field("bytes", &self.bytes)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct Receipt {
    pub generation: String,
    pub files: BTreeMap<String, FileReceipt>,
}
impl fmt::Debug for Receipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Receipt")
            .field("file_count", &self.files.len())
            .finish_non_exhaustive()
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    generation: String,
    files: Vec<Record>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    name: String,
    bytes: u64,
    sha256: String,
}

fn name_valid(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name.as_bytes()[0].is_ascii_alphanumeric()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn directory_flags() -> OFlag {
    OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC
}

fn check_private(fd: &impl AsFd, directory: bool) -> Result<()> {
    let stat = fstat(fd).map_err(|_| GenerationError::Io)?;
    let kind = if directory {
        SFlag::S_IFDIR
    } else {
        SFlag::S_IFREG
    };
    let mode = if directory { 0o700 } else { 0o600 };
    if SFlag::from_bits_truncate(stat.st_mode) & SFlag::S_IFMT != kind
        || stat.st_uid != geteuid().as_raw()
        || stat.st_mode & 0o7777 != mode
        || (!directory && stat.st_nlink != 1)
    {
        return Err(GenerationError::UnsafePath);
    }
    Ok(())
}

// Resolve each component against a pinned directory fd; no canonicalize/check/open race.
fn root_directory(path: &Path) -> Result<File> {
    let path: PathBuf = if path.is_absolute() {
        path.into()
    } else {
        std::env::current_dir()
            .map_err(|_| GenerationError::Io)?
            .join(path)
    };
    let mut fd = open(Path::new("/"), directory_flags(), Mode::empty())
        .map_err(|_| GenerationError::UnsafePath)?;
    for component in path.components() {
        match component {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(name) => {
                fd = openat(&fd, name, directory_flags(), Mode::empty())
                    .map_err(|_| GenerationError::UnsafePath)?;
            }
            _ => return Err(GenerationError::UnsafePath),
        }
    }
    check_private(&fd, true)?;
    Ok(File::from(fd))
}
fn generation_directory(root: &File, generation: &str) -> Result<File> {
    let fd = openat(root, generation, directory_flags(), Mode::empty())
        .map_err(|_| GenerationError::UnsafePath)?;
    check_private(&fd, true)?;
    Ok(File::from(fd))
}
fn write_new(directory: &File, name: &str, bytes: &[u8]) -> Result<()> {
    let fd = openat(
        directory,
        name,
        OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::S_IRUSR | Mode::S_IWUSR,
    )
    .map_err(|_| GenerationError::Io)?;
    check_private(&fd, false)?;
    let mut file = File::from(fd);
    file.write_all(bytes).map_err(|_| GenerationError::Io)?;
    file.sync_all().map_err(|_| GenerationError::Io)
}
fn read_file(directory: &File, name: &str, limit: usize) -> Result<Vec<u8>> {
    let fd = openat(
        directory,
        name,
        OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_NONBLOCK | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| GenerationError::IntegrityMismatch)?;
    check_private(&fd, false)?;
    let stat = fstat(&fd).map_err(|_| GenerationError::Io)?;
    if stat.st_size < 0 || stat.st_size as u64 > limit as u64 {
        return Err(GenerationError::BoundsExceeded);
    }
    let mut bytes = Vec::new();
    File::from(fd)
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| GenerationError::Io)?;
    if bytes.len() > limit {
        return Err(GenerationError::BoundsExceeded);
    }
    Ok(bytes)
}

/// Publishes a fresh generation; even identical retries never overwrite an existing directory.
/// Errors preserve the directory for inspection. After readiness creation an
/// error may leave a verifiable generation with uncertain durability; reconcile
/// using `verify` without assuming either rollback or successful publication.
pub fn publish(
    root: &Path,
    generation: &str,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<Receipt> {
    if !name_valid(generation) || files.keys().any(|name| !name_valid(name)) {
        return Err(GenerationError::InvalidInput);
    }
    if files.len() > MAX_FILES
        || files.values().any(|bytes| bytes.len() > MAX_FILE)
        || files.values().map(Vec::len).sum::<usize>() > MAX_TOTAL
    {
        return Err(GenerationError::BoundsExceeded);
    }
    let root = root_directory(root)?;
    mkdirat(&root, generation, Mode::S_IRWXU).map_err(|error| {
        if error == nix::errno::Errno::EEXIST {
            GenerationError::AlreadyExists
        } else {
            GenerationError::Io
        }
    })?;
    let directory = generation_directory(&root, generation)?;
    let mut records = Vec::new();
    let mut receipts = BTreeMap::new();
    for (name, bytes) in files {
        write_new(&directory, name, bytes)?;
        let sha256 = hash(bytes);
        records.push(Record {
            name: name.clone(),
            bytes: bytes.len() as u64,
            sha256: sha256.clone(),
        });
        receipts.insert(
            name.clone(),
            FileReceipt {
                bytes: bytes.len() as u64,
                sha256,
            },
        );
    }
    directory.sync_all().map_err(|_| GenerationError::Io)?;
    let manifest = serde_json::to_vec(&Manifest {
        version: 1,
        generation: generation.into(),
        files: records,
    })
    .map_err(|_| GenerationError::Io)?;
    if manifest.len() > MAX_MANIFEST {
        return Err(GenerationError::BoundsExceeded);
    }
    write_new(&directory, MANIFEST, &manifest)?;
    directory.sync_all().map_err(|_| GenerationError::Io)?;
    write_new(&directory, READY, hash(&manifest).as_bytes())?;
    directory.sync_all().map_err(|_| GenerationError::Io)?;
    root.sync_all().map_err(|_| GenerationError::Io)?;
    Ok(Receipt {
        generation: generation.into(),
        files: receipts,
    })
}

/// Verifies readiness, private permissions, exact entry set, lengths and content hashes.
/// A successful observation is not a lock against later writes by the same OS user.
pub fn verify(root: &Path, generation: &str) -> Result<Receipt> {
    if !name_valid(generation) {
        return Err(GenerationError::InvalidInput);
    }
    let root = root_directory(root)?;
    let directory = generation_directory(&root, generation)?;
    let mut entries = Dir::openat(&directory, ".", directory_flags(), Mode::empty())
        .map_err(|_| GenerationError::Io)?;
    let mut names = BTreeSet::new();
    for entry in entries.iter() {
        let entry = entry.map_err(|_| GenerationError::Io)?;
        let name = entry
            .file_name()
            .to_str()
            .map_err(|_| GenerationError::IntegrityMismatch)?;
        if matches!(name, "." | "..") {
            continue;
        }
        names.insert(name.to_owned());
        if names.len() > MAX_FILES + 2 {
            return Err(GenerationError::BoundsExceeded);
        }
    }
    if !names.contains(READY) || !names.contains(MANIFEST) {
        return Err(GenerationError::Incomplete);
    }
    let manifest_bytes = read_file(&directory, MANIFEST, MAX_MANIFEST)?;
    if read_file(&directory, READY, 64)? != hash(&manifest_bytes).as_bytes() {
        return Err(GenerationError::IntegrityMismatch);
    }
    let manifest: Manifest =
        serde_json::from_slice(&manifest_bytes).map_err(|_| GenerationError::IntegrityMismatch)?;
    if manifest.version != 1
        || manifest.generation != generation
        || manifest.files.len() > MAX_FILES
    {
        return Err(GenerationError::IntegrityMismatch);
    }
    let mut expected = BTreeSet::from([MANIFEST.to_owned(), READY.to_owned()]);
    let mut total = 0_u64;
    let mut files = BTreeMap::new();
    for record in manifest.files {
        if !name_valid(&record.name) || !expected.insert(record.name.clone()) {
            return Err(GenerationError::IntegrityMismatch);
        }
        if record.bytes > MAX_FILE as u64 {
            return Err(GenerationError::BoundsExceeded);
        }
        total += record.bytes;
        if total > MAX_TOTAL as u64 {
            return Err(GenerationError::BoundsExceeded);
        }
        let bytes = read_file(&directory, &record.name, MAX_FILE)?;
        if bytes.len() as u64 != record.bytes || hash(&bytes) != record.sha256 {
            return Err(GenerationError::IntegrityMismatch);
        }
        files.insert(
            record.name,
            FileReceipt {
                bytes: record.bytes,
                sha256: record.sha256,
            },
        );
    }
    if expected != names {
        return Err(GenerationError::IntegrityMismatch);
    }
    Ok(Receipt {
        generation: generation.into(),
        files,
    })
}
