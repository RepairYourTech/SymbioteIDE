//! The CLI's option and flag model: how a command line is parsed into an
//! [`Options`], and which flags each command can honor.
//!
//! Flag applicability is one decision ([`honored_flags`]) rather than an `if`
//! per flag inside each handler, so adding a command means naming its flags
//! here instead of remembering to reject them in a dispatch arm. A flag a
//! command cannot honor is a usage error that names it, never a silent no-op.
use std::path::PathBuf;

/// Exit codes, stable for scripts.
pub(crate) const EXIT_OK: i32 = 0;
pub(crate) const EXIT_USAGE: i32 = 1;
pub(crate) const EXIT_REFUSED: i32 = 2;
pub(crate) const EXIT_AUTHORIZATION_REQUIRED: i32 = 3;

/// A usage failure: the command line asked for something that cannot be
/// honored. Rendered as `symbiote: <message>` with exit code 1.
#[derive(Debug)]
pub(crate) struct Usage(pub(crate) String);

impl std::fmt::Display for Usage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Usage {}

/// The positional arguments a command's builder receives, after the command
/// name itself.
pub(crate) type Args<'a> = &'a [String];

/// The parsed command line, including the flags that decide authorization.
#[derive(Debug)]
pub(crate) struct Options {
    pub(crate) state_dir: Option<PathBuf>,
    pub(crate) command_id_override: Option<String>,
    pub(crate) policy: Option<PathBuf>,
    /// With `schema`, the directory to regenerate the fixture files into
    /// instead of printing them.
    pub(crate) write: Option<PathBuf>,
    /// With `schema`, the directory to compare against the emitted documents;
    /// reads only, so it is the non-mutating half of `--write`.
    pub(crate) check: Option<PathBuf>,
    /// `--help`/`-h`: print the command table and answer no daemon, whatever
    /// command accompanied it.
    pub(crate) help: bool,
    pub(crate) json: bool,
    pub(crate) yes: bool,
    pub(crate) command: Option<String>,
    pub(crate) args: Vec<String>,
}

/// Flags are recognized anywhere on the line — `symbiote shutdown --yes` must
/// work as naturally as the `--yes`-first form. The first positional token is
/// the command and the rest are its arguments; `--` ends flag parsing, so an
/// argument that itself starts with `--` stays expressible. An unknown flag is
/// a usage error, never silently treated as the command name or an argument.
pub(crate) fn parse_options(arguments: &[String]) -> Result<Options, Usage> {
    let mut options = Options {
        state_dir: None,
        command_id_override: None,
        policy: None,
        write: None,
        check: None,
        help: false,
        json: false,
        yes: false,
        command: None,
        args: Vec::new(),
    };
    let mut positional: Vec<String> = Vec::new();
    let mut only_positional = false;
    let mut index = 0;
    while index < arguments.len() {
        let token = arguments[index].clone();
        if only_positional {
            positional.push(token);
            index += 1;
            continue;
        }
        match token.as_str() {
            "--" => only_positional = true,
            "-h" | "--help" => options.help = true,
            "--json" => options.json = true,
            "--yes" => options.yes = true,
            "--state-dir" | "--command-id" | "--policy" | "--write" | "--check" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .cloned()
                    .ok_or_else(|| Usage(format!("{token} needs a value")))?;
                match token.as_str() {
                    "--state-dir" => options.state_dir = Some(PathBuf::from(value)),
                    "--write" => options.write = Some(PathBuf::from(value)),
                    "--check" => options.check = Some(PathBuf::from(value)),
                    "--command-id" => {
                        // An empty idempotency key is refused rather than
                        // carried: every empty-id invocation would collide with
                        // every other as a replay, and the published envelope
                        // schema requires `command_id` to be non-empty, so
                        // accepting it would let the CLI print an envelope its
                        // own contract rejects.
                        if value.is_empty() {
                            return Err(Usage(
                                "--command-id must be a non-empty idempotency key".into(),
                            ));
                        }
                        options.command_id_override = Some(value);
                    }
                    _ => options.policy = Some(PathBuf::from(value)),
                }
            }
            flag if flag.starts_with("--") => {
                return Err(Usage(format!("unknown option {flag}")));
            }
            _ => positional.push(token),
        }
        index += 1;
    }
    options.command = positional.first().cloned();
    options.args = positional.into_iter().skip(1).collect();
    Ok(options)
}

/// Which flags an invocation actually supplied. Checked as a whole so flag
/// applicability is one decision instead of an `if` per flag.
#[derive(Clone, Copy, Default)]
pub(crate) struct Flags {
    pub(crate) state_dir: bool,
    pub(crate) command_id: bool,
    pub(crate) policy: bool,
    pub(crate) write: bool,
    pub(crate) check: bool,
    pub(crate) help: bool,
    pub(crate) json: bool,
    pub(crate) yes: bool,
}

impl Flags {
    /// The supplied flags `honored` does not include, named as they are typed,
    /// in documentation order.
    pub(crate) fn unsuited_for(self, honored: Flags) -> Vec<&'static str> {
        [
            (self.state_dir && !honored.state_dir, "--state-dir"),
            (self.command_id && !honored.command_id, "--command-id"),
            (self.policy && !honored.policy, "--policy"),
            (self.write && !honored.write, "--write"),
            (self.check && !honored.check, "--check"),
            (self.json && !honored.json, "--json"),
            (self.yes && !honored.yes, "--yes"),
            // Every command grants `--help`, so this never fires; it is
            // listed so a future arm that forgot the grant is caught here
            // rather than by a silently dropped `--help`.
            (self.help && !honored.help, "--help"),
        ]
        .into_iter()
        .filter(|(supplied, _)| *supplied)
        .map(|(_, name)| name)
        .collect()
    }
}

impl Options {
    /// Which flags this invocation supplied, as [`Flags`].
    pub(crate) fn supplied(&self) -> Flags {
        Flags {
            state_dir: self.state_dir.is_some(),
            command_id: self.command_id_override.is_some(),
            policy: self.policy.is_some(),
            write: self.write.is_some(),
            check: self.check.is_some(),
            help: self.help,
            json: self.json,
            yes: self.yes,
        }
    }
}

/// The flags each command can honor. A flag outside this set is a usage error
/// that names it, never a silent no-op. Every daemon command maps a request
/// over the socket, so all five daemon-facing flags apply to each of them
/// (`--policy` is consulted only for a dangerous operation and `--yes` only
/// authorizes one, but both are valid flags on any daemon command). The local
/// commands honor only what they use: `schema` publishes and compares
/// documents, so `--write` and `--check`; `help` renders the command table, so
/// nothing. `--help`/`-h` is the one universal flag, honored by every command
/// (and by no command), because it is answered from the table alone.
pub(crate) fn honored_flags(command: &str) -> Flags {
    match command {
        "schema" => Flags {
            write: true,
            check: true,
            help: true,
            ..Flags::default()
        },
        "help" => Flags {
            help: true,
            ..Flags::default()
        },
        _ => Flags {
            state_dir: true,
            command_id: true,
            policy: true,
            help: true,
            json: true,
            yes: true,
            ..Flags::default()
        },
    }
}
