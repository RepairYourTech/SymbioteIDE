//! The facts one manifest states, read as text.
//!
//! The walk asks this module three questions. Which packages does a manifest
//! reach: the edges of its dependency tables and, when it is a root, of its
//! `[patch]` table, which cargo accepts in more than one spelling — an inline
//! table beside the dependency's name, the dotted key that TOML defines as the
//! same table, a table of its own, or a `[patch]` key that replaces a
//! dependency's source with a directory — and an edge this module does not read
//! is a package whose sources no record names.
//! Which of those edges does a workspace root resolve instead: a declaration
//! that says `workspace = true` names no path of its own, and the root's path
//! applies. And where does a package belong: whether a manifest declares a
//! `[workspace]`, which workspace a `[package]` names for itself, and which
//! paths a root declares for its members to inherit.
//!
//! This is a reader for the part of a manifest the walk needs, not a TOML
//! parser. It is line-oriented, and what it cannot follow it reports rather than
//! guesses.

use std::collections::{BTreeMap, BTreeSet};

/// The facts one manifest states, from one reading of it.
#[derive(Default)]
pub(crate) struct Manifest {
    /// The dependency edges this manifest resolves itself, in the order it
    /// states them. A declaration that takes its path from a workspace root is
    /// not among them — [`Manifest::inherited`] names it instead — and neither
    /// is a `[patch]` key, which is not an edge of a dependency table.
    pub(crate) edges: Vec<DependencyEdge>,
    /// The `[patch]` replacements this manifest declares, as the table states
    /// them. Whether cargo applies them is the *walk's* question rather than this
    /// reader's: measured, a patch in a member's manifest is ignored with a
    /// warning (`patch for the non root package will be ignored`) even when the
    /// directory it names is not there, while a package no `[workspace]` manifest
    /// owns is its own root and its patch applies — and only the walk knows which
    /// directories a given build resolves in (`walk::patch_directories`).
    pub(crate) patches: Vec<DependencyEdge>,
    /// The dependencies this manifest takes from a workspace root rather than
    /// naming a path itself: `dep.workspace = true`, in whichever spelling.
    pub(crate) inherited: BTreeSet<String>,
    /// The paths this manifest declares for members to inherit, when it is a
    /// workspace root: `[workspace.dependencies]`, by dependency name.
    pub(crate) shared: BTreeMap<String, String>,
}

/// One edge out of a manifest's dependency tables.
pub(crate) struct DependencyEdge {
    /// The directory the edge names, relative to the manifest it was read from
    /// — or absolute, when a workspace root declared it: a root's path is
    /// relative to that root rather than to the package that inherits it, so
    /// the walk joins the root's directory before handing the edge on.
    pub(crate) path: String,
    /// Whether the edge names an input only when the directory is there.
    ///
    /// Two edges are conditional, for different reasons. A `[patch]` key is
    /// one, because cargo reads it only where it applies and ignores one it does
    /// not use. A dependency a member inherits is the other, because the root it
    /// inherits from is one of several candidates (see `walk::workspace_roots`)
    /// and a root cargo never read may name a directory that is not there. A
    /// dependency this manifest resolves itself is not: cargo resolves every one
    /// of them for the target being built.
    pub(crate) conditional: bool,
}

/// Reads everything the walk asks of one manifest.
pub(crate) fn read(manifest: &str) -> Manifest {
    let mut stated = Manifest::default();
    for (table, lines) in tables(manifest) {
        // One declaration per name, however the table spells its keys: a dotted
        // spelling spreads one declaration over several lines.
        let mut declarations: BTreeMap<String, Declaration> = BTreeMap::new();
        for line in lines {
            if let Some((name, keys)) = declared(line, table.names_one()) {
                declarations.entry(name.to_owned()).or_default().merge(keys);
            }
        }
        for (name, keys) in declarations {
            let patch = match table {
                Table::Shared { .. } => {
                    if let Some(path) = keys.path {
                        stated.shared.insert(name, path);
                    }
                    continue;
                }
                Table::Edges { patch } | Table::Edge { patch, .. } => patch,
                Table::Workspace | Table::Package | Table::Other => continue,
            };
            // A declaration that inherits its path names no edge here, and a
            // `path` written beside `workspace = true` is ignored: measured, the
            // root's path is the one cargo resolves, in every spelling. Reading
            // the member's own path would follow a directory no build reads.
            if keys.inherited {
                stated.inherited.insert(name);
                continue;
            }
            if let Some(path) = keys.path {
                let edge = DependencyEdge {
                    path,
                    conditional: patch,
                };
                if patch {
                    stated.patches.push(edge);
                } else {
                    stated.edges.push(edge);
                }
            }
        }
    }
    stated
}

