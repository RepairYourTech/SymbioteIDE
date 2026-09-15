#!/usr/bin/env python3
"""Whether a driven binary's embedded record covers every input its build read.

The end-to-end proofs drive workspace binaries, and each binary carries a
record of the inputs its own build compiled (`symbiote-source-stamp`). The
proofs compare that record against the tree, so a proof cannot run against a
binary built from other sources. What no proof checks is whether the record
*covers every input the compiler read*: if a new way to compile a file appears
and the walk does not follow it, the record is quietly short, every proof stays
green, and only an audit finds it.

This tool is that independent check, and it exists because a disagreement was
found by hand: the walk did not follow `#[path = "…"]` module attributes, so a
module reached that way could be compiled and left unnamed — observed with a
probe module in a closure crate pointing at a repo-root fixture, where the
record held 98 entries and named none of it while cargo's own record named the
file. The oracle it uses is cargo's own per-unit dep-info, written by rustc for
each unit that produces the binary. The record's implementation never reads it
— its module documentation says why: it is a private, versioned binary format
written *after* the build script that must write the record, so it can neither
populate a record on a first build nor be an input the documented rebuild could
clear. A check that runs after the build has neither problem.

Three things are compared:

* **Completeness (rustc).** Every workspace file the units of the driven
  binary's closure read must be named by the record. Registry sources and
  generated files under the target directory are excluded, as the record's own
  documentation excludes them, and so are the directories the wire file's
  `exclusion` lines name, which the record excludes on purpose and whose own units
  read them. Which names those are, and where each is skipped, is the file's: the
  target directories cargo looks for at a package root alone are skipped there
  alone, so `src/tests/mod.rs` compiles into the binary and the record must name
  it, while generated, installed and version-control state is skipped wherever it
  sits and a name the file does not list — `dist` among them — is walked. A
  skipped directory bounds the walk and not the build, so a file the compiler
  finds in one by a module's own name is inside the record too, through the
  declaration that names it.

  The comparison rests on the dep-info of those closure units, so it is only as
  good as finding them: where none of the driven package's own units left one — a
  target directory that was cleaned, or one the binary was copied out of — the
  check has measured nothing and says so rather than passing.
* **Completeness (cargo).** The record must also name, for every closure
  package, its own manifest and every workspace root cargo can resolve for it —
  the root a `package.workspace` names, or else each ancestor manifest declaring
  `[workspace]` and the package's own directory, which is the root of one its
  workspace `exclude`s — together with each root's lockfile. These are read by cargo rather
  than compiled by rustc, so no per-unit dep-info names them — on a build-only checkout the
  oracle above names none of them, and a record that silently stopped covering
  them would pass everything else. One of them is not a technicality: a
  workspace manifest carries `edition.workspace = true` and
  `version.workspace = true` for its members, so its content decides what they
  compile as while every `.rs` file stays byte-identical, which is the exact
  class of stale binary the record exists to refuse. The nearest ancestor alone
  is not enough, which is what this half used to require: an invoked workspace
  can list a member that sits below another `[workspace]` manifest, and then the
  invoked root — not the nearer one — is what the member inherits from. A name
  settles it, so a package that names its root resolves to that root alone:
  measured, cargo reads an inherited path from the named root, and a build that
  both nests the package and claims it as a member is refused.
* **Effective configuration.** The record holds `env:CARGO` and every
  configuration file cargo reads for a build of a closure package
  (`.cargo/config.toml`, `.cargo/config`, `rust-toolchain.toml`,
  `rust-toolchain`). A configuration file that exists now and is not named, or
  a named one whose content moved, is a failure — the class that cannot be
  recorded at build time without making cargo relink the stamped crates on
  every build.

A record is refused rather than compared when it says a build could not stand
behind it, and when it cannot be read at all. `MARKS` holds what each mark the
wire names means and what clears it, and a mark the wire does not name, a line
the wire cannot read, or a record the wire cannot read at all — bytes that are not
its encoding, or a marker of its own framing among them — is refused through the
one remedy the wire holds for each. Every value of that rule — the framing, the
encoding of the bytes it frames, the division into lines, the contents, the marks
and the remedies — comes from `wire.txt`, the one copy of it, which the crate
that writes a record embeds too, so the two readers cannot disagree about what a
record says; `embedded_record` and `classify` below are this checker's half of
it. A record
can carry both marks at once; the refusal then reports the one a rebuild clears
first, as `changed_sources` reports it too. `symbiote-source-stamp`'s module
documentation measures the arrangements, and `changed_sources` refuses those
records for the same reason.

It reads, and writes nothing:

    python3 planning/integrity/source_record.py \\
        --binary symbiote-host:symbioted \\
        --binary symbiote-sandbox:symbiote-sandbox-launch

Run it after a build of those binaries (the units' dep-info and the binaries
themselves must exist). Exit status is 0 when every read input is named.
"""

