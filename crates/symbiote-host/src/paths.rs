//! The Host's own directories (#180): the XDG base directories, explicit overrides, and the
//! refusals that keep an unresolved Host from writing somewhere nobody intended.
//!
//! `symbioted` took `--state-dir` and nothing else, so every deployment had to name the directory
//! itself and there was no defined answer for a user service, a container or a headless install.
//! This module is that answer: one owner for where the Host's state and configuration live, read
//! from the environment once and used by the daemon for the socket it binds, the database it opens
//! and the operator configuration it discovers.
//!
//! The precedence is the one the CLI already applies to its policy (`--policy FILE`, else
//! `$SYMBIOTE_CLI_POLICY`): an explicit flag wins, then this Host's own override variable, then the
//! XDG base directory the specification names, then the specification's `$HOME` default. A
//! *relative* value in an `XDG_*` variable is ignored, which is what the specification requires —
//! but a relative value in this Host's own override is an operator error and is refused by name,
//! because silently ignoring it would run the Host somewhere other than the operator said.
//!
//! What this does not do: create or permission the directories (the transport's
//! `private_directory` does that for the state directory, and the daemon refuses an existing
//! directory that is shared, foreign-owned or a symlink), and resolve the cache, log or runtime
//! directories, which nothing in the daemon writes to yet — those stay with the diagnostic and
//! service owners rather than being resolved into unused scaffolding here.

use std::path::{Path, PathBuf};

/// The state directory an operator names without a flag; the flag wins over it.
pub const STATE_OVERRIDE: &str = "SYMBIOTE_HOST_STATE_DIR";
/// The configuration directory an operator names without a flag; the flag wins over it.
pub const CONFIG_OVERRIDE: &str = "SYMBIOTE_HOST_CONFIG_DIR";
/// The application directory under each XDG base directory.
pub const APPLICATION: &str = "symbiote";
/// The layout this module owns: `docs/contracts/host.md` publishes these names and
/// `tests/paths.rs` holds the declarations to it, so a file renamed here or there reds by name.
/// `control.sqlite3` holds canonical state; `host.sock` is the local control socket and
/// `host.lock` the exclusive lock that keeps a second daemon off it; and
/// `runtime-inventory.json` is the [runtime inventory](../../../docs/contracts/runtime-discovery.md)
/// this Host serves reads of.
pub const DATABASE_FILE: &str = "control.sqlite3";
pub const SOCKET_FILE: &str = "host.sock";
pub const LOCK_FILE: &str = "host.lock";
pub const RUNTIME_INVENTORY_FILE: &str = "runtime-inventory.json";
/// The operator configuration discovered in the configuration directory when no flag names one.
pub const OPERATOR_CONFIG_FILE: &str = "operator.json";
/// Linux `sockaddr_un` carries 108 bytes including its terminating NUL. A state directory whose
/// socket would not fit is refused here, by name, rather than failing at `bind` with an errno.
pub const SOCKET_PATH_MAX: usize = 107;

/// Why a directory could not be resolved, with the variable or default that failed. Fields are
/// named rather than formatted into one string so a caller can say which knob to turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathError {
    /// The environment variable or flag the answer would have come from.
    pub source: &'static str,
    /// What is wrong with it, for an operator reading a log.
    pub message: String,
}

impl std::fmt::Display for PathError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.source, self.message)
    }
}

impl std::error::Error for PathError {}

/// The Host's state and configuration directories, resolved once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPaths {
    pub state: PathBuf,
    pub config: PathBuf,
}

/// Where a value came from, so a diagnostic can name the knob rather than the symptom.
fn absolute(name: &'static str, value: &str) -> Result<PathBuf, PathError> {
    let path = PathBuf::from(value);
    if value.is_empty() || !path.is_absolute() {
        return Err(PathError {
            source: name,
            message: format!("{value:?} is not an absolute path"),
        });
    }
    Ok(path)
}

/// The XDG base directory specification's rule: an empty or relative value is treated as unset.
fn xdg(lookup: &impl Fn(&str) -> Option<String>, name: &str) -> Option<String> {
    let value = lookup(name)?;
    Path::new(&value).is_absolute().then_some(value)
}

