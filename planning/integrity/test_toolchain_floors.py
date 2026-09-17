"""Hold every toolchain floor a job names against the matrix that job runs.

A crate states the Rust it supports in its own manifest — `[package] rust-version`, or
`[workspace.package]` where the crate inherits it — and the job that builds it states
the toolchains it runs. Those are two statements of one fact, and for the two spikes
nothing related them: dropping or retargeting an `1.85.0` leg, moving a spike's declared
floor in its manifest alone, or losing the leg a job's own steps are gated on left every
case green. (The one pair that *was* held is the workspace's, and `test_handoff.py` holds
it because `docs/engineering-handoff.md` names the minimum itself: a document's claim,
not a job's.)

What this file holds is the pair a job makes: the crate a job names by `--manifest-path`
declares a floor, and that floor must be one of the toolchains the job runs — as must
every leg the job's own steps are gated on (`if: matrix.toolchain == 'stable'`), since
losing such a leg leaves those steps unrun while the job stays green. A job naming no
manifest — the workspace jobs, which run `--workspace` — is the handoff case's subject.
Each fact is read in every spelling a run states it in, because a spelling read as
nothing is a job held by nothing: the flag as `--manifest-path PATH` or
`--manifest-path=PATH`; a floor with or without spaces around `=`, in either quote
character, in the table that declares it; and a job's toolchains as a flow list, as a
block sequence under a key with a comment after it, or as the one toolchain a job with
no matrix names on its setup step, where a `${{ … }}` template names nothing comparable.
A path this tree does not hold, and a crate declaring no floor, are refused or named by
the cases below rather than read past in silence.

What nothing here decides is whether a crate *compiles* on the floor a leg names: a leg
is a promise to run and only CI answers it. A job's own base is a further limit — it is
read from the first `working-directory` in the job's block whatever level sets it, and a
templated one is read as the repository root — left as they are because no job in this
tree names a manifest from either.
"""
from __future__ import annotations

import pathlib
import re
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github/workflows"
JOB = re.compile(r"^  ([A-Za-z0-9_-]+):\s*$")
NAMED = re.compile(r"--manifest-path[= ](\S+)")
TOOLCHAIN = re.compile(r"^(\s*)toolchain:(.*)$")
ITEM = re.compile(r"^(\s*)-\s*(\S+)\s*$")
GATED = re.compile(r"matrix\.toolchain == '([^']+)'")
DIRECTORY = re.compile(r"^\s+working-directory: (\S+)\s*$", re.M)
FLOOR = re.compile(r"^\s*rust-version\s*=\s*['\"]([^'\"]+)['\"]")
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
    fixture's job names `src-tauri/Cargo.toml` from inside `spikes/linux-shell`. Which
    line decides that, and what a templated one is read as, is the module docstring's
    limit: the first such line in the block, whatever level sets it.
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
    """The toolchains a job runs, as the job itself states them.

    A matrix states its legs as a flow list or as a block sequence, and a job with no
    matrix names one on its setup step (`with: toolchain: stable`), which is a leg like
    any other; whichever a job uses is read where it is written, with any comment after
    the key. Reading one of those forms as nothing would report a job as running none.
    """
    lines = block.splitlines()
    for index, line in enumerate(lines):
        key = TOOLCHAIN.match(line)
        if not key:
            continue
        indent, tail = len(key.group(1)), key.group(2).split("#")[0].strip()
        if tail.startswith("[") and tail.endswith("]"):
            return [leg.strip().strip("\"'") for leg in tail[1:-1].split(",")]
        if not tail:
            run: list[str] = []
            for following in lines[index + 1:]:
                item = ITEM.match(following)
                if not item or len(item.group(1)) <= indent:
                    break
                run.append(item.group(2).strip("\"'"))
            return run
        if "${{" not in tail:
            return [tail.strip("\"'")]
    return []


def workspace_of(manifest: pathlib.Path) -> pathlib.Path | None:
    """The nearest manifest above this one that declares a workspace."""
    for directory in manifest.parents:
        candidate = directory / "Cargo.toml"
        if candidate.is_file() and WORKSPACE.search(candidate.read_text()):
            return candidate
    return None


def declared_floor(text: str, table: str) -> str | None:
    """The `rust-version` one table of a manifest states, however it is written.

    Only `[package]` states a crate's own floor and only `[workspace.package]` states
    the one its members inherit, so the table a line sits in decides what it means.
    """
    inside = False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            inside = stripped == f"[{table}]"
        elif inside and (stated := FLOOR.match(line)):
            return stated.group(1)
    return None