import argparse
import hashlib
import json
import os
import shlex
import subprocess
import sys
from pathlib import Path

# The scalar keywords the wire file spells, which is what the crate's own parser
# reads too: the framing and the encoding of the bytes it frames, the variable a
# record pins, the locators and contents, and what a refusal says about a mark the
# file does not name, a line it cannot read, or a record it cannot read at all.
# The marks come from its `mark` lines, the directory names a walk skips from its
# `exclusion` lines, and `unknown-order` and `encoding` are read beside them in
# `read_wire`.
WIRE_KEYWORDS = {
    "start",
    "end",
    "encoding",
    "environment",
    "prefix",
    "unset",
    "hash",
    "unknown",
    "malformed",
    "undecodable",
    "unframed",
}

# The wire file: the one copy of the wire, which the crate that writes a record
# embeds with `include_str!` (`symbiote-source-stamp/src/wire.rs`) and this tool
# reads to refuse one. The path is the repository's own layout, because the wire
# is the contract between two tools that live in one repository.
WIRE_FILE = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "symbiote-source-stamp"
    / "src"
    / "wire.txt"
)


# The scopes the wire file's `exclusion` lines name: the directory names a walk
# skips at a package root alone (`root`, where cargo looks for a target directory
# of that name) and the ones it skips at every depth (`anywhere`). Both are the
# file's, so this checker skips exactly what the walk skips.
EXCLUSION_SCOPES = ("root", "anywhere")


def read_wire(path):
    """The wire file, parsed: its scalar keywords, its marks in order, and the
    directory names a walk skips by scope.

    A line is a keyword and its tab-separated fields, and ``#`` starts a comment.
    A malformed file raises rather than falling back to a default, because every
    value here is one the crate spells the same record with: a framing this tool
    got wrong would make it read a record the crate did not write.
    """
    scalars = {}
    marks = []
    exclusions = {scope: [] for scope in EXCLUSION_SCOPES}
    for line in Path(path).read_text().splitlines():
        line = line.rstrip()
        if not line or line.startswith("#"):
            continue
        keyword, _, fields = line.partition("\t")
        if keyword == "mark":
            role, _, rest = fields.partition("\t")
            locator, _, remedy = rest.partition("\t")
            if not role or not locator or not remedy:
                raise ValueError(f"the wire file mark {fields!r} is incomplete")
            marks.append((role, locator, remedy))
        elif keyword == "exclusion":
            # Which directory names a walk skips, and where it skips them: data, so
            # the walk's rule and this checker's cannot drift and neither reader
            # decides for itself what a name means.
            scope, _, names = fields.partition("\t")
            if scope not in EXCLUSION_SCOPES:
                raise ValueError(
                    f"the wire file names the exclusion scope {scope!r}, which this "
                    f"checker does not implement"
                )
            listed = names.split(" ")
            if not names or any(not name for name in listed):
                raise ValueError(f"the wire file's exclusion {fields!r} names no directories")
            if exclusions[scope]:
                raise ValueError(f"the wire file spells the {scope!r} exclusions twice")
            exclusions[scope] = listed
        elif keyword == "hash":
            # The shape of a hash — the content a record gives an input that is
            # there — so what tells an input's line from a mark's is the file's to
            # say rather than each reader's to decide.
            length, _, alphabet = fields.partition("\t")
            if not length.isdigit() or int(length) < 1 or not alphabet:
                raise ValueError(f"the wire file's hash shape {fields!r} is incomplete")
            scalars["hash"] = (int(length), alphabet)
        elif keyword == "encoding":
            # How the bytes between the framing markers are spelled: data, so both
            # readers decode a record by the file's own word rather than each
            # assuming one. This checker decodes with Python's codecs, so a file
            # naming an encoding it does not know is refused here rather than
            # silently decoded as the default.
            if fields != "UTF-8":
                raise ValueError(
                    f"the wire file names the record encoding {fields!r}, which this "
                    f"checker does not implement"
                )
            scalars["encoding"] = fields
        elif keyword == "unknown-order":
            # Which of several marks the file does not name a refusal reports. It
            # is read as data so a reader cannot invent its own order, which is
            # how the two readers came to describe the same record differently.
            if fields != "first-in-record":
                raise ValueError(
                    f"the wire file names the unknown-mark order {fields!r}, which this "
                    f"checker does not implement"
                )
        elif keyword in WIRE_KEYWORDS:
            scalars[keyword] = fields
        else:
            raise ValueError(f"the wire file has no keyword {keyword!r}")
    missing = WIRE_KEYWORDS - scalars.keys()
    if missing:
        raise ValueError(f"the wire file names no {sorted(missing)}")
    if not marks:
        raise ValueError("the wire file holds no mark")
    for scope in EXCLUSION_SCOPES:
        if not exclusions[scope]:
            raise ValueError(
                f"the wire file names no {scope!r} exclusion, so a walk cannot be told "
                f"what to skip"
            )
    return scalars, marks, exclusions


