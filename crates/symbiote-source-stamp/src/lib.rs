//! A build-time record of the sources a workspace binary was compiled from,
//! carried by that binary, and the check that the binary still matches them.
//!
//! The end-to-end proofs drive workspace binaries, so a binary that does not
//! contain the sources under test would let a proof stay green while running
//! other code. Comparing timestamps answers "was something touched after the
//! build", which misses a real change whose timestamp is older than the binary
//! (a restored file, a checkout with preserved times, a clock skew) and
//! refuses a binary whose sources were only touched.
//!
//! This crate records the **content** instead and puts the record in the
//! binary, so no record can be read that describes a build other than the one
//! that produced the binary:
//!
//! * [`build_stamp`] is called from a `build.rs`. For the package and every
//!   workspace package it reaches through normal and build dependency edges it
//!   records every file the package's binaries compile — everything under the
//!   package that is not a test, example or bench target, whatever its
//!   extension, because a compile can read a file no manifest mentions
//!   (`include_str!("schema.sql")`) — together with the manifests and build
//!   scripts that name them and the workspace manifest and `Cargo.lock` that
//!   configure and pin each package's build, a path dependency in another
//!   workspace bringing that workspace's root manifest as well, and every file
//!   those sources pull
//!   in through an `include!`, `include_bytes!` or `include_str!` or a
//!   `#[path = "…"]` module attribute, wherever it lives:
//!   `symbiote-workflow` compiles fixtures kept at the workspace root.
//!   It hashes the content of all of them, tells cargo to rerun the build
//!   script when any of them changes, and writes the record into `OUT_DIR` as
//!   a `static`, which the binary includes so the record travels inside the
//!   binary itself.
//! * [`changed_sources`] is called by the proof. It reads the record out of
//!   the binary's own bytes, re-reads every input the record holds, and
//!   refuses a binary that carries no record — one built without the stamp —
//!   naming every input whose content differs. An input is a file, or a value
//!   of the build's environment ([`Input`]).
//!
//! What a compile produces depends on more than the source files, so the
//! record holds the other two kinds of input that can change it while every
//! file stays byte-identical: the configuration files cargo reads for a build
//! of any closure package — `.cargo/config.toml`, `.cargo/config`,
//! `rust-toolchain.toml`, `rust-toolchain`, in each package directory and
//! workspace root and every directory above them, plus the ones beside the
//! registry cache — and the one environment variable both a build script and a
//! proof see alike, `RECORDED_ENVIRONMENT`, which names the toolchain. The flags
//! a build was given are covered through the files that carry them rather than
//! through `RUSTFLAGS`, which the two sides do not see alike; the constant's own
//! documentation says what is left outside and why.
//!
//! What the walk covers is stated rather than claimed whole. Test, example and
//! bench targets are outside it because nothing a binary compiles comes from
//! them, so a change there must not refuse a current binary. Registry
//! dependencies are outside it because `Cargo.lock`, which is inside it, pins
//! them, and dev-dependency edges are outside it because a binary compiles
//! none of them. A configuration file that did not exist when the binary was
//! built and appears afterwards is outside it: cargo reads no such file at
//! build time, and naming the absent location as a build-script input would
//! list a missing path, which makes cargo recompile every stamped crate on
//! every build (measured: `Dirty symbiote-host: the file ... is missing`, and
//! the binary relinked each time). A `rust-toolchain.toml` that appears is
//! still caught, because the toolchain it selects is named by the recorded
//! cargo. Files a package generates into its `OUT_DIR` are outside it
//! too: their content comes from the build script, which is inside it. The
//! include scan reads code, not text — a path named in a comment or a string
//! literal is not an input — and follows a literal path and a `concat!` of
//! literals and `env!("CARGO_MANIFEST_DIR")`, which is how a package names an
//! input it does not keep beside itself. It follows those to a fixed point
//! over Rust sources, so a non-Rust file pulled in by `include!` is recorded
//! but not itself scanned for includes. A `#[path = "…"]` module attribute is
//! followed the same way when it sits on a `mod name;` declaration, resolved
//! against the directory the compiler looks in: the file's own directory at the
//! top level of a file, and a directory inside it when the declaration sits in
//! an inline module block. A file does not say whether the compiler found it as
//! `mod name;` — which puts its modules under a directory of its own name — or
//! named it through a `#[path]`, as a crate root or as a `mod.rs`, which do not,
//! so inside an inline module the record keeps both directories rather than
//! choose: a record that is long is a rebuild, and a record that is short is
//! the failure this crate exists to prevent. A path built any other way, and a
//! `#[path]` on an inline module, is **not skipped**: the build stops and names
//! it.
//!
//! Cargo's dep-info is not read here because it is a private, versioned binary
//! format that is written *after* the build script that must write the record,
//! so it can neither populate a record on a first build nor be a completeness
//! check the documented rebuild could ever clear.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// The record as it is framed inside the binary: [`RECORD_START`], the record
/// itself, then [`RECORD_END`]. The check reads the bytes between the two, so
/// what it verifies is what that binary's own build put there.
pub const RECORD_START: &str = "symbiote-source-record:[";
/// See [`RECORD_START`].
pub const RECORD_END: &str = "]symbiote-source-record";

/// The file `build_stamp` writes into `OUT_DIR`, for the binary to `include!`.
pub const RECORD_FILE: &str = "source_record.rs";

/// The `static` that file defines, holding [`RECORD_START`]..[`RECORD_END`].
pub const RECORD_STATIC: &str = "SOURCE_RECORD";

/// The environment variables a record holds: the cargo that ran the build,
/// which names the toolchain a `rust-toolchain.toml` selects. It is set for a
/// build script and for a test process alike — and to the same value whether a
/// build is started through the rustup shim or through that toolchain's own
/// binary — so the record can be checked against the run rather than only
/// described.
///
/// Two neighbours are deliberately not here:
///
/// * `RUSTFLAGS`, because the two sides never see the same value. Measured,
///   cargo passes it to a test process untouched
///   (`TEST RUSTFLAGS=Some("--cfg from_shell")`) while a build script gets it
///   stripped (`BUILD RUSTFLAGS=None`) and sees only the effective flags in
///   `CARGO_ENCODED_RUSTFLAGS` (`Some("--cfg\u{1f}from_shell")`). Recording
///   what the build saw would refuse a current binary whenever a flag is set;
///   recording what the run sees would refuse one built from a
///   `.cargo/config`. The flags a build is given are covered through the
///   configuration files that carry them.
/// * `RUSTUP_TOOLCHAIN`, because it reports how cargo was invoked rather than
///   which compiler ran: a build through the rustup shim sets it and an
///   invocation of that toolchain's own cargo does not, which would make the
///   two refuse each other alternately. The toolchain is named by `CARGO`'s
///   path, which does not move.
const RECORDED_ENVIRONMENT: [&str; 1] = ["CARGO"];

/// How an environment variable is spelled as the locator of an input, so a
/// record can hold a value beside the files it holds.
const ENVIRONMENT: &str = "env:";

/// The content of an input that is not there: a variable that is not set, or a
/// recorded file that has gone.
const UNSET: &str = "unset";

/// The configuration files cargo reads for a build of any closure package,
/// relative to the directory it is looked for in.
const CONFIGURATION_FILES: [&str; 4] = [
    ".cargo/config.toml",
    ".cargo/config",
    "rust-toolchain.toml",
    "rust-toolchain",
];

/// One input a record holds, as [`changed_sources`] reports it: a file the
/// build compiled, or a value of the environment it compiled under.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Input {
    /// A file, named as the record names it: relative to the workspace, or
    /// absolute when it lives outside it.
    File(PathBuf),
    /// An environment variable, named without the `env:` prefix the record
    /// spells it with.
    Environment(String),
}

impl std::fmt::Display for Input {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Input::File(path) => write!(formatter, "{}", path.display()),
            Input::Environment(name) => write!(formatter, "{ENVIRONMENT}{name}"),
        }
    }
}

/// Directories under a package that its binaries do not compile. The targets
/// cargo builds for tests, examples and benches are excluded because a change
/// there must not refuse a current binary — no rebuild could clear that
/// refusal, since a test file is not an input to the binary's build — and the
/// rest is build output, dependency cache and version-control state.
const UNCOMPILED_DIRECTORIES: [&str; 7] = [
    ".git",
    "benches",
    "dist",
    "examples",
    "node_modules",
    "target",
    "tests",
];

/// The macros through which a source pulls in another file at compile time.
/// The scan compares identifiers, so the `!` that makes each one a macro is
/// not part of the name it looks for.
const INCLUDE_MACROS: [&str; 3] = ["include", "include_bytes", "include_str"];