def floor(manifest: pathlib.Path) -> str | None:
    """The floor a manifest declares, or the one its own workspace declares.

    A manifest inheriting a floor that its workspace's `[workspace.package]` does not
    state is one cargo refuses to build, so there is nothing here to hold it against -
    and the workspace root's own `[package]` floor is not the member's.
    """
    text = manifest.read_text()
    declared = declared_floor(text, "package")
    if declared:
        return declared
    if not INHERITED.search(text):
        return None
    workspace = workspace_of(manifest)
    return declared_floor(workspace.read_text(), "workspace.package") if workspace else None


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
    """The readers hold each fact however a run states it, which each state below was
    measured against before it was widened: a crate named only as `--manifest-path=X`
    was not found at all, a floor written `rust-version="1.85.0"` read as none, an
    inherited floor was read from whichever table came first in the wrong manifest, and a
    matrix written as a block sequence was read as running no toolchain.
    """

    def test_a_crate_is_read_however_its_path_is_spelled(self):
        for command in ("cargo test --manifest-path src-tauri/Cargo.toml",
                        "cargo test --locked --manifest-path=src-tauri/Cargo.toml",
                        "cargo test --manifest-path 'src-tauri/Cargo.toml'"):
            with self.subTest(command=command):
                self.assertEqual(named_in(command, ROOT), [ROOT / "src-tauri/Cargo.toml"])

    def test_a_floor_is_read_from_the_table_that_declares_it(self):
        """Written in any spelling, and inherited from `[workspace.package]` alone."""
        with tempfile.TemporaryDirectory() as directory:
            nested = pathlib.Path(directory) / "nested"
            (nested / "member").mkdir(parents=True)
            nested_manifest = nested / "Cargo.toml"
            member = nested / "member" / "Cargo.toml"
            # the workspace root is also a package, whose own floor is not the member's
            root = ('[package]\nname = "nested"\nversion = "0.1.0"\nrust-version = "1.99"\n'
                    '\n[workspace]\nmembers = ["member"]\n\n[workspace.package]\n'
                    'rust-version = "1.88"\n')
            nested_manifest.write_text(root)
            for spelling, declared in (('rust-version = "1.85.0"', "1.85.0"),
                                       ('rust-version="1.85.0"', "1.85.0"),
                                       ("rust-version  =  '1.85'", "1.85"),
                                       ('rust-version = "1.85" # pinned', "1.85")):
                with self.subTest(declared=spelling):
                    member.write_text(f'[package]\nname = "member"\n{spelling}\n')
                    self.assertEqual(floor(member), declared)
            with self.subTest(inherited="[workspace.package]"):
                member.write_text('[package]\nname = "member"\nrust-version.workspace = true\n')
                self.assertEqual(floor(member), "1.88",
                                 "a crate inherits its own workspace's [workspace.package], "
                                 "not that workspace root's own [package] floor")
            with self.subTest(inherited="no [workspace.package] floor"):
                nested_manifest.write_text(root.replace('\n[workspace.package]\n'
                                                        'rust-version = "1.88"', ""))
                self.assertIsNone(floor(member),
                                  "cargo refuses a crate whose workspace states no floor in "
                                  "[workspace.package], so there is nothing to hold it against")

    def test_a_job_runs_the_toolchain_it_states_however_it_states_it(self):
        matrix = "    strategy:\n      matrix:\n"
        step = ("    steps:\n      - uses: dtolnay/rust-toolchain@master\n"
                "        with:\n")
        for block, running in (
                (matrix + '        toolchain: ["1.85.0", stable]\n', ["1.85.0", "stable"]),
                (matrix + '        toolchain: ["1.85.0", stable]  # the legs\n',
                 ["1.85.0", "stable"]),
                (matrix + "        toolchain:\n          - '1.85.0'\n          - stable\n"
                 "    steps:\n      - run: cargo test\n", ["1.85.0", "stable"]),
                (matrix + "        toolchain:  # the legs\n          - \"1.85.0\"\n"
                 "          - stable\n", ["1.85.0", "stable"]),
                (step + "          toolchain: stable\n", ["stable"]),
                (step + "          toolchain: ${{ matrix.toolchain }}\n", []),
                ("    steps:\n      - uses: actions/checkout@v4\n", [])):
            with self.subTest(block=block):
                self.assertEqual(legs(block), running)


if __name__ == "__main__":
    unittest.main()