WIRE, WIRE_MARKS, EXCLUSIONS = read_wire(WIRE_FILE)

# The encoding the wire states the bytes between the framing markers are in: both
# readers decode a record by it, so a record neither can decode is refused the
# same way rather than raised over by one reader and read as a binary without a
# record by the other.
RECORD_ENCODING = WIRE["encoding"]
RECORD_START = WIRE["start"].encode(RECORD_ENCODING)
RECORD_END = WIRE["end"].encode(RECORD_ENCODING)
ENVIRONMENT_PREFIX = WIRE["prefix"]
TOOLCHAIN_VARIABLE = WIRE["environment"]
UNSET = WIRE["unset"]

# What each mark means and what clears it, so a refusal names the right remedy
# rather than a list of differences the record cannot be trusted to hold. The
# order is the wire's: a record can carry more than one mark, they are not
# equally blocking, and `blocking_mark` reports the first. `cargo-not-asked`
# comes before `resolution-not-named` because the cargo that ran a build is one of
# the readings that can name the workspace its resolution was read in, so losing
# it can lose the resolution with it — and the resolution's own remedy, a rebuild
# from the workspace, leaves a cargo that cannot be asked still unaskable.
MARKS = {locator: remedy for _, locator, remedy in WIRE_MARKS}


def mark_locator(role):
    """The locator the wire file spells for the mark with that `role`.

    A role is how the crate's code asks for a mark; this tool needs the two the
    crate writes when it cannot stand behind a record, to name them in a test and
    to set them in a record.
    """
    for name, locator, _ in WIRE_MARKS:
        if name == role:
            return locator
    raise ValueError(f"the wire file names no mark with the role {role!r}")


UNASKED_CARGO = mark_locator("unasked-cargo")
UNNAMED_RESOLUTION = mark_locator("unnamed-resolution")

# The remedy a refusal prints for a mark the wire does not name, for a line it
# cannot read, for a record whose bytes are not its encoding, and for one whose own
# bytes spell a marker of its framing. The file holds each once, so this checker
# and the crate cannot say different things about the same record — and the order
# such a mark is reported in is the file's too (`unknown-order`, which `read_wire`
# refuses where it names one this checker does not implement). The undecodable
# text is a template as well as a remedy: it spells its own encoding and the byte
# the bytes stop at, and `undecodable_text` below is what fills them in.
UNKNOWN_REMEDY = WIRE["unknown"]
MALFORMED_REMEDY = WIRE["malformed"]
UNDECODABLE_REMEDY = WIRE["undecodable"]
UNFRAMED_REMEDY = WIRE["unframed"]


def undecodable_text(offset):
    """The wire's own text for a record whose bytes are not its encoding.

    The file's remedy with the encoding and the byte the bytes stop being it at
    filled in, so rewording the refusal — including where its offset stands — is an
    edit to the file rather than one in each of two readers. The crate fills the
    same text in the same way (`Wire::undecodable`).
    """
    return UNDECODABLE_REMEDY.replace("{encoding}", RECORD_ENCODING).replace(
        "{byte}", str(offset)
    )

# The length and the alphabet of a hash, which is what every input that is there
# is recorded as: a line whose content is not one names no input. See `classify`.
HASH_LENGTH, HASH_ALPHABET = WIRE["hash"]

# The directory cargo writes a build's artifacts into, whose files are outside
# the record for the reason the record documents: their content comes from the
# build script, which the record does name.
GENERATED_DIRECTORY = "target"

# The directory names a walk skips, from the wire file's own `exclusion` lines:
# `root` names at a package root alone (where cargo looks for a target directory of
# that name) and `anywhere` names at every depth (state a build writes, installs or
# keeps rather than compiles a source from). The rule is the file's, so this
# checker excuses exactly the reads the walk does not name — and a name in neither
# list, `dist` among them, is walked: measured, a `mod dist;` compiling
# `src/dist/mod.rs` was dropped from the record with both readers reporting the
# binary current.
EXCLUDED_AT_ROOT = set(EXCLUSIONS["root"])
EXCLUDED_ANYWHERE = set(EXCLUSIONS["anywhere"])

# The configuration files cargo reads for a build of a package, relative to a
# directory it looks in for them.
CONFIGURATION_FILES = (".cargo/config.toml", ".cargo/config", "rust-toolchain.toml", "rust-toolchain")

# The dependency kinds that put a package into a binary's build closure. A dev
# edge does not: nothing a binary compiles comes from one.
BUILD_EDGES = (None, "build")

# The target kinds a binary can be built from, as opposed to the test, example
# and bench targets that share their naming and read their own inputs.
LIBRARY_KINDS = {"lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"}


