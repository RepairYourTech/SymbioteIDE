"""Hold every toolchain floor a job names against the matrix that job runs.

A crate states the Rust it supports in its own manifest (`rust-version`) and the job
that builds it states the toolchains it runs. Those are two statements of one fact, and
until this file nothing related them for the two spikes. Measured on `78623b3b`, with
the whole integrity suite in the balance: dropping the `1.85.0` leg from either
`linux-proofs.yml` job, retargeting that leg to `1.88.0`, raising a spike's declared
floor in its manifest alone, or lowering it, left all 172 cases green. The one pair
that was held is the workspace's, and `test_handoff.py` holds that one because the
handoff document itself names the minimum — a document's claim about the tree, not a
job's.

What this file holds instead is the pair a job makes. Every job that names a crate by
`--manifest-path` says which crate it builds, so that crate's declared floor must be
one of the toolchains the job runs — and so must every leg the job's own steps are
gated on (`if: matrix.toolchain == 'stable'`), since losing such a leg leaves those
steps unrun while the job stays green. The floor is the manifest's own `rust-version`,
or the one its *own* workspace declares where the manifest inherits it
(`rust-version.workspace = true`): the nearest `[workspace]` above it, which for the
two spikes is the spike, not the repository root. A manifest inheriting a floor no
workspace states is one cargo itself refuses to build, so it has no floor to hold.
A job naming no manifest — the workspace jobs, which run `--workspace` — is the
handoff case's subject. A crate that declares no floor has no claim to hold and is read
past; a path a job names that this tree does not hold is refused rather than read past,
since reading past it would drop the crate from the two cases below without saying so.

Each fact is read in every spelling a run may state it in: the flag as
`--manifest-path PATH` or `--manifest-path=PATH`, a floor with or without spaces around
`=` and in either quote character, and a matrix as a flow list
(`toolchain: ["1.85.0", stable]`) or as a block sequence (`toolchain:` and then one
`- 1.85.0` line per leg). A spelling read as nothing is a job held by nothing, which is
the drift this file exists to catch, so the readers below are exercised on each.

What nothing here decides is whether a crate *compiles* on the floor a leg names. A leg
is a promise to run, and only CI answers it: the fixture's `1.85.0` leg answered on the
pull request that added it, compiling and passing its own test in 3m1s. Nothing here
reads an installed toolchain, and a matrix is not evidence that its legs succeed.
"""
from __future__ import annotations

import pathlib
import re
import tempfile
import unittest
from itertools import takewhile

ROOT = pathlib.Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github/workflows"
JOB = re.compile(r"^  ([A-Za-z0-9_-]+):\s*$")
NAMED = re.compile(r"--manifest-path[= ](\S+)")
LEGS = re.compile(r"^\s+toolchain: \[([^\]]+)\]", re.M)
MATRIX = re.compile(r"^\s+toolchain:\s*$")
SEQUENCE = re.compile(r"^\s+-\s*\S+\s*$")
SCALAR = re.compile(r"^\s+toolchain:\s*(\S+)\s*$", re.M)
GATED = re.compile(r"matrix\.toolchain == '([^']+)'")
DIRECTORY = re.compile(r"^\s+working-directory: (\S+)\s*$", re.M)
FLOOR = re.compile(r"^\s*rust-version\s*=\s*['\"]([^'\"]+)['\"]", re.M)
INHERITED = re.compile(r"^\s*rust-version\.workspace\s*=\s*true", re.M)
WORKSPACE = re.compile(r"^\s*\[workspace[.\]]", re.M)


def jobs(workflow: str) -> dict[str, str]:
    """Each job of a workflow, with the text of its own block.

    A job key is indented two spaces under `jobs:` and everything a job holds is
    indented further, so the block ends where a line starts at column zero.
    """
    found: dict[str, str] = {}
    name: str | None = None
    inside = False
    for line in workflow.splitlines():
        if line.startswith("jobs:"):
            inside = True
            continue
        if not inside:
            continue
        if line and not line.startswith(" "):
            break
        key = JOB.fullmatch(line)
        if key:
            name = key.group(1)
            found[name] = ""
        elif name is not None:
            found[name] += line + "\n"
    return found