/// A manifest grouped by its tables: each header with the lines under it, so a
/// reader sees one table at a time. Grouping matters because a name may be
/// declared in more than one table — a dependency and the `[patch]` replacement
/// of it are both compiled, and each is read from its own table.
fn tables(manifest: &str) -> Vec<(Table, Vec<&str>)> {
    let mut tables: Vec<(Table, Vec<&str>)> = Vec::new();
    for line in manifest.lines() {
        let line = code(line);
        match table_name(line) {
            Some(header) => tables.push((Table::of(&header), Vec::new())),
            // A line above the first header belongs to no table and declares
            // nothing: a manifest whose first line sets no table has no
            // dependencies to declare.
            None => {
                if let Some((_, lines)) = tables.last_mut() {
                    lines.push(line);
                }
            }
        }
    }
    tables
}

/// What a manifest table is to the readers of this module.
enum Table {
    /// A table whose keys name dependencies: `[dependencies]`,
    /// `[build-dependencies]`, and their target-specific forms.
    Edges { patch: bool },
    /// A table describing one dependency, whose name the header carries as its
    /// last segment: `[dependencies.dep]`. Cargo lets a manifest spell one edge
    /// either way.
    Edge { name: String, patch: bool },
    /// `[workspace.dependencies]` and `[workspace.dependencies.dep]`: the same
    /// two shapes, declaring paths for members to inherit. None of them is an
    /// edge of this manifest — cargo compiles one only where a member asks for
    /// it by name.
    Shared { name: Option<String> },
    /// `[workspace]`: what makes a manifest a root, declaring no edge itself.
    Workspace,
    /// `[package]`, whose `workspace` key is read by [`package_workspace`].
    Package,
    /// Anything else, every dev-dependency table included.
    Other,
}

impl Table {
    /// The table a manifest header opens, from the segments of its name: a
    /// dependency is declared by the segment before the last and named by the
    /// last, so `[dependencies]` lists the edges and `[dependencies.dep]`
    /// describes one of them.
    ///
    /// A `[patch]` key is a path like any other here — cargo compiles the
    /// directory it names where it applies — and is read in both spellings too,
    /// as the one kind of edge cargo may ignore.
    fn of(header: &str) -> Self {
        let segments = split_unquoted(header, '.');
        if segments.contains(&"dev-dependencies") {
            return Self::Other;
        }
        let declaration =
            |segment: &str| segment == "dependencies" || segment == "build-dependencies";
        match segments.as_slice() {
            ["workspace"] => Self::Workspace,
            ["package"] => Self::Package,
            ["workspace", "dependencies"] => Self::Shared { name: None },
            ["workspace", "dependencies", name] => Self::Shared {
                name: Some((*name).to_owned()),
            },
            ["patch", _] => Self::Edges { patch: true },
            ["patch", _, name] => Self::Edge {
                name: (*name).to_owned(),
                patch: true,
            },
            [.., name] if declaration(name) => Self::Edges { patch: false },
            [.., declares, name] if declaration(declares) => Self::Edge {
                name: (*name).to_owned(),
                patch: false,
            },
            _ => Self::Other,
        }
    }