class Record:
    """What one record says, read by the wire's own rule (`classify`).

    ``inputs`` maps an input's locator to the content the record spells for it;
    ``named`` and ``unknown`` are the marks it carries — the locators the wire
    names, and the ones it does not — each in the order the wire reports one in;
    and ``malformed`` is the first line the wire reads as none of them, or None,
    in which case the record cannot be read at all.
    """

    def __init__(self, inputs, named, unknown, malformed):
        self.inputs = inputs
        self.named = named
        self.unknown = unknown
        self.malformed = malformed


def classify(line):
    """What one record line is, by the rule the wire file states.

    Returns ``(kind, locator, content)`` with the kind ``"named"`` (a mark the
    wire names), ``"unknown"`` (a mark it does not), ``"input"`` or
    ``"malformed"``. The crate implements the same text (`Wire::line`), so a shape
    cannot be one thing to this checker and another to the crate that wrote the
    record; what each kind is, and the order the kinds are tried in, is stated in
    the file rather than here.
    """
    if "\r" in line:
        return ("malformed", None, None)
    content, separator, locator = line.partition("\t")
    if not separator or not locator:
        return ("malformed", None, None)
    if locator in MARKS:
        return ("named", locator, None)
    if locator.startswith(ENVIRONMENT_PREFIX):
        return ("input", locator, content)
    if content == UNSET:
        return ("unknown", locator, None)
    if is_hash(content):
        return ("input", locator, content)
    return ("malformed", None, None)


def is_hash(content):
    """Whether a content is the hash a record gives an input: the length and the
    alphabet the wire file spells, so neither reader decides that for itself."""
    return len(content) == HASH_LENGTH and all(c in HASH_ALPHABET for c in content)


class UndecodableRecord(Exception):
    """A binary's framed record bytes are not the encoding the wire states.

    Raised rather than returned as ``None``, because a record that cannot be
    decoded is not a binary without one: the crate refuses such a binary, and this
    checker has to refuse it the same way. ``offset`` is where the bytes stop being
    that encoding, counted from the start of the record, which `undecodable_text`
    spells into the wire's own text for it.
    """

    def __init__(self, offset):
        super().__init__(offset)
        self.offset = offset


class UnframedRecord(Exception):
    """A binary's framed record bytes spell a marker of their own framing.

    Raised rather than read up to the first marker met, which would take a record
    missing every line after it for the record: a line a build writes spells
    neither marker, so where this record ends cannot be read from its bytes, and
    the crate refuses such a record too.
    """


def embedded_record(blob):
    """The record a binary carries, decoded by the encoding the wire states.

    ``None`` for a binary carrying no record — no start marker, or no end marker
    that ends one of the record's lines — `UnframedRecord` where the bytes framed
    hold a marker of their own, and `UndecodableRecord` where they are not that
    encoding. Both the framing and the encoding are the file's, so this reader
    takes a record out of a binary exactly as the crate does and neither decides
    for itself where a record begins, where it ends, or what its bytes are.
    """
    start = blob.find(RECORD_START)
    if start < 0:
        return None
    start += len(RECORD_START)
    # The end marker ends the record only where it follows one of its lines, with
    # the LF that line ends with: a marker the record's own bytes spell does not
    # end it, and is refused below rather than cutting the record short.
    end = blob.find(b"\n" + RECORD_END, start)
    if end < 0:
        return None
    framed = blob[start : end + 1]
    for marker in (RECORD_START, RECORD_END):
        if marker in framed:
            raise UnframedRecord()
    try:
        return framed.decode(RECORD_ENCODING)
    except UnicodeDecodeError as error:
        raise UndecodableRecord(error.start) from None


def read_record(binary):
    """The record a binary carries, as a `Record` — or None.

    ``None`` for a binary with no record — which is a failure of its own rather
    than an empty record, and what bytes no end marker closes one of a record's
    lines frame — plus `UnframedRecord` and `UndecodableRecord` for one the wire
    frames but cannot read. The record is divided into lines the way the wire
    file states — at each LF, the last line ending with one — and every line is
    read by its rule, so what the crate reads as a mark this checker reads as a
    mark too, and a blank line is refused rather than passed over.
    """
    text = embedded_record(Path(binary).read_bytes())
    if text is None:
        return None
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    record = Record({}, [], [], None)
    for line in lines:
        kind, locator, content = classify(line)
        if kind == "malformed":
            if record.malformed is None:
                record.malformed = line
        elif kind == "input":
            record.inputs[locator] = content
        elif kind == "named":
            record.named.append(locator)
        else:
            record.unknown.append(locator)
    return record


def content_hash(path):
    """The sha256 a record spells for a readable file, or the wire's ``unset``."""
    try:
        return hashlib.sha256(Path(path).read_bytes()).hexdigest()
    except OSError:
        return UNSET


def parse_dep_info(text):
    """Every path one dep-info file's records name.

    The format is rustc's makefile-ish one: a target, a colon, then the paths
    it depends on, escaped and continued across lines — and cargo appends
    further records whose targets are workspace-relative paths, so a reader
    that only understood "the first target" would take those for dependencies.
    Comment lines (cargo appends ``# env-dep:…``) are not dependencies, and a
    trailing colon is the record separator rather than part of a path.
    """
    lines = [line for line in text.splitlines() if not line.startswith("#")]
    joined = " ".join(line.rstrip("\\") for line in lines)
    return [token.rstrip(":") for token in shlex.split(joined) if token.rstrip(":")]