def named_in(block: str, base: pathlib.Path) -> list[pathlib.Path]:
    """The manifests a job's own text names, against the job's own directory.

    A job names its crate in every cargo step it runs; one crate is one pair.
    """
    return [base / named.strip("\"'") for named in dict.fromkeys(NAMED.findall(block))]


def pairs() -> list[tuple[str, str, str, pathlib.Path]]:
    """(workflow, job, the job's own text, the manifest it names).

    `--manifest-path` is resolved against the job's own `working-directory`, since the
    fixture's job names `src-tauri/Cargo.toml` from inside `spikes/linux-shell`.
    """
    found: list[tuple[str, str, str, pathlib.Path]] = []
    for path in sorted(WORKFLOWS.glob("*.yml")):
        for job, block in jobs(path.read_text()).items():
            directory = DIRECTORY.search(block)
            base = ROOT / directory.group(1) if directory else ROOT
            found.extend((path.name, job, block, manifest)
                         for manifest in named_in(block, base))
    return found


def legs(block: str) -> list[str]:
    """The toolchains a job runs, as the job itself declares them.

    A matrix states its legs as a flow list or as a block sequence; a job with no matrix
    names one on its setup step instead (`with: toolchain: stable`), which is a leg like
    any other. Reading only one of those forms would report a job as running none of
    them, and a template names nothing this reader can compare.
    """
    matrix = LEGS.search(block)
    if matrix:
        return [leg.strip().strip("\"'") for leg in matrix.group(1).split(",")]
    lines = block.splitlines()
    for index, line in enumerate(lines):
        if MATRIX.fullmatch(line):
            run = list(takewhile(SEQUENCE.fullmatch, lines[index + 1:]))
            return [leg.strip().lstrip("- ").strip("\"'") for leg in run]
    return [value.strip("\"'") for value in SCALAR.findall(block)
            if "${{" not in value]


def workspace_of(manifest: pathlib.Path) -> pathlib.Path | None:
    """The nearest manifest above this one that declares a workspace."""
    for directory in manifest.parents:
        candidate = directory / "Cargo.toml"
        if candidate.is_file() and WORKSPACE.search(candidate.read_text()):
            return candidate
    return None


def floor(manifest: pathlib.Path) -> str | None:
    """The floor a manifest declares, or the one its own workspace declares.

    A manifest inheriting a floor that no workspace states is one cargo refuses to
    build, so there is nothing here for a job to be held against.
    """
    text = manifest.read_text()
    declared = FLOOR.search(text)
    if declared:
        return declared.group(1)
    if not INHERITED.search(text):
        return None
    workspace = workspace_of(manifest)
    inherited = FLOOR.search(workspace.read_text()) if workspace else None
    return inherited.group(1) if inherited else None


def version(declared: str) -> str:
    """A floor as the matrix spells a toolchain: `1.85` and `1.85.0` are one version."""
    return declared if declared.count(".") == 2 else f"{declared}.0"


def relative(path: pathlib.Path) -> str:
    return str(path.relative_to(ROOT))


def floored() -> list[tuple[str, str, pathlib.Path, str, list[str]]]:
    """(workflow, job, manifest, floor, legs) for every job naming a floored crate."""
    return [(workflow, job, manifest, declared, legs(block))
            for workflow, job, block, manifest in pairs()
            if manifest.is_file() and (declared := floor(manifest))]


class DeclaredToolchainFloors(unittest.TestCase):
    def test_every_crate_a_job_names_is_a_manifest_this_tree_holds(self):
        named = pairs()
        self.assertTrue(named, "no workflow job names a crate by --manifest-path")
        for workflow, job, _block, manifest in named:
            with self.subTest(workflow=workflow, job=job):
                self.assertTrue(manifest.is_file(),
                                f"{workflow}: the job {job} names {manifest}, "
                                f"which is not a file this tree holds")

    def test_every_job_builds_the_crate_it_names_on_the_floor_that_crate_declares(self):
        held = floored()
        self.assertTrue(held, "no workflow job names a crate that declares a toolchain floor")
        for workflow, job, manifest, declared, running in held:
            with self.subTest(workflow=workflow, job=job, manifest=relative(manifest)):
                self.assertIn(version(declared), running,
                              f"{relative(manifest)} declares {declared}, so the job {job} "
                              f"that builds it must run {version(declared)}: it runs {running}")

    def test_every_job_runs_the_legs_its_own_steps_are_gated_on(self):
        for workflow, job, block, manifest in pairs():
            for gated in dict.fromkeys(GATED.findall(block)):
                with self.subTest(workflow=workflow, job=job, leg=gated):
                    self.assertIn(gated, legs(block),
                                  f"{workflow}: the job {job} gates steps on the {gated} leg "
                                  f"and runs {legs(block)}, so those steps would never run")