/// Records the content of the inputs this package's binaries are built from —
/// its sources, the files they compile, the manifests and configuration that
/// describe and shape the build, and the environment it runs under — as a
/// `static` in `OUT_DIR`, for the binary to include, and asks cargo to rerun
/// the build script when any of them changes. Call this from a build script's
/// `main`.
pub fn build_stamp() {
    let manifest_dir = PathBuf::from(environment("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(environment("OUT_DIR"));
    let workspace = workspace_root(&manifest_dir)
        .unwrap_or_else(|problem| panic!("cannot record the sources under test: {problem}"));

    let (sources, unfollowed) = recorded_sources(&manifest_dir, &workspace);
    if let Some(problem) = unfollowed.first() {
        panic!(
            "cannot record the sources under test: {problem}. A record that cannot name every \
             input a binary compiles would let a proof pass against a binary built from other \
             sources, so this build stops here."
        );
    }

    let mut record = String::new();
    for source in &sources {
        let content = std::fs::read(source)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", source.display()));
        record.push_str(&format!(
            "{:x}\t{}\n",
            Sha256::digest(&content),
            relative_to(&workspace, source).display()
        ));
        println!("cargo:rerun-if-changed={}", source.display());
    }
    // The environment this compile runs under, held as an input of its own:
    // the cargo that ran it, which names the toolchain. Cargo re-runs this
    // build script when it changes, so the record describes the toolchain in
    // use rather than the one that built it first.
    for name in RECORDED_ENVIRONMENT {
        record.push_str(&format!(
            "{}\t{ENVIRONMENT}{name}\n",
            environment_content(name)
        ));
        println!("cargo:rerun-if-env-changed={name}");
    }

    let generated = out_dir.join(RECORD_FILE);
    std::fs::write(&generated, record_static(&record)).unwrap_or_else(|error| {
        panic!(
            "cannot write the source record at {}: {error}",
            generated.display()
        )
    });
}

/// The record as the Rust source of the `static` a binary includes. The record
/// is spelled as a string literal by `Debug`, which is defined to be a literal
/// with the same value, so a path holding a quote or a backslash survives.
fn record_static(record: &str) -> String {
    format!(
        "// Generated by symbiote-source-stamp: the content this binary was compiled from.\n\
         #[doc(hidden)]\n\
         static {RECORD_STATIC}: &str = {:?};\n",
        format!("{RECORD_START}{record}{RECORD_END}")
    )
}

/// Every file whose recorded content no longer matches what is on disk, or an
/// error naming why the binary at `binary` cannot be checked at all. An empty
/// list means the binary carries a record of exactly the content the tree
/// holds.
pub fn changed_sources(binary: &Path, manifest_dir: &Path) -> Result<Vec<Input>, String> {
    let bytes = std::fs::read(binary)
        .map_err(|error| format!("cannot read the binary at {} ({error})", binary.display()))?;
    let record = embedded_record(&bytes).ok_or_else(|| {
        format!(
            "the binary at {} carries no build record, so a build of the sources under test did \
             not produce it",
            binary.display()
        )
    })?;

    let workspace = workspace_root(manifest_dir)?;
    let recorded = read_record(&record)?;
    if recorded.is_empty() {
        return Err(format!(
            "the binary at {} carries an empty build record, which names no input to check",
            binary.display()
        ));
    }
    Ok(recorded
        .iter()
        .filter_map(|(locator, hash)| {
            let (input, content) = input_at(locator, &workspace);
            (content != *hash).then_some(input)
        })
        .collect())
}

/// The input a locator names — with a relative file locator resolved against
/// the workspace — and the content it holds now, spelled as a record spells
/// content: a file's sha256, an environment variable's, or [`UNSET`] when it
/// is not there.
fn input_at(locator: &str, workspace: &Path) -> (Input, String) {
    match locator.strip_prefix(ENVIRONMENT) {
        Some(name) => (
            Input::Environment(name.to_owned()),
            environment_content(name),
        ),
        None => {
            let path = workspace.join(locator);
            let content = content_hash(&path).unwrap_or_else(|| UNSET.to_owned());
            (Input::File(path), content)
        }
    }
}

/// The content of an environment variable: its value's sha256, or [`UNSET`]
/// when it is not set at all, which is a different fact from set and empty.
fn environment_content(name: &str) -> String {
    std::env::var(name)
        .map(|value| format!("{:x}", Sha256::digest(value.as_bytes())))
        .unwrap_or_else(|_| UNSET.to_owned())
}

/// The sha256 of every input the record holds, keyed by the locator it is
/// spelled with — a path, or `env:NAME` for an environment variable. A line
/// without both halves is reported rather than skipped: a record that lost one
/// is not a shorter record but an unreadable one.
fn read_record(record: &str) -> Result<BTreeMap<String, String>, String> {
    let mut recorded = BTreeMap::new();
    for line in record.lines().filter(|line| !line.is_empty()) {
        let Some((hash, locator)) = line.split_once('\t') else {
            return Err("the source record has a malformed line".to_owned());
        };
        recorded.insert(locator.to_owned(), hash.to_owned());
    }
    Ok(recorded)
}

/// The record a binary carries: the bytes between the markers `build_stamp`
/// wrote into the `static` the binary includes. `None` for a binary built
/// without the stamp.
fn embedded_record(binary: &[u8]) -> Option<String> {
    let start = find(binary, RECORD_START.as_bytes())? + RECORD_START.len();
    let end = start + find(&binary[start..], RECORD_END.as_bytes())?;
    std::str::from_utf8(&binary[start..end])
        .ok()
        .map(str::to_owned)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Every file the build of the package at `manifest_dir` reads — the walk over
/// each closure package's directory, every file those sources include or
/// declare as a module from wherever it lives, the workspace manifest and
/// lockfile of each closure package's own workspace, which sit above every
/// package directory, and the configuration files cargo reads for the build —
/// together with the include sites the scan cannot follow, each one a reason
/// this build must stop. Followed to a fixed point over the Rust sources, so an included `.rs`
/// file that includes another is recorded too, and a non-Rust file pulled in by
/// `include!` is recorded but not scanned for includes of its own.
fn recorded_sources(manifest_dir: &Path, workspace: &Path) -> (BTreeSet<PathBuf>, Vec<String>) {
    let packages = closure_directories(manifest_dir);
    let mut sources = BTreeSet::new();
    for directory in &packages {
        sources.extend(package_sources(directory));
    }

    let mut scanned = BTreeSet::new();
    let mut unfollowed = Vec::new();
    loop {
        let pending: Vec<PathBuf> = sources
            .iter()
            .filter(|source| is_rust(source) && !scanned.contains(*source))
            .cloned()
            .collect();
        if pending.is_empty() {
            break;
        }
        for source in pending {
            scanned.insert(source.clone());
            match followed_inputs(&source, &packages) {
                Ok(included) => {
                    for file in included {
                        let file = canonical(&file);
                        if file.is_file() {
                            sources.insert(file);
                        }
                    }
                }
                Err(problem) => unfollowed.push(problem),
            }
        }
    }

    // A workspace manifest and its lockfile sit above every package of that
    // workspace, so the walk over package directories reaches neither. The
    // lockfile pins the registry dependencies; the manifest configures the
    // packages — a member says `edition.workspace = true`, which makes the
    // edition its code is compiled under a property of a file kept outside it.
    // Every closure package is asked, not only the one being built: a path
    // dependency may live in a workspace of its own, and then *that* root
    // manifest is the one its edition and version come from.
    let mut roots = BTreeSet::from([workspace.to_path_buf()]);
    for package in &packages {
        if let Ok(root) = workspace_root(package) {
            roots.insert(root);
        }
    }
    for root in roots {
        for shared in ["Cargo.toml", "Cargo.lock"] {
            let file = root.join(shared);
            if file.is_file() {
                sources.insert(file);
            }
        }
    }

    // The files cargo reads to configure a build rather than to find one: a
    // `rustflags` entry in a `.cargo/config.toml`, or the channel a
    // `rust-toolchain.toml` selects, changes what the compiler produces while
    // every source file stays byte-identical. Only those that exist are
    // recorded — the module documentation says why the absent ones are not.
    sources.extend(configuration_files(&packages, workspace));
    (sources, unfollowed)
}

/// The configuration files cargo reads for a build of a package in this
/// workspace: each of [`CONFIGURATION_FILES`] in every closure package's own
/// directory and in the workspace root, in every directory above those, and
/// beside the registry cache — whichever of them exist.
///
/// Above a package, because cargo reads configuration from its working
/// directory upwards whichever directory a build was started in; and beside
/// `CARGO_HOME`, because that is where a machine-wide configuration lives.
fn configuration_files(packages: &[PathBuf], workspace: &Path) -> Vec<PathBuf> {
    let mut directories: BTreeSet<PathBuf> = BTreeSet::new();
    for start in packages.iter().map(PathBuf::as_path).chain([workspace]) {
        let mut directory = Some(canonical(start));
        while let Some(candidate) = directory {
            directories.insert(candidate.clone());
            directory = candidate.parent().map(Path::to_path_buf);
        }
    }
    if let Some(cache) = std::env::var_os("CARGO_HOME") {
        directories.insert(PathBuf::from(cache));
    }
    directories
        .into_iter()
        .flat_map(|directory| {
            CONFIGURATION_FILES
                .iter()
                .map(move |name| directory.join(name))
        })
        .filter(|file| file.is_file())
        .map(|file| canonical(&file))
        .collect()
}

/// The inputs one source pulls in — the files its include macros name and the
/// module sources its `#[path]` attributes name — or the reason this scan
/// cannot follow one of them. A literal path is resolved the way the compiler
/// resolves it — relative to the file that names it — and a `concat!` of
/// literals and `env!("CARGO_MANIFEST_DIR")` is resolved against the package
/// the file belongs to. An input the compiler takes from `OUT_DIR` is left to
/// the build script that wrote it, which is itself recorded.
fn followed_inputs(source: &Path, packages: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let Ok(text) = std::fs::read_to_string(source) else {
        return Ok(Vec::new());
    };
    let Some(directory) = source.parent() else {
        return Ok(Vec::new());
    };
    let mut inputs = Vec::new();
    for include in include_arguments(&text) {
        match include {
            Include::Generated => {}
            Include::Literal(path) => inputs.push(directory.join(path)),
            Include::Parts(parts) => {
                let mut joined = String::new();
                for part in parts {
                    match part {
                        Part::Text(text) => joined.push_str(&text),
                        Part::ManifestDir => {
                            let package = enclosing_package(source, packages).ok_or_else(|| {
                                format!(
                                    "{} is not inside a workspace package, so its \
                                     env!(\"CARGO_MANIFEST_DIR\") cannot be resolved",
                                    source.display()
                                )
                            })?;
                            joined.push_str(&package.to_string_lossy());
                        }
                        Part::OutDir | Part::Variable => {
                            unreachable!(
                                "evaluate leaves OUT_DIR and unknown variables out of Parts"
                            )
                        }
                    }
                }
                let path = PathBuf::from(joined);
                inputs.push(if path.is_absolute() {
                    path
                } else {
                    directory.join(path)
                });
            }
            Include::Unknown(snippet) => {
                return Err(format!(
                    "{} names an input through a path it builds while compiling, which the source \
                     record cannot follow: {snippet}. Write the path as a literal, or as a \
                     concat! of literals and env!(\"CARGO_MANIFEST_DIR\"), so the record covers \
                     every input the binary compiles",
                    source.display()
                ));
            }
        }
    }
    for module in module_paths(&text) {
        match module {
            ModulePath::File { path, inline } => {
                inputs.extend(module_inputs(source, &path, &inline));
            }
            ModulePath::Inline { path } => {
                return Err(format!(
                    "{} attaches a `#[path = {path:?}]` to an inline module, which moves the \
                     directory that module's children are found in — including children this \
                     scan does not model — so the source record cannot follow it. Declare the \
                     module in its own file, where the record covers it",
                    source.display()
                ));
            }
            ModulePath::Unreadable(snippet) => {
                return Err(format!(
                    "{} names a module source through a `#[path]` this scan cannot read: \
                     {snippet}. Write it as a literal `#[path = \"…\"]` attribute directly on a \
                     `mod name;` declaration, so the record covers every input the binary \
                     compiles",
                    source.display()
                ));
            }
        }
    }
    Ok(inputs)
}

/// A module source a `#[path]` attribute names, as far as the scan can follow
/// it.
#[derive(Debug)]
enum ModulePath {
    /// `#[path = "…"] mod name;`, the file the module's own source lives in.
    File { path: String, inline: Vec<String> },
    /// `#[path = "…"] mod name { … }`, which moves the directory the module's
    /// children are found in.
    Inline { path: String },
    /// A `#[path]` this scan cannot read, spelled for the refusal.
    Unreadable(String),
}

/// The module sources a source's `#[path]` attributes name, each with the
/// inline modules it sits inside. Read from the code, so a `#[path]` a comment
/// or a string literal names is not an input, and every enclosing inline module
/// is tracked because it moves the directory the attribute resolves against.
fn module_paths(source: &str) -> Vec<ModulePath> {
    let tokens = tokens(source);
    let mut paths = Vec::new();
    // One entry per open brace: the inline module it opens, when it opens one.
    let mut frames: Vec<Option<String>> = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        match &tokens[index] {
            Token::Punct('{') => {
                frames.push(opened_module(&tokens, index));
                index += 1;
            }
            Token::Punct('}') => {
                frames.pop();
                index += 1;
            }
            Token::Punct('#') => match attribute(&tokens, index) {
                Some(Attribute::Path { value, next }) => {
                    paths.push(declaration(&tokens, next, value, &enclosing(&frames)));
                    index = next;
                }
                Some(Attribute::Other { next }) => index = next,
                None => index += 1,
            },
            _ => index += 1,
        }
    }
    paths
}

/// The inline modules a scan is inside, outermost first.
fn enclosing(frames: &[Option<String>]) -> Vec<String> {
    frames.iter().flatten().cloned().collect()
}

/// The inline module whose body the `{` at `index` opens, or `None` for a brace
/// that closes over a function body, a block or anything else.
fn opened_module(tokens: &[Token], index: usize) -> Option<String> {
    match tokens.get(..index) {
        Some([.., Token::Ident(keyword), Token::Ident(name)]) if keyword == "mod" => {
            Some(module_name(name).to_owned())
        }
        _ => None,
    }
}

/// The name a module declaration gives, without the `r#` of a raw identifier,
/// which is the name the compiler spells its directory with.
fn module_name(identifier: &str) -> &str {
    identifier.strip_prefix("r#").unwrap_or(identifier)
}

/// One `#[…]` attribute group.
enum Attribute {
    /// `#[path = …]`, with the value it names and the index after its `]`.
    Path {
        value: Result<String, String>,
        next: usize,
    },
    /// Any other attribute group, with the index after it.
    Other { next: usize },
}

/// The attribute group the `#` at `index` opens, or `None` when it opens none.
/// A `path` inside another attribute (`#[cfg_attr(unix, path = "…")]`) names an
/// input this scan cannot decide, so it is read as a value it cannot follow
/// rather than passed over.
fn attribute(tokens: &[Token], index: usize) -> Option<Attribute> {
    let mut cursor = index + 1;
    if matches!(tokens.get(cursor), Some(Token::Punct('!'))) {
        cursor += 1;
    }
    if !matches!(tokens.get(cursor), Some(Token::Punct('['))) {
        return None;
    }
    let end = matching_bracket(tokens, cursor)?;
    let content = &tokens[cursor + 1..end];
    let next = end + 1;
    Some(match content {
        [Token::Ident(name), Token::Punct('='), rest @ ..] if name == "path" => Attribute::Path {
            value: match rest {
                [Token::Literal(path)] => Ok(path.clone()),
                _ => Err(render(rest)),
            },
            next,
        },
        _ if names_a_path(content) => Attribute::Path {
            value: Err(render(content)),
            next,
        },
        _ => Attribute::Other { next },
    })
}

/// Whether an attribute's content assigns `path`, which only a conditional
/// attribute does without `path` being the attribute itself.
fn names_a_path(content: &[Token]) -> bool {
    content
        .windows(2)
        .any(|pair| matches!(pair, [Token::Ident(name), Token::Punct('=')] if name == "path"))
}

/// The index of the bracket closing the one at `index`.
fn matching_bracket(tokens: &[Token], index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, token) in tokens[index..].iter().enumerate() {
        match token {
            Token::Punct('[' | '(' | '{') => depth += 1,
            Token::Punct(']' | ')' | '}') => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index + offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// The module a `#[path]` attribute declares: the file the module's source
/// lives in, an inline module, or the reason this scan cannot read it. The
/// attributes and the `pub` visibility between the attribute and the item are
/// skipped, and an item that is not a `mod` declaration is refused rather than
/// passed over, because a `#[path]` this scan walks past could name an input it
/// never records.
fn declaration(
    tokens: &[Token],
    mut index: usize,
    value: Result<String, String>,
    inline: &[String],
) -> ModulePath {
    while matches!(tokens.get(index), Some(Token::Punct('#'))) {
        match attribute(tokens, index) {
            Some(Attribute::Other { next }) => index = next,
            _ => break,
        }
    }
    if matches!(tokens.get(index..), Some([Token::Ident(word), ..]) if word == "pub") {
        index += 1;
        if matches!(tokens.get(index), Some(Token::Punct('('))) {
            index = matching_bracket(tokens, index).map_or(index, |end| end + 1);
        }
    }
    let path = match value {
        Ok(path) => path,
        Err(snippet) => return ModulePath::Unreadable(format!("#[path = {snippet}]")),
    };
    match module_declaration(tokens, index) {
        Some(false) => ModulePath::File {
            path,
            inline: inline.to_vec(),
        },
        Some(true) => ModulePath::Inline { path },
        None => ModulePath::Unreadable(format!(
            "#[path = {path:?}] on an item that is not a module declaration"
        )),
    }
}

/// Whether a module declaration starts at `index`, and whether it is an inline
/// module (`mod name {`) rather than one with a file of its own (`mod name;`).
fn module_declaration(tokens: &[Token], index: usize) -> Option<bool> {
    match tokens.get(index..) {
        Some(
            [
                Token::Ident(keyword),
                Token::Ident(_),
                Token::Punct(';'),
                ..,
            ],
        ) if keyword == "mod" => Some(false),
        Some(
            [
                Token::Ident(keyword),
                Token::Ident(_),
                Token::Punct('{'),
                ..,
            ],
        ) if keyword == "mod" => Some(true),
        _ => None,
    }
}

/// The files a `#[path]` module declaration compiles, from the directory the
/// compiler resolves it against. A declaration at the top level of a file
/// resolves against the file's own directory. One inside an inline module block
/// resolves against that directory with the inline modules appended, and with
/// the module's own name in front when the compiler found the file as `mod
/// name;` — which a file named by a `#[path]`, a crate root and a `mod.rs` do
/// not. A file alone does not say which it is, so both candidates are recorded:
/// one of them is the compiler's, and a record that is long is a rebuild.
fn module_inputs(source: &Path, path: &str, inline: &[String]) -> Vec<PathBuf> {
    let Some(directory) = source.parent() else {
        return Vec::new();
    };
    if inline.is_empty() {
        return vec![directory.join(path)];
    }
    let mut bases = vec![directory.to_path_buf()];
    if let Some(name) = added_module_directory(source) {
        bases.push(directory.join(name));
    }
    bases
        .into_iter()
        .map(|base| {
            inline
                .iter()
                .fold(base, |directory, name| directory.join(name))
                .join(path)
        })
        .collect()
}

/// The directory a file adds to the paths of its nested modules, or `None` when
/// it adds none: a crate root (`lib.rs`, `main.rs`) and a `mod.rs` are the
/// directory of their own module, while a file the compiler found as `mod
/// name;` keeps its modules under a directory of its own name.
fn added_module_directory(source: &Path) -> Option<&str> {
    let name = source.file_name()?.to_str()?;
    if matches!(name, "lib.rs" | "main.rs" | "mod.rs") {
        return None;
    }
    source.file_stem()?.to_str()
}

/// A path an include macro names, as far as the scan can follow it.
enum Include {
    /// A literal path, relative to the file that names it.
    Literal(String),
    /// A `concat!` of literal text and the package directory, in order.
    Parts(Vec<Part>),
    /// A path under the compiler's `OUT_DIR`, whose producer is recorded.
    Generated,
    /// A path built in a way this scan cannot follow, spelled for the refusal.
    Unknown(String),
}

/// One piece of a `concat!` an include macro names.
enum Part {
    Text(String),
    ManifestDir,
    OutDir,
    /// A path the scan cannot name, whose producer is unknown to it.
    Variable,
}

/// The include macros one source names, read from its code: comments are not
/// code and neither is a macro named inside a string literal, so neither can
/// add an input or stop a build.
fn include_arguments(source: &str) -> Vec<Include> {
    let tokens = tokens(source);
    let mut arguments = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let Token::Ident(name) = &tokens[index] else {
            index += 1;
            continue;
        };
        if !INCLUDE_MACROS.contains(&name.as_str())
            || !matches!(tokens.get(index + 1), Some(Token::Punct('!')))
            || !matches!(tokens.get(index + 2), Some(Token::Punct('(')))
        {
            index += 1;
            continue;
        }
        let mut depth = 1usize;
        let mut end = index + 3;
        while end < tokens.len() && depth > 0 {
            match tokens[end] {
                Token::Punct('(') => depth += 1,
                Token::Punct(')') => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                break;
            }
            end += 1;
        }
        let argument = tokens.get(index + 3..end).unwrap_or_default();
        arguments.push(match evaluate(argument) {
            Include::Unknown(snippet) => Include::Unknown(format!("{name}!({snippet})")),
            followed => followed,
        });
        index = end + 1;
    }
    arguments
}

/// What one include macro's argument names: a literal, a `concat!` this scan
/// can resolve, a path under `OUT_DIR`, or something it cannot follow.
fn evaluate(argument: &[Token]) -> Include {
    if let [Token::Literal(path)] = argument {
        return Include::Literal(path.clone());
    }
    let Some(parts) = concat_parts(argument) else {
        return Include::Unknown(render(argument));
    };
    if parts.iter().any(|part| matches!(part, Part::OutDir)) {
        return Include::Generated;
    }
    if parts.iter().any(|part| matches!(part, Part::Variable)) {
        return Include::Unknown(render(argument));
    }
    Include::Parts(parts)
}

/// The pieces of a `concat!(...)` argument, or `None` when the argument is not
/// a `concat!` of string literals and `env!` values.
fn concat_parts(argument: &[Token]) -> Option<Vec<Part>> {
    let [
        Token::Ident(name),
        Token::Punct('!'),
        Token::Punct('('),
        inner @ ..,
        Token::Punct(')'),
    ] = argument
    else {
        return None;
    };
    if name != "concat" {
        return None;
    }
    split_commas(inner)
        .into_iter()
        // An empty element is the remainder after a trailing or repeated comma.
        // A trailing comma is ordinary Rust, so it must not read as a path
        // this scan cannot follow.
        .filter(|element| !element.is_empty())
        .map(part)
        .collect()
}

/// One `concat!` element: a string literal, or `env!("...")`.
fn part(element: &[Token]) -> Option<Part> {
    match element {
        [Token::Literal(text)] => Some(Part::Text(text.clone())),
        [
            Token::Ident(name),
            Token::Punct('!'),
            Token::Punct('('),
            Token::Literal(variable),
            Token::Punct(')'),
        ] if name == "env" => Some(match variable.as_str() {
            "CARGO_MANIFEST_DIR" => Part::ManifestDir,
            "OUT_DIR" => Part::OutDir,
            _ => Part::Variable,
        }),
        _ => None,
    }
}

/// `tokens` split on the commas that are not inside brackets or parentheses.
fn split_commas(tokens: &[Token]) -> Vec<&[Token]> {
    let mut elements = Vec::new();
    let (mut start, mut depth) = (0, 0usize);
    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Punct('(' | '[' | '{') => depth += 1,
            Token::Punct(')' | ']' | '}') => depth = depth.saturating_sub(1),
            Token::Punct(',') if depth == 0 => {
                elements.push(&tokens[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    elements.push(&tokens[start..]);
    elements
}

/// A source spelled back as text, for a refusal a reader can act on.
fn render(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| match token {
            Token::Ident(name) => name.clone(),
            Token::Literal(value) => format!("{value:?}"),
            Token::Punct(character) => character.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// What the scan needs to tell code from text: an identifier, a string
/// literal's value, or one punctuation character. Whitespace is dropped, so an
/// argument written across lines reads the same as one written on a line;
/// comments are dropped, so a path a comment names is not an input; a literal is
/// kept as a value, so a macro named inside a string is not code; and a raw
/// identifier is one identifier, so `mod r#type` names the module `type`.
enum Token {
    Ident(String),
    Literal(String),
    Punct(char),
}

fn tokens(source: &str) -> Vec<Token> {
    let characters: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character == '/' && characters.get(index + 1) == Some(&'/') {
            while index < characters.len() && characters[index] != '\n' {
                index += 1;
            }
        } else if character == '/' && characters.get(index + 1) == Some(&'*') {
            index = after_block_comment(&characters, index);
        } else if let Some((literal, next)) = string_literal(&characters, index) {
            tokens.push(Token::Literal(literal));
            index = next;
        } else if let Some(next) = char_literal(&characters, index) {
            index = next;
        } else if character == 'r'
            && characters.get(index + 1) == Some(&'#')
            && characters
                .get(index + 2)
                .is_some_and(|next| next.is_alphanumeric() || *next == '_')
        {
            let start = index;
            index += 2;
            while index < characters.len()
                && (characters[index].is_alphanumeric() || characters[index] == '_')
            {
                index += 1;
            }
            tokens.push(Token::Ident(characters[start..index].iter().collect()));
        } else if character.is_alphanumeric() || character == '_' {
            let start = index;
            while index < characters.len()
                && (characters[index].is_alphanumeric() || characters[index] == '_')
            {
                index += 1;
            }
            tokens.push(Token::Ident(characters[start..index].iter().collect()));
        } else if character.is_whitespace() {
            index += 1;
        } else {
            tokens.push(Token::Punct(character));
            index += 1;
        }
    }
    tokens
}

/// The index after the block comment opening at `index`. Rust's block comments
/// nest, so the depth is counted rather than the first `*/` taken.
fn after_block_comment(characters: &[char], index: usize) -> usize {
    let mut index = index + 2;
    let mut depth = 1usize;
    while index < characters.len() && depth > 0 {
        if characters[index] == '/' && characters.get(index + 1) == Some(&'*') {
            depth += 1;
            index += 2;
        } else if characters[index] == '*' && characters.get(index + 1) == Some(&'/') {
            depth -= 1;
            index += 2;
        } else {
            index += 1;
        }
    }
    index
}

/// The string literal starting at `index` — plain, byte or raw — with the
/// index after it. Its escapes are resolved for the bytes a path can hold, so
/// the value the compiler would see is what the walk is given.
fn string_literal(characters: &[char], index: usize) -> Option<(String, usize)> {
    let mut cursor = index;
    if characters.get(cursor) == Some(&'b') {
        cursor += 1;
    }
    if characters.get(cursor) == Some(&'r') {
        let mut hashes = 0usize;
        cursor += 1;
        while characters.get(cursor) == Some(&'#') {
            hashes += 1;
            cursor += 1;
        }
        if characters.get(cursor) != Some(&'"') {
            return None;
        }
        cursor += 1;
        let start = cursor;
        while cursor < characters.len() {
            if characters[cursor] == '"'
                && characters
                    .get(cursor + 1..cursor + 1 + hashes)
                    .is_some_and(|closing| closing.iter().all(|character| *character == '#'))
            {
                return Some((
                    characters[start..cursor].iter().collect(),
                    cursor + 1 + hashes,
                ));
            }
            cursor += 1;
        }
        return None;
    }
    if characters.get(cursor) != Some(&'"') {
        return None;
    }
    cursor += 1;
    let mut value = String::new();
    while cursor < characters.len() {
        match characters[cursor] {
            '"' => return Some((value, cursor + 1)),
            '\\' => {
                let escaped = *characters.get(cursor + 1)?;
                cursor += 2;
                match escaped {
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    't' => value.push('\t'),
                    '0' => value.push('\0'),
                    'u' => {
                        if characters.get(cursor) == Some(&'{') {
                            cursor += 1;
                            while cursor < characters.len() && characters[cursor] != '}' {
                                cursor += 1;
                            }
                            cursor += 1;
                        }
                    }
                    other => value.push(other),
                }
            }
            character => {
                value.push(character);
                cursor += 1;
            }
        }
    }
    None
}

/// The index after the character literal starting at `index`, or `None` for a
/// lifetime (which is an identifier's apostrophe, not a literal).
fn char_literal(characters: &[char], index: usize) -> Option<usize> {
    let mut cursor = index;
    if characters.get(cursor) == Some(&'b') {
        cursor += 1;
    }
    if characters.get(cursor) != Some(&'\'') {
        return None;
    }
    cursor += 1;
    if characters.get(cursor) == Some(&'\\') {
        cursor += 1;
        match characters.get(cursor)? {
            'x' => cursor += 3,
            'u' => {
                cursor += 1;
                if characters.get(cursor) == Some(&'{') {
                    while cursor < characters.len() && characters[cursor] != '}' {
                        cursor += 1;
                    }
                    cursor += 1;
                }
            }
            _ => cursor += 1,
        }
    } else {
        cursor += 1;
    }
    (characters.get(cursor) == Some(&'\'')).then_some(cursor + 1)
}

/// Every workspace package directory reachable from `manifest_dir` through
/// normal and build dependency edges, the package itself included.
fn closure_directories(manifest_dir: &Path) -> Vec<PathBuf> {
    let (mut pending, mut reached) = (vec![manifest_dir.to_path_buf()], BTreeSet::new());
    while let Some(directory) = pending.pop() {
        let directory = canonical(&directory);
        if !reached.insert(directory.clone()) {
            continue;
        }
        let manifest = std::fs::read_to_string(directory.join("Cargo.toml"))
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()));
        for dependency in path_dependencies(&manifest) {
            pending.push(directory.join(dependency));
        }
    }
    reached.into_iter().collect()
}

/// The package directory a file belongs to: the deepest of `packages` that
/// contains it, which is the value `env!("CARGO_MANIFEST_DIR")` has in it.
fn enclosing_package(source: &Path, packages: &[PathBuf]) -> Option<PathBuf> {
    packages
        .iter()
        .filter(|package| source.starts_with(package))
        .max_by_key(|package| package.components().count())
        .cloned()
}

/// Every file under a package that its binaries compile: all of the package
/// except the directories in [`UNCOMPILED_DIRECTORIES`], whatever the file is
/// called. Every file rather than `.rs` alone, because a compile can read a
/// file no extension announces (`include_str!("schema.sql")`).
fn package_sources(directory: &Path) -> Vec<PathBuf> {
    let mut sources = BTreeSet::new();
    let mut directories = vec![directory.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(&directory)
            .into_iter()
            .flatten()
            .flatten()
        {
            let name = entry.file_name().to_string_lossy().to_string();
            if UNCOMPILED_DIRECTORIES.contains(&name.as_str()) {
                continue;
            }
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => directories.push(entry.path()),
                Ok(_) => {
                    sources.insert(entry.path());
                }
                _ => {}
            }
        }
    }
    sources.into_iter().collect()
}

/// The `path = "..."` values of a manifest's dependency tables. Dev-dependency
/// tables are skipped: a binary never compiles them, so their content says
/// nothing about it and refusing a current binary for one would be a refusal
/// no rebuild could clear.
fn path_dependencies(manifest: &str) -> Vec<String> {
    let mut dependencies = Vec::new();
    let mut table = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            table = is_dependency_table(line);
            continue;
        }
        if !table {
            continue;
        }
        if let Some(value) = inline_table_value(line, "path") {
            dependencies.push(value);
        }
    }
    dependencies
}

fn is_dependency_table(header: &str) -> bool {
    let header = header.trim_start_matches('[').trim_end_matches(']');
    if header.contains("dev-dependencies") {
        return false;
    }
    header == "dependencies"
        || header == "build-dependencies"
        || header.ends_with(".dependencies")
        || header.ends_with(".build-dependencies")
}

/// The value of `key` inside a single-line inline table (`{ key = "value" }`).
/// A dependency written across several lines is not read; no manifest here
/// uses that form.
fn inline_table_value(line: &str, key: &str) -> Option<String> {
    let (_, table) = line.split_once('=')?;
    let table = table.trim().strip_prefix('{')?.trim_end_matches('}');
    table.split(',').find_map(|entry| {
        let (name, value) = entry.split_once('=')?;
        (name.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
    })
}

/// The directory holding the workspace's packages: the nearest ancestor
/// manifest that declares `[workspace]`.
fn workspace_root(manifest_dir: &Path) -> Result<PathBuf, String> {
    let mut directory = Some(canonical(manifest_dir));
    while let Some(candidate) = directory {
        let manifest = candidate.join("Cargo.toml");
        if std::fs::read_to_string(&manifest)
            .is_ok_and(|manifest| manifest.lines().any(|line| line.trim() == "[workspace]"))
        {
            return Ok(candidate);
        }
        directory = candidate.parent().map(Path::to_path_buf);
    }
    Err(format!(
        "no [workspace] manifest above {}",
        manifest_dir.display()
    ))
}

/// The sha256 of a file's bytes, or `None` when it cannot be read — a file the
/// build consumed and the tree no longer holds is a difference, not an error.
fn content_hash(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|content| format!("{:x}", Sha256::digest(content)))
}

fn is_rust(source: &Path) -> bool {
    source
        .extension()
        .is_some_and(|extension| extension == "rs")
}

fn relative_to(workspace: &Path, source: &Path) -> PathBuf {
    source
        .strip_prefix(workspace)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| source.to_path_buf())
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn environment(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is set when a build script runs"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("symbiote-stamp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        directory
    }

    /// A binary carrying a record over exactly `sources` — the shape
    /// `build_stamp` produces, so a test can check what the walk recorded
    /// rather than re-listing what it should have found.
    fn record_over(root: &Path, sources: &BTreeSet<PathBuf>) -> PathBuf {
        let mut record = String::new();
        for source in sources {
            let content = std::fs::read(source).expect("a recorded source");
            record.push_str(&format!(
                "{:x}\t{}\n",
                Sha256::digest(&content),
                relative_to(root, source).display()
            ));
        }
        let binary = root.join("symbiote-example");
        std::fs::write(
            &binary,
            format!("binary bytes before {RECORD_START}{record}{RECORD_END} and after"),
        )
        .expect("the binary");
        binary
    }

    fn record_for(root: &Path, files: &[(&str, &str)]) -> PathBuf {
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\n[package]\nname = \"symbiote-example\"\n",
        )
        .expect("the root manifest");
        let mut sources = BTreeSet::new();
        for (path, content) in files {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("directories");
            std::fs::write(&file, content).expect("the source");
            sources.insert(file);
        }
        record_over(root, &sources)
    }

    #[test]
    fn dependency_tables_are_read_and_dev_and_target_tables_are_not() {
        let manifest = "\
[package]
name = \"example\"

[dependencies]
symbiote-a = { path = \"../a\" }
serde = { version = \"1\", path = \"../serde\", features = [\"derive\"] }
nix = \"0.30\"

[build-dependencies]
stamp = { path = \"../stamp\" }

[dev-dependencies]
symbiote-test = { path = \"../test\" }

[target.'cfg(unix)'.dependencies]
unix = { path = \"../unix\" }

[target.'cfg(unix)'.dev-dependencies]
unix-test = { path = \"../unix-test\" }

[[bin]]
name = \"example\"
path = \"src/bin/example.rs\"
";
        assert_eq!(
            path_dependencies(manifest),
            ["../a", "../serde", "../stamp", "../unix"]
        );
    }

    #[test]
    fn a_path_dependency_split_across_lines_is_not_claimed() {
        assert!(path_dependencies("[dependencies]\nx = {\n  path = \"../x\"\n}\n").is_empty());
    }

    #[test]
    fn the_scan_reads_code_and_not_comments_or_strings() {
        for text in [
            "/// include_str!(\"doc.txt\")",
            "// include_str!(\"doc.txt\")",
            "/* include_str!(\"doc.txt\") */",
            "/* /* nested */ include_str!(\"doc.txt\") */",
            "let example = \"include_str!(\\\"doc.txt\\\")\";",
            "let example = r#\"include_str!(\"doc.txt\")\"#;",
            "let example = '\\'';",
            "/// #[path = \"doc.rs\"]",
            "// #[path = \"doc.rs\"]",
            "/* #[path = \"doc.rs\"] */",
            "let example = \"#[path = \\\"doc.rs\\\"] mod m;\";",
            "let example = r#\"#[path = \"doc.rs\"] mod m;\"#;",
        ] {
            assert_eq!(
                (include_arguments(text).len(), module_paths(text).len()),
                (0, 0),
                "{text} names no input: recording one would refuse a current binary for a change \
                 to a file the build never reads"
            );
        }
    }

    #[test]
    fn an_include_after_a_string_is_still_code() {
        let text = "let quote = \"a \\\" b\"; include_str!(\"real.json\");";
        assert!(matches!(
            include_arguments(text).as_slice(),
            [Include::Literal(path)] if path == "real.json"
        ));
    }

    #[test]
    fn a_literal_and_a_built_path_are_told_apart() {
        let literal = include_arguments("include_str!(\n  \"../fixtures/a.json\"\n)");
        assert!(matches!(
            literal.as_slice(),
            [Include::Literal(path)] if path == "../fixtures/a.json"
        ));
        assert!(matches!(
            include_arguments("include_bytes!(\"logo.svg\").as_slice()").as_slice(),
            [Include::Literal(path)] if path == "logo.svg"
        ));

        let manifest_dir = include_arguments(
            "include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/fixtures/a.json\"))",
        );
        assert!(matches!(
            manifest_dir.as_slice(),
            [Include::Parts(parts)]
                if matches!(parts.as_slice(), [Part::ManifestDir, Part::Text(text)] if text == "/fixtures/a.json")
        ));

        assert!(matches!(
            include_arguments("include_bytes!(concat!(env!(\"OUT_DIR\"), \"/generated.bin\"))")
                .as_slice(),
            [Include::Generated]
        ));
    }

    #[test]
    fn a_trailing_comma_in_a_built_path_is_not_a_refusal() {
        for text in [
            "include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/fixtures/a.json\",))",
            "include_str!(concat!(\n    env!(\"CARGO_MANIFEST_DIR\"),\n    \"/fixtures/a.json\",\n))",
        ] {
            assert!(
                matches!(include_arguments(text).as_slice(), [Include::Parts(_)]),
                "{text} is ordinary Rust naming a path the scan can follow, so it must not stop \
                 the build"
            );
        }
    }

    #[test]
    fn a_path_the_scan_cannot_follow_is_reported_rather_than_skipped() {
        for text in [
            "include_str!(PATH_CONST)",
            "include_str!(concat!(env!(\"SOMEWHERE_ELSE\"), \"/a.json\"))",
            "include_bytes!(path_from_the_build())",
        ] {
            assert!(
                matches!(include_arguments(text).as_slice(), [Include::Unknown(_)]),
                "{text} names an input the record cannot name, which must stop the build rather \
                 than leave the record quietly short"
            );
        }
    }

    #[test]
    fn a_recorded_path_is_the_one_the_scan_resolves() {
        let root = fixture("resolved");
        let packages = vec![root.join("crates/app")];
        let source = root.join("crates/app/src/lib.rs");
        std::fs::create_dir_all(source.parent().expect("a parent")).expect("the package");
        std::fs::write(&source, "// no includes here\n").expect("the source");

        let literal = followed_inputs(&source, &packages).expect("a followable path");
        assert!(literal.is_empty());

        std::fs::write(
            &source,
            "const F: &str = include_str!(\"../../../fixtures/a.json\");\n",
        )
        .expect("the source");
        assert_eq!(
            followed_inputs(&source, &packages).expect("a followable path"),
            [root.join("crates/app/src/../../../fixtures/a.json")]
        );

        std::fs::write(
            &source,
            "const F: &str = include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/fixtures/a.json\"));\n",
        )
        .expect("the source");
        assert_eq!(
            followed_inputs(&source, &packages).expect("a followable path"),
            [root.join("crates/app/fixtures/a.json")]
        );

        std::fs::write(&source, "const F: &str = include_str!(UNKNOWN_PATH);\n")
            .expect("the source");
        let problem = followed_inputs(&source, &packages).expect_err("a refusal");
        assert!(problem.contains("include_str"), "{problem}");
        assert!(problem.contains("UNKNOWN_PATH"), "{problem}");
    }

    #[test]
    fn the_workspace_root_is_the_nearest_workspace_manifest() {
        let root = fixture("workspace");
        std::fs::create_dir_all(root.join("crates/inner/src")).expect("crates");
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/inner\"]\n",
        )
        .expect("the root manifest");
        std::fs::write(
            root.join("crates/inner/Cargo.toml"),
            "[package]\nname = \"inner\"\n",
        )
        .expect("the package manifest");
        assert_eq!(
            workspace_root(&root.join("crates/inner")).expect("the workspace root"),
            canonical(&root)
        );
    }

    #[test]
    fn every_compiled_file_is_recorded_and_test_targets_are_not() {
        let root = fixture("walk");
        for path in [
            "src/lib.rs",
            "src/schema.sql",
            "Cargo.toml",
            "build.rs",
            "assets/logo.svg",
        ] {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("directories");
            std::fs::write(&file, "content").expect("the source");
        }
        for path in [
            "tests/process.rs",
            "examples/demo.rs",
            "benches/speed.rs",
            "target/debug/leftover",
        ] {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("directories");
            std::fs::write(&file, "content").expect("the target");
        }
        let sources = package_sources(&root);
        for path in [
            "src/lib.rs",
            "src/schema.sql",
            "Cargo.toml",
            "build.rs",
            "assets/logo.svg",
        ] {
            assert!(
                sources.contains(&root.join(path)),
                "{path} is an input to the binary's build and must be recorded"
            );
        }
        for path in [
            "tests/process.rs",
            "examples/demo.rs",
            "benches/speed.rs",
            "target/debug/leftover",
        ] {
            assert!(
                !sources.contains(&root.join(path)),
                "{path} is not compiled into the binary, so recording it would refuse a \
                 current binary for a change no rebuild could clear"
            );
        }
    }

    #[test]
    fn an_input_compiled_from_outside_the_package_is_recorded_and_named() {
        let root = fixture("outside");
        std::fs::create_dir_all(root.join("crates/app/src")).expect("the package");
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\"]\n",
        )
        .expect("the root manifest");
        std::fs::write(
            root.join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\n",
        )
        .expect("the package manifest");
        std::fs::write(
            root.join("crates/app/src/lib.rs"),
            "pub const FIXTURE: &str = include_str!(\"../../../fixtures/register.json\");\n",
        )
        .expect("the source that compiles a file outside its package");
        let compiled = root.join("fixtures/register.json");
        std::fs::create_dir_all(compiled.parent().expect("a parent")).expect("the fixtures");
        std::fs::write(&compiled, "{\"one\": true}\n").expect("the fixture");

        let (sources, unfollowed) = recorded_sources(&root.join("crates/app"), &root);
        assert!(unfollowed.is_empty(), "{unfollowed:?}");
        assert!(
            sources.contains(&canonical(&compiled)),
            "a package that compiles {} must record it; the walk found {sources:?}",
            compiled.display()
        );

        let binary = record_over(&root, &sources);
        std::fs::write(&compiled, "{\"two\": true}\n").expect("the changed fixture");
        assert_eq!(
            changed_sources(&binary, &root).expect("a readable record"),
            [Input::File(compiled)]
        );
    }

    #[test]
    fn an_input_the_build_names_through_a_built_path_is_recorded_and_named() {
        let root = fixture("built-path");
        std::fs::create_dir_all(root.join("crates/app/src")).expect("the package");
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\"]\n",
        )
        .expect("the root manifest");
        std::fs::write(
            root.join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\n",
        )
        .expect("the package manifest");
        std::fs::write(
            root.join("crates/app/src/lib.rs"),
            "pub const FIXTURE: &str = include_str!(concat!(\n    env!(\"CARGO_MANIFEST_DIR\"),\n    \"/fixtures/register.json\"\n));\n",
        )
        .expect("the source that builds its path at compile time");
        let compiled = root.join("crates/app/fixtures/register.json");
        std::fs::create_dir_all(compiled.parent().expect("a parent")).expect("the fixtures");
        std::fs::write(&compiled, "{\"one\": true}\n").expect("the fixture");

        let (sources, unfollowed) = recorded_sources(&root.join("crates/app"), &root);
        assert!(unfollowed.is_empty(), "{unfollowed:?}");
        assert!(
            sources.contains(&canonical(&compiled)),
            "a package that compiles {} must record it; the walk found {sources:?}",
            compiled.display()
        );

        let binary = record_over(&root, &sources);
        std::fs::write(&compiled, "{\"two\": true}\n").expect("the changed fixture");
        assert_eq!(
            changed_sources(&binary, &root).expect("a readable record"),
            [Input::File(compiled)]
        );
    }

    #[test]
    fn an_input_the_scan_cannot_follow_stops_the_record() {
        let root = fixture("unfollowed");
        std::fs::create_dir_all(root.join("crates/app/src")).expect("the package");
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\"]\n",
        )
        .expect("the root manifest");
        std::fs::write(
            root.join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\n",
        )
        .expect("the package manifest");
        std::fs::write(
            root.join("crates/app/src/lib.rs"),
            "pub static SCHEMA: &str = include_str!(SCHEMA_PATH);\n",
        )
        .expect("the source that builds its path elsewhere");

        let (sources, unfollowed) = recorded_sources(&root.join("crates/app"), &root);
        assert_eq!(unfollowed.len(), 1, "{unfollowed:?}");
        assert!(
            unfollowed[0].contains("lib.rs") && unfollowed[0].contains("SCHEMA_PATH"),
            "{}",
            unfollowed[0]
        );
        assert!(
            !sources.iter().any(|source| source.ends_with("SCHEMA_PATH")),
            "the unnameable input must be refused, not dropped"
        );
    }

    #[test]
    fn a_generated_path_is_left_to_the_build_script_that_writes_it() {
        let root = fixture("generated");
        std::fs::create_dir_all(root.join("crates/app/src")).expect("the package");
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\"]\n",
        )
        .expect("the root manifest");
        std::fs::write(
            root.join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\n",
        )
        .expect("the package manifest");
        std::fs::write(
            root.join("crates/app/src/lib.rs"),
            "include!(concat!(env!(\"OUT_DIR\"), \"/generated.rs\"));\n",
        )
        .expect("the source that includes a generated file");

        let (_, unfollowed) = recorded_sources(&root.join("crates/app"), &root);
        assert!(unfollowed.is_empty(), "{unfollowed:?}");
    }

    #[test]
    fn an_environment_input_that_differs_is_named() {
        let root = fixture("environment");
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\n[package]\nname = \"symbiote-example\"\n",
        )
        .expect("the root manifest");
        let binary = root.join("symbiote-example");
        // A variable no test sets: a record that names it as set is a
        // difference, and a record that names it `unset` is not — which is
        // also the difference between set-and-empty and not set at all.
        for (hash, expected) in [("0123456789abcdef", 1), (UNSET, 0), ("", 1)] {
            std::fs::write(
                &binary,
                format!(
                    "bytes {RECORD_START}{hash}\t{ENVIRONMENT}SYMBIOTE_STAMP_PROBE_VARIABLE\n\
                     {RECORD_END} bytes"
                ),
            )
            .expect("the binary");
            assert_eq!(
                changed_sources(&binary, &root)
                    .expect("a readable record")
                    .len(),
                expected,
                "a record holding {hash:?} for an unset variable"
            );
        }
        assert_eq!(
            changed_sources(&binary, &root).expect("a readable record"),
            [Input::Environment(
                "SYMBIOTE_STAMP_PROBE_VARIABLE".to_owned()
            )]
        );
    }

    #[test]
    fn a_recorded_source_whose_content_differs_is_named() {
        let root = fixture("changed");
        let binary = record_for(&root, &[("src/lib.rs", "one\n")]);
        assert!(
            changed_sources(&binary, &root)
                .expect("a readable record")
                .is_empty()
        );

        std::fs::write(root.join("src/lib.rs"), "two\n").expect("the changed source");
        assert_eq!(
            changed_sources(&binary, &root).expect("a readable record"),
            [Input::File(root.join("src/lib.rs"))]
        );

        std::fs::remove_file(root.join("src/lib.rs")).expect("the removed source");
        assert_eq!(
            changed_sources(&binary, &root).expect("a readable record"),
            [Input::File(root.join("src/lib.rs"))]
        );
    }

    #[test]
    fn a_binary_without_a_record_cannot_be_checked() {
        let root = fixture("no-record");
        let binary = record_for(&root, &[("src/lib.rs", "one\n")]);
        std::fs::write(&binary, "no record here").expect("a binary without a record");
        let problem = changed_sources(&binary, &root).expect_err("no record");
        assert!(problem.contains("carries no build record"), "{problem}");

        let missing = root.join("symbioted");
        let problem = changed_sources(&missing, &root).expect_err("no binary");
        assert!(problem.contains("cannot read the binary"), "{problem}");
    }

    #[test]
    fn the_generated_static_carries_the_record_verbatim() {
        let record = "aa\tcrates/one/Cargo.toml\nbb\tCargo.lock\n";
        let generated = record_static(record);
        assert!(generated.contains(RECORD_STATIC), "{generated}");
        assert!(
            generated.lines().count() == 3,
            "the record must be one escaped literal, not raw lines: {generated}"
        );
        assert!(
            !generated.contains("Cargo.lock\nbb"),
            "a raw newline would end the literal: {generated}"
        );

        // The check reads what a binary holds, which is what that literal
        // compiles to: the record, framed by the markers.
        let binary = format!("noise{RECORD_START}{record}{RECORD_END}noise");
        assert_eq!(embedded_record(binary.as_bytes()).as_deref(), Some(record));
    }

    #[test]
    fn a_workspace_whose_root_is_not_a_workspace_is_an_error() {
        let root = fixture("no-workspace");
        let problem = workspace_root(&root).expect_err("no workspace");
        assert!(problem.contains("no [workspace] manifest"), "{problem}");
    }

    #[test]
    fn the_workspace_manifest_is_recorded_and_named() {
        let root = fixture("workspace-manifest");
        let package = root.join("crates/app");
        std::fs::create_dir_all(package.join("src")).expect("the package");
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\"]\n\n[workspace.package]\nedition = \"2024\"\n",
        )
        .expect("the workspace manifest");
        std::fs::write(
            package.join("Cargo.toml"),
            "[package]\nname = \"app\"\nedition.workspace = true\n",
        )
        .expect("the package manifest");
        std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");

        let (sources, unfollowed) = recorded_sources(&package, &root);
        assert!(unfollowed.is_empty(), "{unfollowed:?}");
        assert!(
            sources.contains(&root.join("Cargo.toml")),
            "the manifest that sets the edition this package compiles under must be recorded; the \
             walk found {sources:?}"
        );

        let binary = record_over(&root, &sources);
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\"]\n\n[workspace.package]\nedition = \"2021\"\n",
        )
        .expect("the changed workspace manifest");
        assert_eq!(
            changed_sources(&binary, &root).expect("a readable record"),
            [Input::File(root.join("Cargo.toml"))]
        );
    }

    #[test]
    fn a_dependency_in_a_second_workspace_contributes_that_workspace_root() {
        let root = fixture("second-workspace");
        let package = root.join("wsA/app");
        let elsewhere = root.join("other");
        std::fs::create_dir_all(package.join("src")).expect("the package");
        std::fs::create_dir_all(elsewhere.join("dep/src")).expect("the dependency");
        std::fs::write(
            root.join("wsA/Cargo.toml"),
            "[workspace]\nmembers = [\"app\"]\n\n[workspace.package]\nedition = \"2024\"\n",
        )
        .expect("the workspace manifest");
        std::fs::write(
            package.join("Cargo.toml"),
            "[package]\nname = \"app\"\nedition.workspace = true\n\n[dependencies]\n\
             dep = { path = \"../../other/dep\" }\n",
        )
        .expect("the package manifest");
        std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");
        // The dependency is a member of a workspace of its own, whose root is
        // the manifest that sets the edition and version it is compiled with.
        std::fs::write(
            elsewhere.join("Cargo.toml"),
            "[workspace]\nmembers = [\"dep\"]\n\n[workspace.package]\nedition = \"2024\"\n",
        )
        .expect("the second workspace manifest");
        std::fs::write(
            elsewhere.join("dep/Cargo.toml"),
            "[package]\nname = \"dep\"\nedition.workspace = true\n",
        )
        .expect("the dependency manifest");
        std::fs::write(elsewhere.join("dep/src/lib.rs"), "pub fn nothing() {}\n")
            .expect("the dependency source");

        let (sources, unfollowed) = recorded_sources(&package, &root.join("wsA"));
        assert!(unfollowed.is_empty(), "{unfollowed:?}");
        assert!(
            sources.contains(&elsewhere.join("Cargo.toml")),
            "the manifest that sets this dependency's edition must be recorded; the walk found \
             {sources:?}"
        );

        let binary = record_over(&root.join("wsA"), &sources);
        std::fs::write(
            elsewhere.join("Cargo.toml"),
            "[workspace]\nmembers = [\"dep\"]\n\n[workspace.package]\nedition = \"2021\"\n",
        )
        .expect("the changed second workspace manifest");
        assert_eq!(
            changed_sources(&binary, &package).expect("a readable record"),
            [Input::File(elsewhere.join("Cargo.toml"))]
        );
    }

    #[test]
    fn a_module_attribute_at_the_top_of_a_file_is_recorded_and_named() {
        let root = fixture("module");
        std::fs::create_dir_all(root.join("crates/app/src")).expect("the package");
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\"]\n",
        )
        .expect("the root manifest");
        std::fs::write(
            root.join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\n",
        )
        .expect("the package manifest");
        std::fs::write(
            root.join("crates/app/src/lib.rs"),
            "#[path = \"../../../fixtures/module.rs\"]\nmod module;\n",
        )
        .expect("the source that declares a module outside its package");
        let compiled = root.join("fixtures/module.rs");
        std::fs::create_dir_all(compiled.parent().expect("a parent")).expect("the fixtures");
        std::fs::write(&compiled, "pub const ONE: u8 = 1;\n").expect("the module");

        let (sources, unfollowed) = recorded_sources(&root.join("crates/app"), &root);
        assert!(unfollowed.is_empty(), "{unfollowed:?}");
        assert!(
            sources.contains(&canonical(&compiled)),
            "a module declared outside the package must be recorded; the walk found {sources:?}"
        );

        let binary = record_over(&root, &sources);
        std::fs::write(&compiled, "pub const ONE: u8 = 2;\n").expect("the changed module");
        assert_eq!(
            changed_sources(&binary, &root).expect("a readable record"),
            [Input::File(compiled)]
        );
    }

    #[test]
    fn a_module_attribute_inside_an_inline_module_records_both_directories() {
        let root = fixture("nested-module");
        let package = root.join("crates/app");
        std::fs::create_dir_all(package.join("src")).expect("the package");
        std::fs::write(package.join("Cargo.toml"), "[package]\nname = \"app\"\n")
            .expect("the package manifest");
        std::fs::write(
            package.join("src/leaf.rs"),
            "mod inline {\n    #[path = \"deep.rs\"]\n    mod deep;\n}\n",
        )
        .expect("the source that nests a module attribute");
        // The compiler resolves this against `src/leaf/` when it found the file
        // as `mod leaf;`, and against `src/` when a `#[path]` named it. The scan
        // cannot tell the two apart from the file alone, so a record that is
        // never short names both.
        let candidates = ["src/leaf/inline/deep.rs", "src/inline/deep.rs"];
        for path in candidates {
            let file = package.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("the directories");
            std::fs::write(&file, "pub const DEEP: u8 = 1;\n").expect("the module");
        }

        let (sources, unfollowed) = recorded_sources(&package, &root);
        assert!(unfollowed.is_empty(), "{unfollowed:?}");
        for path in candidates {
            assert!(
                sources.contains(&package.join(path)),
                "the record must not be short of {path}; the walk found {sources:?}"
            );
        }
    }

    #[test]
    fn a_module_path_resolves_where_the_compiler_looks_for_it() {
        let root = PathBuf::from("example");
        let nested = ["inner".to_owned()];
        assert_eq!(
            module_inputs(&root.join("src/lib.rs"), "a.rs", &[]),
            [root.join("src/a.rs")]
        );
        assert_eq!(
            module_inputs(&root.join("src/lib.rs"), "a.rs", &nested),
            [root.join("src/inner/a.rs")]
        );
        assert_eq!(
            module_inputs(&root.join("src/leaf/mod.rs"), "a.rs", &nested),
            [root.join("src/leaf/inner/a.rs")]
        );
        assert_eq!(
            module_inputs(&root.join("src/leaf.rs"), "a.rs", &nested),
            [
                root.join("src/inner/a.rs"),
                root.join("src/leaf/inner/a.rs")
            ]
        );
    }

    #[test]
    fn a_module_attribute_is_read_from_the_item_it_declares() {
        assert!(matches!(
            module_paths("#[path = \"../shared.rs\"]\npub mod shared;\n").as_slice(),
            [ModulePath::File { path, inline }] if path == "../shared.rs" && inline.is_empty()
        ));
        assert!(matches!(
            module_paths("#[cfg(test)]\n#[path = \"../shared.rs\"]\nmod shared;\n").as_slice(),
            [ModulePath::File { path, .. }] if path == "../shared.rs"
        ));
        assert!(matches!(
            module_paths("mod inline {\n    #[path = \"../shared.rs\"]\n    mod shared;\n}\n").as_slice(),
            [ModulePath::File { inline, .. }] if inline.as_slice() == ["inline"]
        ));
        assert!(matches!(
            module_paths("pub mod r#type {\n    #[path = \"deep.rs\"]\n    mod deep;\n}\n")
                .as_slice(),
            [ModulePath::File { inline, .. }] if inline.as_slice() == ["type"]
        ));
    }

    #[test]
    fn a_module_attribute_the_scan_cannot_read_is_reported_rather_than_skipped() {
        for text in [
            "#[path = concat!(\"src\", \"/a.rs\")]\nmod a;\n",
            "#[cfg_attr(unix, path = \"../a.rs\")]\nmod a;\n",
            "#[path = \"sub\"]\nmod inline {\n}\n",
            "mod outer {\n    #[path = \"sub\"]\n    mod inner {\n    }\n}\n",
            "#[path = \"../a.rs\"]\nfn a() {}\n",
        ] {
            let paths = module_paths(text);
            assert!(
                matches!(
                    paths.as_slice(),
                    [ModulePath::Unreadable(_)] | [ModulePath::Inline { .. }]
                ),
                "{text} names a module source the record cannot follow, which must stop the build \
                 rather than leave the record quietly short; the scan read {paths:?}"
            );
        }
    }
}