def cargo_metadata(workspace):
    """Every package cargo knows, resolved, from ``cargo metadata``."""
    completed = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=workspace,
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(completed.stdout)


def closure_package_ids(metadata, package_name):
    """The packages a build of `package_name` compiles: normal and build edges.

    Returns the ids of the package itself and of everything it reaches. A
    registry dependency is included: its own sources are outside the record by
    design, but its *build script* can read a workspace file, and the caller
    compares any that it does.
    """
    by_name = {}
    for package in metadata["packages"]:
        by_name.setdefault(package["name"], package["id"])
    if package_name not in by_name:
        raise SystemExit(f"no package named {package_name} in this workspace")
    edges = {}
    for node in metadata["resolve"]["nodes"]:
        edges[node["id"]] = [
            dependency["pkg"]
            for dependency in node["deps"]
            if any(kind["kind"] in BUILD_EDGES for kind in dependency["dep_kinds"])
        ]
    reached = set()
    pending = [by_name[package_name]]
    while pending:
        package = pending.pop()
        if package in reached:
            continue
        reached.add(package)
        pending.extend(edges.get(package, []))
    return reached


def unit_names(metadata, package_ids):
    """The crate names of the units a build compiles for those packages.

    Only the targets a binary can be built from: a library, a binary or a
    proc-macro, spelled as cargo names their dep-info files (the crate name,
    underlined). Test, example and bench targets are left out although their
    dep-info shares the naming shape, because they read files the record
    excludes on purpose — a test target's ``tests/`` inputs and the fixtures
    only its own code compiles.
    """
    names = set()
    for package in metadata["packages"]:
        if package["id"] not in package_ids:
            continue
        for target in package["targets"]:
            kinds = set(target["kind"])
            if kinds & LIBRARY_KINDS or "bin" in kinds:
                names.add(target["name"].replace("-", "_"))
    return names


def unit_dep_info(target_dir, metadata, package_ids):
    """The dep-info files of the units that produce one binary's artifacts.

    A unit's dep-info is named after the crate it compiles, with the unit's own
    metadata hash appended: ``symbiote_sandbox_launch-<hash>.d``, whose records
    name artifacts spelled ``symbiote-sandbox-launch-<hash>``. The file name is
    the unit's identity here — it is the crate name as cargo spells it, with
    underscores, and it does not depend on which record the unit happened to
    write first. A build script's own dep-info sits under
    ``build/<package>-<hash>`` and is taken by directory, since its crate name
    is cargo's own ``build_script_build``.
    """
    names = unit_names(metadata, package_ids)
    found = {}
    deps = Path(target_dir) / "deps"
    for dep_info in sorted(deps.glob("*.d")) if deps.is_dir() else []:
        unit = dep_info.name[: -len(".d")].rsplit("-", 1)[0]
        if unit in names:
            found[dep_info] = unit
    build = Path(target_dir) / "build"
    for package in metadata["packages"]:
        if package["id"] not in package_ids:
            continue
        for directory in sorted(build.glob(f"{package['name']}-*")) if build.is_dir() else []:
            for dep_info in sorted(directory.glob("*.d")):
                found[dep_info] = dep_info.name
    return found


def workspace_reads(dep_info_files, workspace):
    """Every workspace file those units read, and which dep-info named it.

    A registry or sysroot source is outside the workspace and so outside what
    the record can name; a file the build generates under the target directory
    is outside it for the reason the record documents (the build script that
    writes it is inside it).
    """
    reads = {}
    target_prefix = Path(workspace).resolve() / GENERATED_DIRECTORY
    for dep_info, _ in dep_info_files.items():
        for path in parse_dep_info(dep_info.read_text(errors="replace")):
            resolved = Path(path)
            if not resolved.is_absolute():
                resolved = Path(workspace) / resolved
            resolved = Path(os.path.normpath(resolved))
            if target_prefix in resolved.parents or resolved == target_prefix:
                continue
            if Path(workspace) not in resolved.parents:
                continue
            reads.setdefault(resolved, dep_info)
    return reads


def blocking_mark(record):
    """The locator of the mark a rebuild has to clear first, or None.

    A record can carry more than one mark, and the wire file holds the order: a
    mark `MARKS` knows comes before one it does not, in the order `MARKS` spells
    them, and where none is known the order is the wire's other one, the record's
    own — the first line it spells, which ``read_record`` kept. `check` reads the
    locator this returns against `MARKS`, so a mark whose meaning this version
    does not know is still refused, with the line's own locator named.
    """
    for known in MARKS:
        if known in record.named:
            return known
    return record.unknown[0] if record.unknown else None