class EverySpellingOfOneStatement(unittest.TestCase):
    """The readers hold each fact however a run states it.

    A spelling read as nothing is silence rather than a refusal: the job is held by
    nothing, which is the drift this file exists to catch. Each state below was measured
    against the readers before they were widened — `--manifest-path=X` and
    `rust-version="1.85.0"` were read as no crate and no floor at all, an inherited
    floor was read from the repository root rather than the crate's own workspace, a
    workspace stating no floor raised `AttributeError` instead of reading as none, and a
    matrix written as a block sequence — or a job naming its one toolchain on a step —
    was read as running no toolchain at all.
    """

    def test_a_crate_is_read_however_its_path_is_spelled(self):
        for command in ("cargo test --manifest-path src-tauri/Cargo.toml",
                        "cargo test --locked --manifest-path=src-tauri/Cargo.toml",
                        "cargo test --manifest-path 'src-tauri/Cargo.toml'"):
            with self.subTest(command=command):
                self.assertEqual(named_in(command, ROOT), [ROOT / "src-tauri/Cargo.toml"])

    def test_a_floor_is_read_however_its_spacing_and_quotes_are_written(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest = pathlib.Path(directory) / "Cargo.toml"
            for text, declared in (('rust-version = "1.85.0"', "1.85.0"),
                                   ('rust-version="1.85.0"', "1.85.0"),
                                   ("rust-version  =  '1.85'", "1.85"),
                                   ('rust-version = "1.85" # pinned', "1.85")):
                with self.subTest(text=text):
                    manifest.write_text(f"[package]\n{text}\n")
                    self.assertEqual(floor(manifest), declared)

    def test_an_inherited_floor_is_read_from_the_crates_own_workspace(self):
        with tempfile.TemporaryDirectory() as directory:
            base = pathlib.Path(directory)
            (base / "Cargo.toml").write_text('[workspace]\nmembers = ["nested"]\n\n'
                                             '[workspace.package]\nrust-version = "1.90"\n')
            nested = base / "nested"
            (nested / "member").mkdir(parents=True)
            (nested / "Cargo.toml").write_text("[workspace]\n\n[workspace.package]\n"
                                               'rust-version = "1.88"\n')
            member = nested / "member" / "Cargo.toml"
            member.write_text("[package]\nrust-version.workspace = true\n")
            self.assertEqual(floor(member), "1.88",
                             "a crate inherits its own workspace's floor, not an outer one's")
            (nested / "Cargo.toml").write_text("[workspace]\n")
            self.assertIsNone(floor(member),
                              "a workspace stating no floor leaves cargo to refuse the crate; "
                              "there is no floor for a job to be held against")

    def test_a_matrix_is_read_as_a_flow_list_or_as_a_block_sequence(self):
        flow = ("    strategy:\n      matrix:\n        toolchain: [\"1.85.0\", stable]\n"
                "    steps:\n      - run: cargo test\n")
        sequence = ("    strategy:\n      matrix:\n        toolchain:\n"
                    '          - "1.85.0"\n          - stable\n'
                    "    steps:\n      - run: cargo test\n")
        for block in (flow, sequence):
            with self.subTest(block=block):
                self.assertEqual(legs(block), ["1.85.0", "stable"])
        self.assertEqual(legs("    steps:\n      - uses: actions/checkout@v4\n"), [],
                         "a job stating no toolchain at all runs no declared leg")

    def test_a_job_stating_one_toolchain_names_it_as_a_leg(self):
        block = ("    steps:\n      - uses: dtolnay/rust-toolchain@master\n"
                 "        with:\n          toolchain: stable\n"
                 "      - run: cargo test\n")
        self.assertEqual(legs(block), ["stable"],
                         "a job with no matrix runs the toolchain its own step names")
        self.assertEqual(legs(block.replace("toolchain: stable",
                                            "toolchain: ${{ matrix.toolchain }}")), [],
                         "a template names no toolchain this reader can compare")


if __name__ == "__main__":
    unittest.main()
