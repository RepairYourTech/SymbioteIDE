//! The Host's own directories (#180): the resolution in `symbiote_host::paths`, and the daemon
//! proving it uses what the environment says.
//!
//! Before this, `symbioted` required `--state-dir` and had no defined answer for a user service, a
//! container or a headless install — `docs/contracts/host.md` listed XDG defaults as pending. These
//! cases drive the resolution through its real surface with an explicit environment (never the test
//! process's own, so the shapes are exact and parallel), and then drive the real binary: a daemon
//! started with no directory flag binds where the environment says, discovers operator
//! configuration in the config directory, and refuses by name when nothing resolves. The two facts
//! the environment cannot answer are held here too: the socket bound is driven against the kernel's
//! own `bind`, and the published file names are read from the document that owns them.

use std::collections::BTreeMap;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use symbiote_contract_read::region;
use symbiote_host::paths::{
    CONFIG_OVERRIDE, DATABASE_FILE, HostPaths, LOCK_FILE, OPERATOR_CONFIG_FILE, PathError,
    RUNTIME_INVENTORY_FILE, SOCKET_FILE, SOCKET_PATH_MAX, STATE_OVERRIDE,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn environment(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: BTreeMap<String, String> = pairs
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    move |name: &str| map.get(name).cloned()
}

fn resolve(pairs: &[(&str, &str)]) -> Result<HostPaths, PathError> {
    HostPaths::resolve(environment(pairs), None, None)
}

#[test]
fn the_xdg_base_directories_are_the_hosts_defaults() {
    let home = resolve(&[("HOME", "/home/operator")]).unwrap();
    assert_eq!(
        home.state,
        Path::new("/home/operator/.local/state/symbiote")
    );
    assert_eq!(home.config, Path::new("/home/operator/.config/symbiote"));

    let xdg = resolve(&[
        ("HOME", "/home/operator"),
        ("XDG_STATE_HOME", "/mnt/state"),
        ("XDG_CONFIG_HOME", "/mnt/config"),
    ])
    .unwrap();
    assert_eq!(xdg.state, Path::new("/mnt/state/symbiote"));
    assert_eq!(xdg.config, Path::new("/mnt/config/symbiote"));
}

#[test]
fn the_flag_beats_the_override_which_beats_the_xdg_base_directory() {
    let layers = [
        ("HOME", "/home/operator"),
        ("XDG_STATE_HOME", "/xdg/state"),
        ("XDG_CONFIG_HOME", "/xdg/config"),
        (STATE_OVERRIDE, "/override/state"),
        (CONFIG_OVERRIDE, "/override/config"),
    ];
    let overridden = resolve(&layers).unwrap();
    assert_eq!(overridden.state, Path::new("/override/state"));
    assert_eq!(overridden.config, Path::new("/override/config"));

    let flagged = HostPaths::resolve(
        environment(&layers),
        Some(Path::new("/flag/state")),
        Some(Path::new("/flag/config")),
    )
    .unwrap();
    assert_eq!(flagged.state, Path::new("/flag/state"));
    assert_eq!(flagged.config, Path::new("/flag/config"));
}

#[test]
fn an_empty_override_means_unset_like_the_cli_policy_variable() {
    let paths = resolve(&[
        ("HOME", "/home/operator"),
        (STATE_OVERRIDE, ""),
        (CONFIG_OVERRIDE, ""),
    ])
    .unwrap();
    assert_eq!(
        paths.state,
        Path::new("/home/operator/.local/state/symbiote")
    );
    assert_eq!(paths.config, Path::new("/home/operator/.config/symbiote"));
}

#[test]
fn a_relative_xdg_base_is_ignored_and_a_relative_override_is_refused() {
    // The specification's rule: a relative XDG base is not usable, so the default applies.
    let ignored = resolve(&[
        ("HOME", "/home/operator"),
        ("XDG_STATE_HOME", "relative/state"),
        ("XDG_CONFIG_HOME", "./config"),
    ])
    .unwrap();
    assert_eq!(
        ignored.state,
        Path::new("/home/operator/.local/state/symbiote")
    );
    assert_eq!(ignored.config, Path::new("/home/operator/.config/symbiote"));

    // This Host's own override is operator intent: ignoring it would run the Host somewhere the
    // operator did not name, so it is an error rather than a silent fallback.
    for (name, pairs) in [
        (
            STATE_OVERRIDE,
            vec![("HOME", "/home/operator"), (STATE_OVERRIDE, "state")],
        ),
        (
            CONFIG_OVERRIDE,
            vec![("HOME", "/home/operator"), (CONFIG_OVERRIDE, "config")],
        ),
    ] {
        let refusal = resolve(&pairs).unwrap_err();
        assert_eq!(refusal.source, name, "{refusal:?}");
        assert!(
            refusal.message.contains("absolute"),
            "the refusal says what is wrong: {refusal:?}"
        );
    }
}

#[test]
fn a_host_with_no_home_and_no_xdg_base_refuses_and_names_what_is_missing() {
    let refusal = resolve(&[]).unwrap_err();
    assert_eq!(refusal.source, "HOME", "{refusal:?}");
    assert!(
        refusal.message.contains("XDG"),
        "the refusal names the default that could not be used: {refusal:?}"
    );

    // A relative HOME is not a place to write either.
    assert!(resolve(&[("HOME", "home/operator")]).is_err());
}

/// The published layout: `docs/contracts/host.md`'s table of what lives under each resolved
/// directory, read between structural anchors so a reworded sentence is not a contract move. Each
/// row is the role, the directory, and the file name.
fn published_layout(document: &str) -> Vec<(String, String, String)> {
    let table = region(document, "| Role | Directory | File name |", "\n\n");
    let mut rows = Vec::new();
    for line in table.lines() {
        let cells: Vec<String> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().trim_matches('`').to_string())
            .collect();
        if cells.len() == 3 && !cells[0].is_empty() && !cells[0].starts_with("---") {
            rows.push((cells[0].clone(), cells[1].clone(), cells[2].clone()));
        }
    }
    rows
}