def recorded_files(record, base):
    """The file inputs a record names, resolved against the record's base.

    A relative locator is relative to the workspace the record was written
    against — the driven package's own nearest root, which is what `check`
    passes — not to whichever workspace this check happens to be invoked in,
    since the two differ for a package that sits below a nearer `[workspace]`
    manifest than the root its build used.
    """
    files = {}
    for locator, hash in record.inputs.items():
        if locator.startswith(ENVIRONMENT_PREFIX):
            continue
        path = Path(locator)
        if not path.is_absolute():
            path = Path(base) / path
        files[Path(os.path.normpath(path))] = hash
    return files


def table_name(line):
    """The name inside a table header, or None when the line is not one.

    TOML allows whitespace inside the brackets and a comment after them, so
    `[ workspace ]` and `[workspace] # the members` both name the same table as
    `[workspace]`, and a line that is a key is not a header at all.
    """
    line = line.split("#", 1)[0].strip()
    if not (line.startswith("[") and line.endswith("]")):
        return None
    return "".join(line[1:-1].split())


def declares_workspace(text):
    """Whether a manifest declares a workspace of its own."""
    return any(table_name(line) == "workspace" for line in text.splitlines())


def manifest_text(path):
    """A manifest's text, or empty when it cannot be read."""
    try:
        return Path(path).read_text(errors="replace")
    except OSError:
        return ""


def string_value(value):
    """The content of a TOML string scalar — `"…"` or `'…'` — or None.

    A comment or whitespace after the closing quote is ignored. A `workspace` a
    manifest does not spell as a string is not a path this can follow, and
    leaving it unrecorded is the safe direction where the ancestor roots are
    still candidates.
    """
    value = value.strip()
    if not value or value[0] not in "\"'":
        return None
    quote = value[0]
    escaped = False
    for index, character in enumerate(value[1:], start=1):
        if escaped:
            escaped = False
        elif character == "\\" and quote == '"':
            escaped = True
        elif character == quote:
            return value[1:index]
    return None


def package_workspace(manifest_path):
    """The `workspace = "..."` a manifest's `[package]` table names, or None.

    A package that names its own workspace root rather than inheriting the one
    above it. Cargo spells it as a plain key — `workspace = "../.."` — so this
    reads a scalar.
    """
    table = False
    for line in manifest_text(manifest_path).splitlines():
        name = table_name(line)
        if name is not None:
            table = name == "package"
            continue
        if not table:
            continue
        key, separator, value = line.partition("=")
        if separator and key.strip() == "workspace":
            return string_value(value)
    return None


def nearest_workspace_root(directory):
    """The root a record's relative locators are spelled against, or None.

    The root the package names with `package.workspace`, else the nearest
    ancestor manifest declaring `[workspace]`. A name first, because that is the
    root cargo reads: measured, a package whose manifest says
    `workspace = "../root"` resolves to the named root even where an ancestor
    manifest declares `[workspace]`, and the ancestor is then not parsed at all.
    The same rule the record's own walk applies for its base.
    """
    package = Path(directory).resolve()
    target = package_workspace(package / "Cargo.toml")
    if target is not None:
        return Path(os.path.normpath(package / target))
    current = package
    while True:
        manifest = current / "Cargo.toml"
        if manifest.is_file() and declares_workspace(manifest_text(manifest)):
            return current
        if current.parent == current:
            return None
        current = current.parent


def workspace_roots(directory):
    """Every workspace root cargo can resolve for the package at `directory`.

    The one root a `package.workspace` key names, or else each ancestor manifest
    that declares `[workspace]` and the package's own directory.

    A named root alone, because a name settles it: measured, cargo resolves such
    a package to the named directory even where an ancestor manifest declares
    `[workspace]`, the named root is where an inherited `workspace = true` path
    is read from, and a build whose workspace both nests the package and claims
    it as a member stops with `member of the wrong workspace`. So no ancestor of
    a package that names its workspace is a root a build of it can read.

    Without a name, not the nearest ancestor alone, because that is not cargo's
    rule: an invoked
    workspace lists members that may sit below another `[workspace]` manifest,
    and then *the invoked root* is the one whose `[workspace.package]` values a
    member inherits — the manifest passed on the way down is never loaded. A
    package its workspace `exclude`s is a workspace of one — measured, cargo
    reads the excluding manifest, walks past it, and resolves the package to
    itself — which is why its own directory is a root whether or not anything
    above it excludes it. This is the superset of what cargo can read for a
    package, and it is the same rule the record's own walk applies; naming an
    input a build did not read costs a needless refusal, and the opposite costs
    the guarantee.
    """
    package = Path(directory).resolve()
    target = package_workspace(package / "Cargo.toml")
    if target is not None:
        return [Path(os.path.normpath(package / target))]
    roots = [package]
    current = package
    while True:
        manifest = current / "Cargo.toml"
        if manifest.is_file() and declares_workspace(manifest_text(manifest)):
            roots.append(current)
        if current.parent == current:
            break
        current = current.parent
    return sorted(set(roots))