    /// The dependency this table's header names, when it names one:
    /// `[dependencies.dep]` describes `dep`, and `[dependencies]` describes none
    /// in particular.
    fn names_one(&self) -> Option<&str> {
        match self {
            Self::Edge { name, .. } | Self::Shared { name: Some(name) } => Some(name),
            _ => None,
        }
    }
}

/// The keys of one dependency declaration, as far as the walk reads them.
#[derive(Default)]
struct Declaration {
    /// The `path` the declaration names.
    path: Option<String>,
    /// Whether it says `workspace = true`.
    inherited: bool,
}

impl Declaration {
    /// Reads one `key = value` pair of the declaration.
    fn set(&mut self, key: &str, value: &str) {
        match key {
            "path" => self.path = string_value(value),
            "workspace" => self.inherited = true_value(value),
            _ => {}
        }
    }

    /// Merges the keys another line sets on the same declaration: a dotted
    /// spelling spreads one declaration over several lines.
    fn merge(&mut self, other: Self) {
        if other.path.is_some() {
            self.path = other.path;
        }
        self.inherited |= other.inherited;
    }
}

/// The dependency one line declares and the keys it sets on it.
///
/// A table that lists dependencies names each one in the line, and cargo takes
/// the keys either way it is spelled: `dep = { path = "…", workspace = true }`
/// sets two keys at once, and the dotted key `dep.path = "…"` sets one. A table
/// that describes a single dependency spells its keys bare, so `name` carries
/// the dependency its header named. A line that declares no dependency — a key
/// of a table that has nothing to do with dependencies — yields `None`.
fn declared<'a>(line: &'a str, name: Option<&'a str>) -> Option<(&'a str, Declaration)> {
    let (key, value) = line.split_once('=')?;
    let mut keys = Declaration::default();
    if let Some(table) = value.trim().strip_prefix('{') {
        for entry in split_unquoted(table.trim_end_matches('}'), ',') {
            if let Some((name, value)) = entry.split_once('=') {
                keys.set(name.trim(), value);
            }
        }
        return Some((key.trim(), keys));
    }
    match name {
        Some(name) => {
            keys.set(key.trim(), value);
            Some((name, keys))
        }
        None => {
            let (name, key) = key.trim().rsplit_once('.')?;
            keys.set(key.trim(), value);
            Some((name.trim(), keys))
        }
    }
}

/// The name inside a table header, or `None` when the line is not one. TOML
/// allows whitespace inside the brackets and a comment after them, so
/// `[ workspace ]` and `[workspace] # the members` both name the same table as
/// `[workspace]`, and a line that is a key is not a header at all.
fn table_name(line: &str) -> Option<String> {
    let line = line.split('#').next()?.trim();
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.split_whitespace().collect())
}

/// `text` split at every `delimiter` no quoted string holds, each part kept
/// whole: the `.` of a table header and the `,` of an inline table separate
/// fields only outside quotes, so a quoted name, path or list that holds one
/// keeps it — `[target.'cfg(unix)'.dependencies]` is three segments and
/// `path = "../a,b"` is one entry.
pub(crate) fn split_unquoted(text: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = text;
    while let Some(index) = unquoted(rest, delimiter) {
        parts.push(&rest[..index]);
        rest = &rest[index + 1..];
    }
    parts.push(rest);
    parts
}

/// The index of the first `target` in `line` that no quoted string holds, or
/// `None` when every one of them is quoted. A quoted string is delimited by
/// `"` or `'`, and one rule reads a header, a path, a comment and an inline
/// table alike: the `.`, `,` or `#` inside quotes is a character of the name or
/// the path it belongs to, not the punctuation it looks like.
fn unquoted(line: &str, target: char) -> Option<usize> {
    let mut quote = None;
    for (index, character) in line.char_indices() {
        match quote {
            Some(open) if character == open => quote = None,
            Some(_) => {}
            None if character == '"' || character == '\'' => quote = Some(character),
            None if character == target => return Some(index),
            None => {}
        }
    }
    None
}

