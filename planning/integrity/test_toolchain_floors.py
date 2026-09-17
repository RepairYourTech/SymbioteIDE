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
or the workspace's where the manifest inherits it (`rust-version.workspace = true`,
which every member crate does). A job naming no manifest — the workspace jobs, which
run `--workspace` — is the handoff case's subject. A crate that declares no floor has
no claim to hold and is read past; a path a job names that this tree does not hold is
refused rather than read past, since reading past it would drop the crate from the two
cases below without saying so.

What nothing here decides is whether a crate *compiles* on the floor a leg names. A leg
is a promise to run, and only CI answers it: the fixture's `1.85.0` leg answered on the
pull request that added it, compiling and passing its own test in 3m1s. Nothing here
reads an installed toolchain, and a matrix is not evidence that its legs succeed.
"""
from __future__ import annotations

import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github/workflows"
JOB = re.compile(r"^  ([A-Za-z0-9_-]+):\s*$")
NAMED = re.compile(r"--manifest-path (\S+)")
LEGS = re.compile(r"^\s+toolchain: \[([^\]]+)\]", re.M)
GATED = re.compile(r"matrix\.toolchain == '([^']+)'")
DIRECTORY = re.compile(r"^\s+working-directory: (\S+)\s*$", re.M)
FLOOR = re.compile(r'rust-version = "([^"]+)"')
INHERITED = re.compile(r"rust-version\.workspace = true")


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


def pairs() -> list[tuple[str, str, str, pathlib.Path]]:
    """(workflow, job, the job's own text, the manifest it names).

    `--manifest-path` is resolved against the job's own `working-directory`, since the
    fixture's job names `src-tauri/Cargo.toml` from inside `spikes/linux-shell`. A job
    names its crate in every cargo step it runs; that is one pair.
    """
    found: list[tuple[str, str, str, pathlib.Path]] = []
    for path in sorted(WORKFLOWS.glob("*.yml")):
        for job, block in jobs(path.read_text()).items():
            directory = DIRECTORY.search(block)
            base = ROOT / directory.group(1) if directory else ROOT
            for named in dict.fromkeys(NAMED.findall(block)):
                found.append((path.name, job, block, base / named))
    return found


def legs(block: str) -> list[str]:
    """The toolchains a job runs, as its own matrix declares them."""
    matrix = LEGS.search(block)
    return [leg.strip().strip('"') for leg in matrix.group(1).split(",")] if matrix else []


def floor(manifest: pathlib.Path) -> str | None:
    """The floor a manifest declares, or the workspace's where it inherits it."""
    text = manifest.read_text()
    declared = FLOOR.search(text)
    if declared:
        return declared.group(1)
    if INHERITED.search(text):
        return FLOOR.search((ROOT / "Cargo.toml").read_text()).group(1)
    return None


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


if __name__ == "__main__":
    unittest.main()