def cargo_inputs(metadata, package_ids):
    """The build inputs cargo itself reads for those packages.

    The manifest of each closure package, and every workspace root cargo can
    resolve for it — a manifest and its lockfile sit above every package, where
    no walk over package directories reaches them. The workspace this check was
    invoked in is required as well, since its `[workspace.package]` table is
    what every member of it inherits. Only local packages: a registry
    dependency's sources are outside the record by design, pinned by the
    lockfile it names.
    """
    inputs = set()
    for package in metadata["packages"]:
        if package["id"] not in package_ids or package["source"] is not None:
            continue
        manifest = Path(package["manifest_path"])
        inputs.add(manifest)
        for root in workspace_roots(manifest.parent):
            inputs.add(root / "Cargo.toml")
            inputs.add(root / "Cargo.lock")
    invoked = Path(metadata["workspace_root"])
    inputs.add(invoked / "Cargo.toml")
    inputs.add(invoked / "Cargo.lock")
    return sorted(inputs)


def excluded(path, package_dirs):
    """Whether the record excludes this path by the wire file's own rule.

    Under a closure package, the names the file skips at every depth are outside
    the walk wherever they sit and the names it skips at a package root alone are
    outside it there alone, which is the only place cargo looks for a target
    directory of that name. Those units read files the record does not name on
    purpose, so a comparison that ignored the rule would fail on them; a directory
    that merely carries one of those names deeper in the package is not one of
    them, and a record that left it out is short rather than excused.

    A skipped directory bounds the walk rather than the build, so this excuses
    only what no declaration names: a file the compiler finds in one by a module's
    own name — measured, `mod target;` compiles `src/target/mod.rs` — is recorded
    through the declaration that reaches it, and where a record leaves one out the
    refusal is the one above rather than a silence here.
    """
    for directory in package_dirs:
        if directory in path.parents:
            relative = path.relative_to(directory)
            if any(part in EXCLUDED_ANYWHERE for part in relative.parts):
                return True
            if relative.parts and relative.parts[0] in EXCLUDED_AT_ROOT:
                return True
    return False


def configuration_candidates(workspace, package_dirs):
    """Every configuration file a build of a closure package could read."""
    directories = set()
    for start in list(package_dirs) + [Path(workspace).resolve()]:
        directory = start.resolve()
        while True:
            directories.add(directory)
            if directory.parent == directory:
                break
            directory = directory.parent
    cargo_home = os.environ.get("CARGO_HOME")
    if cargo_home:
        directories.add(Path(cargo_home))
    return sorted(directory / name for directory in directories for name in CONFIGURATION_FILES)