/// The code of a manifest line: everything before the `#` a comment starts at,
/// which is nothing but text and declares no edge. A `#` inside a string is a
/// character of the value — a path may hold one.
fn code(line: &str) -> &str {
    match unquoted(line, '#') {
        Some(index) => &line[..index],
        None => line,
    }
}

/// Whether a TOML boolean scalar is `true`: cargo spells `workspace = true`
/// without quotes, so this reads the bare word rather than a string, and
/// `string_value` reads `false` as no path rather than as one named `false`.
fn true_value(value: &str) -> bool {
    value.trim() == "true"
}

/// Whether a manifest declares a workspace of its own.
pub(crate) fn declares_workspace(manifest: &str) -> bool {
    tables(manifest)
        .iter()
        .any(|(table, _)| matches!(table, Table::Workspace))
}

/// The `workspace = "..."` path of a manifest's `[package]` table, if it has
/// one: the workspace a package names for itself rather than inheriting from an
/// ancestor. Cargo spells it as a plain key — `workspace = "../.."` — so this
/// reads a scalar.
pub(crate) fn package_workspace(manifest: &str) -> Option<String> {
    for (table, lines) in tables(manifest) {
        if !matches!(table, Table::Package) {
            continue;
        }
        for line in lines {
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            if name.trim() == "workspace" {
                return string_value(value);
            }
        }
    }
    None
}

/// The name a manifest's `[package]` table declares, if it has one: what cargo
/// selects a package by (`-p <name>`) when the walk asks cargo about a build.
pub(crate) fn package_name(manifest: &str) -> Option<String> {
    for (table, lines) in tables(manifest) {
        if !matches!(table, Table::Package) {
            continue;
        }
        for line in lines {
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            if name.trim() == "name" {
                return string_value(value);
            }
        }
    }
    None
}

/// The content of a TOML string scalar — `"…"` or `'…'` — with a comment or
/// whitespace after it ignored. `None` for anything else: a value a manifest
/// does not spell as a string is not a path this can follow, and leaving it
/// unrecorded is the safe direction where the walk's other candidates still
/// cover the file.
fn string_value(value: &str) -> Option<String> {
    let value = value.trim();
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &value[1..];
    let mut escaped = false;
    for (index, character) in rest.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && quote == '"' {
            escaped = true;
            continue;
        }
        if character == quote {
            return Some(rest[..index].to_owned());
        }
    }
    None
}

/// The arguments a cargo configuration file expands an alias to, if it defines
/// one: `[alias]`'s key, as a string or as an array of strings, the two spellings
/// cargo accepts.
///
/// A cargo configuration file is not a manifest, but the part this reads is the
/// same text: a table header and `key = <value>` lines. What it is for is the
/// invocation reading in `walk`: an alias replaces the subcommand, so the
/// command line of the cargo that ran a build names the alias and not the
/// manifest the alias made it build (measured: `[alias] b = "build
/// --manifest-path ../sibling/member/Cargo.toml"` leaves the parent process's
/// command line reading `["cargo", "b", "--offline"]` while that manifest's
/// workspace is what the resolution was read in), and the expansion is the only
/// place that fact is written.
///
/// A string value is split on whitespace, which is how cargo reads one.
pub(crate) fn alias(configuration: &str, name: &str) -> Option<Vec<String>> {
    let mut in_alias = false;
    for line in configuration.lines() {
        let line = code(line);
        if let Some(header) = table_name(line).as_deref() {
            in_alias = header == "alias";
            continue;
        }
        if !in_alias {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == name {
            return strings(value.trim());
        }
    }
    None
}

/// The argument strings a TOML value holds: one string, split on whitespace as a
/// command line, or an array of strings taken as they are.
fn strings(value: &str) -> Option<Vec<String>> {
    if let Some(single) = string_value(value) {
        return Some(single.split_whitespace().map(str::to_owned).collect());
    }
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    let mut arguments = Vec::new();
    for element in split_unquoted(inner, ',') {
        let element = element.trim();
        if !element.is_empty() {
            arguments.push(string_value(element)?);
        }
    }
    Some(arguments)
}

#[cfg(test)]
mod tests;
