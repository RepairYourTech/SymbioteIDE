#!/usr/bin/env python3
"""Whether a driven binary's embedded record covers what the compiler read.

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

Two things are compared:

* **Completeness.** Every workspace file the units of the driven binary's
  closure read must be named by the record. Registry sources and generated
  files under the target directory are excluded, as the record's own
  documentation excludes them, and so are the test, example and bench
  directories, which the record excludes on purpose and whose own units read
  them.
* **Effective configuration.** The record holds `env:CARGO` and every
  configuration file cargo reads for a build of a closure package
  (`.cargo/config.toml`, `.cargo/config`, `rust-toolchain.toml`,
  `rust-toolchain`). A configuration file that exists now and is not named, or
  a named one whose content moved, is a failure — the class that cannot be
  recorded at build time without making cargo relink the stamped crates on
  every build.

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

RECORD_START = b"symbiote-source-record:["
RECORD_END = b"]symbiote-source-record"

# The environment variable a record holds, spelled `env:NAME`, which names the
# toolchain that ran the build.
ENVIRONMENT_PREFIX = "env:"
TOOLCHAIN_VARIABLE = "CARGO"

# The directory cargo writes a build's artifacts into, whose files are outside
# the record for the reason the record documents: their content comes from the
# build script, which the record does name.
GENERATED_DIRECTORY = "target"

# Directory names the record excludes under a package, whose own units read
# them: a change there must not refuse a current binary.
EXCLUDED_DIRECTORIES = {".git", "benches", "dist", "examples", "node_modules", "target", "tests"}

# The configuration files cargo reads for a build of a package, relative to a
# directory it looks in for them.
CONFIGURATION_FILES = (".cargo/config.toml", ".cargo/config", "rust-toolchain.toml", "rust-toolchain")

# The dependency kinds that put a package into a binary's build closure. A dev
# edge does not: nothing a binary compiles comes from one.
BUILD_EDGES = (None, "build")

# The target kinds a binary can be built from, as opposed to the test, example
# and bench targets that share their naming and read their own inputs.
LIBRARY_KINDS = {"lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"}


def read_record(binary):
    """The record a binary carries, as ``locator -> hash`` — or None.

    ``None`` for a binary with no record, which is a failure of its own rather
    than an empty record.
    """
    content = Path(binary).read_bytes()
    start = content.find(RECORD_START)
    if start < 0:
        return None
    start += len(RECORD_START)
    end = content.find(RECORD_END, start)
    if end < 0:
        return None
    record = {}
    for line in content[start:end].decode().splitlines():
        if not line:
            continue
        hash, _, locator = line.partition("\t")
        record[locator] = hash
    return record


def content_hash(path):
    """The sha256 a record spells for a readable file, or ``unset``."""
    try:
        return hashlib.sha256(Path(path).read_bytes()).hexdigest()
    except OSError:
        return "unset"


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


def recorded_files(record, workspace):
    """The file inputs a record names, resolved against the workspace."""
    files = {}
    for locator, hash in record.items():
        if locator.startswith(ENVIRONMENT_PREFIX):
            continue
        path = Path(locator)
        if not path.is_absolute():
            path = Path(workspace) / path
        files[Path(os.path.normpath(path))] = hash
    return files


def excluded(path, package_dirs):
    """Whether the record excludes this path by its own documented rule.

    Under a closure package, the test, example and bench targets and the build
    output are outside the walk. Those units read files the record does not
    name on purpose, so a comparison that ignored the rule would fail on them.
    """
    for directory in package_dirs:
        if directory in path.parents:
            relative = path.relative_to(directory)
            if any(part in EXCLUDED_DIRECTORIES for part in relative.parts):
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
    """Every problem found: a read input the record does not name, or a
    configuration file it does not hold, or holds stale."""
    problems = []
    summaries = []
    for package_name, binary_name in binaries:
        binary = Path(target_dir) / binary_name
        if not binary.is_file():
            problems.append(f"{binary} does not exist: build it before checking its record")
            continue
        record = read_record(binary)
        if record is None:
            problems.append(f"{binary} carries no record, so it cannot be checked")
            continue
        package_ids = closure_package_ids(metadata, package_name)
        package_dirs = [
            Path(package["manifest_path"]).parent
            for package in metadata["packages"]
            if package["id"] in package_ids
            and Path(package["manifest_path"]).parent.is_relative_to(workspace)
        ]
        dep_info = unit_dep_info(target_dir, metadata, package_ids)
        reads = workspace_reads(dep_info, workspace)
        recorded = recorded_files(record, workspace)
        missing = sorted(
            (path, source)
            for path, source in reads.items()
            if path not in recorded and not excluded(path, package_dirs)
        )
        for path, source in missing:
            problems.append(
                f"{binary_name}: the record does not name {path}, which {source} says was read"
            )

        # The effective configuration: what the record holds must be what is
        # there now, and what is there now must be in the record.
        if record.get(f"{ENVIRONMENT_PREFIX}{TOOLCHAIN_VARIABLE}", "unset") == "unset":
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
        summaries.append(
            f"{binary_name}: {len(reads)} workspace inputs read by {len(dep_info)} units, "
            f"{len(recorded)} named by the record, {len(missing)} unnamed"
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
    print("OK: every workspace input the compiler read is named by the record that drove it")
    return 0


if __name__ == "__main__":
    sys.exit(main())