/// The default under `$HOME` when the matching `XDG_*` variable is unset or unusable.
fn home(lookup: &impl Fn(&str) -> Option<String>, rest: &str) -> Result<PathBuf, PathError> {
    let home = lookup("HOME").filter(|value| Path::new(value).is_absolute());
    let home = home.ok_or(PathError {
        source: "HOME",
        message: format!(
            "no absolute HOME and no usable XDG base directory: the Host will not guess {rest}"
        ),
    })?;
    Ok(PathBuf::from(home).join(rest))
}

impl HostPaths {
    /// Resolve the Host's directories from `lookup`, an explicit `--state-dir`/`--config-dir` pair,
    /// and the environment overrides. `lookup` is a function rather than the process environment so
    /// a case can drive every shape without mutating global state.
    pub fn resolve(
        lookup: impl Fn(&str) -> Option<String>,
        explicit_state: Option<&Path>,
        explicit_config: Option<&Path>,
    ) -> Result<Self, PathError> {
        let state = match explicit_state {
            Some(path) => absolute("--state-dir", &path.to_string_lossy())?,
            None => match lookup(STATE_OVERRIDE) {
                Some(value) if value.is_empty() => HostPaths::state_from_xdg(&lookup)?,
                Some(value) => absolute(STATE_OVERRIDE, &value)?,
                None => HostPaths::state_from_xdg(&lookup)?,
            },
        };
        let config = match explicit_config {
            Some(path) => absolute("--config-dir", &path.to_string_lossy())?,
            None => match lookup(CONFIG_OVERRIDE) {
                Some(value) if value.is_empty() => HostPaths::config_from_xdg(&lookup)?,
                Some(value) => absolute(CONFIG_OVERRIDE, &value)?,
                None => HostPaths::config_from_xdg(&lookup)?,
            },
        };
        let paths = Self { state, config };
        paths.check_socket_fits()?;
        Ok(paths)
    }

    fn state_from_xdg(lookup: &impl Fn(&str) -> Option<String>) -> Result<PathBuf, PathError> {
        Ok(match xdg(lookup, "XDG_STATE_HOME") {
            Some(base) => PathBuf::from(base).join(APPLICATION),
            None => home(lookup, ".local/state")?.join(APPLICATION),
        })
    }

    fn config_from_xdg(lookup: &impl Fn(&str) -> Option<String>) -> Result<PathBuf, PathError> {
        Ok(match xdg(lookup, "XDG_CONFIG_HOME") {
            Some(base) => PathBuf::from(base).join(APPLICATION),
            None => home(lookup, ".config")?.join(APPLICATION),
        })
    }

    /// Resolve from this process's environment, which is what the daemon does once at startup.
    pub fn from_process(
        explicit_state: Option<&Path>,
        explicit_config: Option<&Path>,
    ) -> Result<Self, PathError> {
        Self::resolve(
            |name| std::env::var(name).ok(),
            explicit_state,
            explicit_config,
        )
    }

    pub fn socket(&self) -> PathBuf {
        self.state.join(SOCKET_FILE)
    }

    pub fn database(&self) -> PathBuf {
        self.state.join(DATABASE_FILE)
    }

    /// The private file this Host's [runtime
    /// inventory](../../../docs/contracts/runtime-discovery.md) is published into and served
    /// from. It is named here beside the other state-directory files so one owner decides the
    /// layout, not the module that happens to read it.
    pub fn runtime_inventory(&self) -> PathBuf {
        self.state.join(RUNTIME_INVENTORY_FILE)
    }

    /// The operator configuration this Host would load when no flag names one. The caller decides
    /// whether its absence is "no provisioning" or an error.
    pub fn operator_config(&self) -> PathBuf {
        self.config.join(OPERATOR_CONFIG_FILE)
    }

    fn check_socket_fits(&self) -> Result<(), PathError> {
        let socket = self.socket();
        let bytes = std::os::unix::ffi::OsStrExt::as_bytes(socket.as_os_str()).len();
        if bytes > SOCKET_PATH_MAX {
            return Err(PathError {
                source: "state directory",
                message: format!(
                    "the socket {} is {bytes} bytes, past the {SOCKET_PATH_MAX}-byte Linux socket \
                     path bound; choose a shorter state directory",
                    socket.display()
                ),
            });
        }
        Ok(())
    }
}