def check(workspace, target_dir, binaries, metadata):
    """Every problem found: a read input the record does not name, a
    configuration file it does not hold or holds stale, or a target directory
    holding no dep-info for a unit of the driven package, where nothing was
    measured to compare the record with."""
    problems = []
    summaries = []
    for package_name, binary_name in binaries:
        binary = Path(target_dir) / binary_name
        if not binary.is_file():
            problems.append(f"{binary} does not exist: build it before checking its record")
            continue
        try:
            record = read_record(binary)
        except UnframedRecord:
            # The wire states that a record's own bytes hold neither marker, so a
            # record spelling one is one no build wrote: where it ends cannot be
            # read from its bytes, and reading it up to the first marker met would
            # take a record missing every line after it for the record.
            problems.append(f"{binary_name}: {UNFRAMED_REMEDY}")
            continue
        except UndecodableRecord as unreadable:
            # The wire frames the record and states its encoding, so a record
            # whose bytes are not that encoding is refused rather than raised over
            # — the same refusal the crate reaches, in the file's own text, with
            # the byte it stops at filled into that text rather than spelled here.
            problems.append(f"{binary_name}: {undecodable_text(unreadable.offset)}")
            continue
        if record is None:
            problems.append(f"{binary} carries no record, so it cannot be checked")
            continue
        if record.malformed is not None:
            # The wire reads neither what the record holds nor that it cannot be
            # stood behind, so there is nothing here to compare: the line is named
            # rather than taken for a mark or for an input the record did not mean
            # to hold.
            problems.append(
                f"{binary_name}: {MALFORMED_REMEDY} The line is {record.malformed!r}."
            )
            continue
        mark = blocking_mark(record)
        if mark is not None:
            # The mark names the fact the build could not establish, so the refusal
            # says that and what clears it rather than naming differences the record
            # cannot be trusted to hold.
            problems.append(f"{binary_name}: {MARKS.get(mark, UNKNOWN_REMEDY)} (mark: {mark})")
            continue
        package_ids = closure_package_ids(metadata, package_name)
        package_dirs = [
            Path(package["manifest_path"]).parent
            for package in metadata["packages"]
            if package["id"] in package_ids
            and Path(package["manifest_path"]).parent.is_relative_to(workspace)
        ]
        # A record's relative locators are spelled against the workspace its own
        # build resolved, which is the driven package's nearest `[workspace]`
        # ancestor — not necessarily the workspace this check is invoked in.
        driven = next(
            (p for p in metadata["packages"] if p["name"] == package_name), None
        )
        dep_info = unit_dep_info(target_dir, metadata, package_ids)
        # The dep-info of the driven package's own units is what says this target
        # directory is the one its build wrote to, and the comparison below rests
        # on it whole. Measured: against a target directory holding no dep-info at
        # all — as one does after `cargo clean` — the check reported OK having read
        # 0 units and 0 inputs, and a dependency's units alone passed the same way,
        # so a record nothing was compared with was called complete rather than
        # unmeasured.
        own = unit_names(metadata, {driven["id"]}) if driven is not None else set()
        if not any(unit in own for unit in dep_info.values()):
            problems.append(
                f"{binary_name}: {target_dir} holds no dep-info for any unit of "
                f"{package_name}, so nothing here measured what this binary's build read "
                f"— build it before checking its record"
            )
            continue
        reads = workspace_reads(dep_info, workspace)
        base = workspace
        if driven is not None:
            base = nearest_workspace_root(Path(driven["manifest_path"]).parent) or workspace
        recorded = recorded_files(record, base)
        missing = sorted(
            (path, source)
            for path, source in reads.items()
            if path not in recorded and not excluded(path, package_dirs)
        )
        for path, source in missing:
            problems.append(
                f"{binary_name}: the record does not name {path}, which {source} says was read"
            )

        # The inputs cargo reads rather than rustc, which no dep-info above
        # names: each closure package's manifest and its own workspace root's
        # manifest and lockfile. Checked from `cargo metadata` and the tree, not
        # from the record's walk.
        cargo = cargo_inputs(metadata, package_ids)
        cargo_missing = [
            path for path in cargo if path.is_file() and Path(os.path.normpath(path)) not in recorded
        ]
        for path in cargo_missing:
            problems.append(
                f"{binary_name}: {path} is read by cargo to build this binary, but the "
                f"record does not name it"
            )

        # The effective configuration: what the record holds must be what is
        # there now, and what is there now must be in the record.
        if record.inputs.get(f"{ENVIRONMENT_PREFIX}{TOOLCHAIN_VARIABLE}", UNSET) == UNSET:
            problems.append(
                f"{binary_name}: the record names no {ENVIRONMENT_PREFIX}"
                f"{TOOLCHAIN_VARIABLE}, so the toolchain that built it is not pinned"
            )
        for candidate in configuration_candidates(workspace, package_dirs):
            recorded_hash = recorded.get(Path(os.path.normpath(candidate)))
            if candidate.is_file():
                if recorded_hash is None:
                    problems.append(
                        f"{binary_name}: {candidate} exists and configures the build, but the "
                        f"record does not name it"
                    )
                elif recorded_hash != content_hash(candidate):
                    problems.append(f"{binary_name}: {candidate} is not the content recorded")
            elif recorded_hash is not None:
                problems.append(f"{binary_name}: {candidate} is recorded but no longer exists")
        existing = [path for path in cargo if path.is_file()]
        summaries.append(
            f"{binary_name}: {len(reads)} workspace inputs read by {len(dep_info)} units, "
            f"{len(recorded)} named by the record, {len(missing)} unnamed; "
            f"{len(existing)} inputs cargo read, {len(cargo_missing)} unnamed"
        )
    return problems, summaries


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--binary",
        action="append",
        default=[],
        metavar="PACKAGE:NAME",
        help="a driven binary, by the package that builds it and its artifact name",
    )
    parser.add_argument("--workspace", default=".", help="the workspace root (default: .)")
    parser.add_argument("--target-dir", default="target/debug", help="where cargo wrote the debug profile")
    arguments = parser.parse_args()

    binaries = []
    for entry in arguments.binary:
        package_name, separator, binary_name = entry.partition(":")
        if not separator or not package_name or not binary_name:
            parser.error(f"--binary wants PACKAGE:NAME, not {entry!r}")
        binaries.append((package_name, binary_name))
    if not binaries:
        parser.error("name at least one --binary")

    workspace = Path(arguments.workspace).resolve()
    metadata = cargo_metadata(workspace)
    problems, summaries = check(workspace, Path(arguments.target_dir), binaries, metadata)
    for summary in summaries:
        print(summary)
    for problem in problems:
        print(f"PROBLEM: {problem}", file=sys.stderr)
    if problems:
        print(
            f"FAIL: {len(problems)} problem(s) — a driven binary's record does not cover the "
            f"inputs its own build read, or the configuration that built it",
            file=sys.stderr,
        )
        return 1
    print(
        "OK: every workspace input the compiler read, and every input cargo read to build "
        "it, is named by the record that drove it"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