/// A real private state directory whose socket path is exactly `length` bytes, built from nested
/// one-letter components so a length no single name reaches is still reachable. This is what lets
/// the bound be driven at the platform's edge: `bind` is the oracle, not the crate's numeral.
fn state_directory_with_socket_path(length: usize) -> PathBuf {
    let mut path = std::env::temp_dir().join(format!(
        "symbiote-paths-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut left = length - path.to_string_lossy().len() - 1 - SOCKET_FILE.len();
    assert!(
        left >= 2,
        "a {length}-byte socket path needs {left} bytes of directory under {path:?}"
    );
    while left > 0 {
        let take = (left - 1).min(200);
        path.push("d".repeat(take));
        left -= take + 1;
    }
    std::fs::create_dir_all(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(path.join(SOCKET_FILE).to_string_lossy().len(), length);
    path
}

#[test]
fn a_state_directory_whose_socket_would_not_fit_is_refused_before_bind() {
    // The oracle is the kernel, not this crate's numeral: the longest socket path the Host accepts
    // is one `bind` accepts, and one byte past it is one `bind` refuses. A moved `SOCKET_PATH_MAX`
    // therefore reds here in either direction — widened, the Host would accept a socket path the
    // kernel rejects; narrowed, it would refuse one the kernel binds.
    let at = state_directory_with_socket_path(SOCKET_PATH_MAX);
    let accepted = resolve(&[
        (STATE_OVERRIDE, &at.to_string_lossy()),
        ("HOME", "/home/operator"),
    ])
    .unwrap();
    let listener = UnixListener::bind(accepted.socket())
        .expect("the kernel binds the longest socket path the Host accepts");
    drop(listener);
    let _ = std::fs::remove_file(accepted.socket());

    let past = state_directory_with_socket_path(SOCKET_PATH_MAX + 1);
    let refusal = resolve(&[
        (STATE_OVERRIDE, &past.to_string_lossy()),
        ("HOME", "/home/operator"),
    ])
    .unwrap_err();
    assert_eq!(refusal.source, "state directory", "{refusal:?}");
    assert!(
        refusal.message.contains(&SOCKET_PATH_MAX.to_string()),
        "the refusal states the bound: {refusal:?}"
    );
    assert!(
        UnixListener::bind(past.join(SOCKET_FILE)).is_err(),
        "the kernel refuses one byte past the bound, so the Host refuses where the platform does"
    );

    for directory in [at, past] {
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[test]
fn the_socket_database_and_operator_config_are_named_from_the_resolved_directories() {
    // The document owns the published names and this case holds the crate's declarations to it, so
    // a file renamed in either place reds here by name; the joins below drive where each one lands.
    let document = include_str!("../../../docs/contracts/host.md");
    let expected = [
        ("Control socket", "state directory", SOCKET_FILE),
        ("Canonical store", "state directory", DATABASE_FILE),
        ("Operator lock", "state directory", LOCK_FILE),
        (
            "Runtime inventory",
            "state directory",
            RUNTIME_INVENTORY_FILE,
        ),
        (
            "Operator configuration",
            "configuration directory",
            OPERATOR_CONFIG_FILE,
        ),
    ]
    .map(|(role, directory, name)| (role.to_string(), directory.to_string(), name.to_string()));
    assert_eq!(
        published_layout(document),
        expected,
        "the document publishes the layout this crate declares"
    );

    let paths = resolve(&[
        ("HOME", "/home/operator"),
        ("XDG_STATE_HOME", "/srv/state"),
        ("XDG_CONFIG_HOME", "/srv/config"),
    ])
    .unwrap();
    assert_eq!(paths.state, Path::new("/srv/state/symbiote"));
    assert_eq!(paths.config, Path::new("/srv/config/symbiote"));
    assert_eq!(paths.socket(), paths.state.join(SOCKET_FILE));
    assert_eq!(paths.database(), paths.state.join(DATABASE_FILE));
    assert_eq!(
        paths.runtime_inventory(),
        paths.state.join(RUNTIME_INVENTORY_FILE)
    );
    assert_eq!(
        paths.operator_config(),
        paths.config.join(OPERATOR_CONFIG_FILE)
    );
}

/// A directory that is private to this user, so the daemon's own refusal cannot be what fails.
fn private_directory(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "symbiote-paths-{}-{}-{name}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}

/// Starts the real daemon with exactly `environment`, and waits for its socket.
fn daemon(state: &Path, environment: &[(&str, &str)]) -> (Child, PathBuf) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_symbioted"));
    command.env_clear();
    for (name, value) in environment {
        command.env(name, value);
    }
    let child = command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let socket = state.join(SOCKET_FILE);
    let until = Instant::now() + Duration::from_secs(10);
    while !socket.exists() {
        assert!(Instant::now() < until, "the daemon did not bind {socket:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
    (child, socket)
}

fn health(state: &Path) -> bool {
    Command::new(env!("CARGO_BIN_EXE_symbiote"))
        .arg("--state-dir")
        .arg(state)
        .arg("health")
        .output()
        .unwrap()
        .status
        .success()
}

#[test]
fn the_daemon_binds_where_the_environment_says_without_a_flag() {
    let state = private_directory("env-state");
    let home = private_directory("home");
    let (mut child, _) = daemon(
        &state,
        &[
            (STATE_OVERRIDE, &state.to_string_lossy()),
            ("HOME", &home.to_string_lossy()),
        ],
    );
    // The socket binds before the database opens, so wait for the store rather than assume it.
    let until = Instant::now() + Duration::from_secs(10);
    while !state.join(DATABASE_FILE).exists() {
        assert!(Instant::now() < until, "the daemon never opened its store");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(health(&state), "the CLI reached the daemon it did not name");
    // The XDG default is untouched: the override is what placed it.
    assert!(!home.join(".local/state/symbiote").exists());

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn the_daemon_discovers_operator_configuration_in_the_config_directory() {
    let state = private_directory("discovered-state");
    let config = private_directory("discovered-config");
    let reservation = private_directory("reservations");
    let file = config.join(OPERATOR_CONFIG_FILE);
    let mut document = std::fs::File::create(&file).unwrap();
    document
        .write_all(
            serde_json::json!({"reservation_base": reservation.to_string_lossy()})
                .to_string()
                .as_bytes(),
        )
        .unwrap();
    document
        .set_permissions(std::fs::Permissions::from_mode(0o600))
        .unwrap();
    drop(document);

    // Its diagnostics go to a file rather than a pipe, so the wait below cannot deadlock on a
    // full buffer and the daemon's own readiness line can be read as it arrives.
    let log = state.join("diagnostics.log");
    let mut command = Command::new(env!("CARGO_BIN_EXE_symbioted"));
    command
        .env_clear()
        .env(STATE_OVERRIDE, &state)
        .env(CONFIG_OVERRIDE, &config)
        .env("HOME", private_directory("discovered-home"))
        .stdout(Stdio::null())
        .stderr(Stdio::from(std::fs::File::create(&log).unwrap()));
    let mut child = command.spawn().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let diagnostics = std::fs::read_to_string(&log).unwrap_or_default();
        if diagnostics.contains("operator provisioning active") {
            break;
        }
        assert!(
            child.try_wait().unwrap().is_none(),
            "the daemon exited: {diagnostics:?}"
        );
        assert!(
            Instant::now() < until,
            "the daemon never reported the operator configuration the config directory held: \
             {diagnostics:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn a_state_directory_the_host_cannot_create_names_its_path() {
    let missing = std::env::temp_dir().join(format!(
        "symbiote-absent-{}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    assert!(!missing.exists());
    let state = missing.join("child");
    let home = private_directory("absent-home");
    let output = Command::new(env!("CARGO_BIN_EXE_symbioted"))
        .env_clear()
        .env(STATE_OVERRIDE, &state)
        .env("HOME", &home)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let diagnostics = String::from_utf8_lossy(&output.stderr);
    assert!(
        diagnostics.contains(&state.to_string_lossy().to_string()),
        "the refusal names the directory it could not create: {diagnostics:?}"
    );
}

#[test]
fn a_daemon_with_nothing_to_resolve_exits_with_the_reason() {
    let output = Command::new(env!("CARGO_BIN_EXE_symbioted"))
        .env_clear()
        .output()
        .unwrap();
    assert!(!output.status.success(), "a Host with no directory refuses");
    let diagnostics = String::from_utf8_lossy(&output.stderr);
    assert!(
        diagnostics.contains("HOME") && diagnostics.contains("XDG"),
        "the refusal names what is missing rather than failing later: {diagnostics:?}"
    );
}
