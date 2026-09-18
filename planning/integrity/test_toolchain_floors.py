"""Hold every toolchain floor a job names against the toolchains that job runs.

A crate states the Rust it supports in its manifest — `[package] rust-version`, or
`[workspace.package]` where it inherits it — and the job that builds it states the toolchains
it runs. For the two spikes nothing related them: dropping or retargeting an `1.85.0` leg,
moving a crate's declared floor in its manifest alone, or losing the leg a job's own steps are
gated on (`if: matrix.toolchain == 'stable'`, which leaves those steps unrun) left every case
green. The workspace's own pair is `test_handoff.py`'s, because the handoff names that minimum
itself: a document's claim, not a job's, and a job naming no manifest — the `--workspace` jobs
— is that case's subject.

Both facts are read by `toolchains.py` for both rules, so a reformat is one file's business.
What nothing here decides is whether a crate *compiles* on the floor a leg names: a leg is a
promise to run, and only CI answers it.
"""
from __future__ import annotations

import pathlib
import tempfile
import unittest

from toolchains import (
    ROOT,
    floor,
    gated,
    legs,
    manifests,
    pairs,
    version,
    workspace_jobs,
)


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
            for leg in gated(block):
                with self.subTest(workflow=workflow, job=job, leg=leg):
                    self.assertIn(leg, legs(block),
                                  f"{workflow}: the job {job} gates steps on the {leg} leg "
                                  f"and runs {legs(block)}, so those steps would never run")


class EverySpellingOfOneStatement(unittest.TestCase):
    """Each state below was measured against a reader that read it wrong before it was held
    here: a crate named only as `--manifest-path=X` was not found at all, a floor written
    `rust-version="1.85.0"` read as none, an inherited floor came from the wrong manifest's
    table, a matrix written as a block sequence read as running no toolchain, a commented
    table header read as no table, one step's directory decided every other step, a templated
    one silently meant the repository root, a step installing a toolchain beside a matrix hid
    it, and a step's command written under `run: |` or split across lines with a trailing
    backslash was read as proving nothing at all. The last case here drives those two one
    level out: the live workflow contains neither shape, which is why they are written in the
    case rather than taken from the tree.
    """

    def test_a_crate_is_read_however_its_path_is_spelled(self):
        for command in ("cargo test --manifest-path src-tauri/Cargo.toml",
                        "cargo test --locked --manifest-path=src-tauri/Cargo.toml",
                        "cargo test --manifest-path 'src-tauri/Cargo.toml'"):
            with self.subTest(command=command):
                block = f"    steps:\n      - run: {command}\n"
                self.assertEqual(manifests("w.yml", "job", block),
                                 [ROOT / "src-tauri/Cargo.toml"])

    def test_a_manifest_is_read_in_the_directory_its_own_step_runs_in(self):
        """The fixture's own shape: the job's default (written after its steps, since no
        order of a job's own keys decides this), a step's own override, a comment between
        the steps, a step written as a flow mapping, and the one template GitHub resolves to
        the workspace; a step naming its crate through anything else is refused by name.
        """
        job = ("    steps:\n"
               "      - run: cargo test --manifest-path src-tauri/Cargo.toml\n"
               "      - working-directory: crates/examples\n"
               "        run: cargo test --manifest-path=one/Cargo.toml\n"
               "      - working-directory: ${{ github.workspace }}\n"
               "        run: cargo test --manifest-path two/Cargo.toml\n"
               "      - uses: actions/checkout@v4\n"
               "      -\n"
               "        working-directory: ${{ github.workspace }}/crates\n"
               "        run: cargo test --manifest-path tree/Cargo.toml\n"
               "      # a comment between the steps names no step and ends none\n"
               "      - {run: cargo test --manifest-path four/Cargo.toml}\n"
               "    defaults:\n      run:\n        working-directory: spikes/linux-shell\n")
        self.assertEqual(manifests("w.yml", "job", job),
                         [ROOT / "spikes/linux-shell/src-tauri/Cargo.toml",
                          ROOT / "crates/examples/one/Cargo.toml",
                          ROOT / "two/Cargo.toml",
                          ROOT / "crates/tree/Cargo.toml",
                          ROOT / "spikes/linux-shell/four/Cargo.toml"])
        for refused in ("    steps:\n      - working-directory: ${{ runner.temp }}/x\n"
                        "        run: cargo test --manifest-path a/Cargo.toml\n",
                        "    steps:\n      - run: cargo test --manifest-path ${{ matrix.crate }}"
                        "/Cargo.toml\n",
                        "    steps:\n      - run: cargo test --manifest-path \"$DIR/Cargo.toml\"\n"):
            with self.subTest(refused=refused):
                with self.assertRaisesRegex(AssertionError, "cannot resolve"):
                    manifests("w.yml", "job", refused)

    def test_a_floor_is_read_from_the_table_that_declares_it(self):
        """Written in any spelling, declared under a header with a comment or inner spaces,
        and inherited from `[workspace.package]` alone.
        """
        with tempfile.TemporaryDirectory() as directory:
            nested = pathlib.Path(directory) / "nested"
            (nested / "member").mkdir(parents=True)
            nested_manifest = nested / "Cargo.toml"
            member = nested / "member" / "Cargo.toml"
            # the workspace root is also a package, whose own floor is not the member's
            root = ('[package]  # the root is a crate too\nname = "nested"\n'
                    'version = "0.1.0"\nrust-version = "1.99"\n'
                    '\n[workspace]\nmembers = ["member"]\n'
                    '\n[ workspace.package ]  # what members inherit\nrust-version = "1.88"\n')
            nested_manifest.write_text(root)
            for header, spelling, declared in (("[package]", 'rust-version = "1.85.0"', "1.85.0"),
                                               ("[package]  # the crate", 'rust-version="1.85.0"',
                                                "1.85.0"),
                                               ("[ package ]", "rust-version  =  '1.85'", "1.85"),
                                               ("[package]",
                                                'rust-version = "1.85" # pinned', "1.85")):
                with self.subTest(declared=spelling, header=header):
                    member.write_text(f'{header}\nname = "member"\n{spelling}\n')
                    self.assertEqual(floor(member), declared)
            with self.subTest(inherited="[workspace.package]"):
                member.write_text('[package]\nname = "member"\nrust-version.workspace = true\n')
                self.assertEqual(floor(member), "1.88",
                                 "a crate inherits its own workspace's [workspace.package], "
                                 "not that workspace root's own [package] floor")
            with self.subTest(inherited="a root the crate names itself"):
                inner = nested / "inner"
                (inner / "pinned").mkdir(parents=True)
                (inner / "Cargo.toml").write_text('[workspace]\nmembers = []\n\n'
                                                   '[workspace.package]\nrust-version = "1.77"\n')
                pinned = inner / "pinned" / "Cargo.toml"
                pinned.write_text('[package]\nname = "pinned"\nworkspace = "../.."\n'
                                  'rust-version.workspace = true\n')
                self.assertEqual(floor(pinned), "1.88",
                                 "cargo honours the root a crate names itself over the "
                                 "nested [workspace] above it")
            with self.subTest(inherited="no [workspace.package] floor"):
                nested_manifest.write_text(root.replace('[ workspace.package ]  # what members '
                                                        'inherit\nrust-version = "1.88"\n', ""))
                self.assertIsNone(floor(member),
                                  "cargo refuses a crate whose workspace states no floor in "
                                  "[workspace.package], so there is nothing to hold it against")
            with self.subTest(missing="a manifest this tree does not hold"):
                with self.assertRaisesRegex(AssertionError, "is not a manifest this tree holds"):
                    floor(nested / "member" / "Ghost.toml")

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
                # a block sequence may sit at its key's own indentation
                (matrix + "        toolchain:\n        - \"1.85.0\"\n        - stable\n",
                 ["1.85.0", "stable"]),
                (step + "          toolchain: stable\n", ["stable"]),
                (step + "          toolchain: ${{ matrix.toolchain }}\n", []),
                # a step installing one toolchain beside a matrix: the job runs both
                (matrix + '        toolchain: ["1.85.0", stable]\n' + step
                 + "          toolchain: nightly\n", ["1.85.0", "stable", "nightly"]),
                # a flow sequence may span lines, and a block scalar states one value
                (matrix + '        toolchain: [\n          "1.85.0",\n          stable,\n        ]\n',
                 ["1.85.0", "stable"]),
                (step + "          toolchain: |\n            stable\n", ["stable"]),
                # an item whose content is on the next line states no leg of its own
                (matrix + "        toolchain:\n          -\n", []),
                # a matrix written inline as a flow mapping states the same legs
                ('    strategy:\n      matrix: {toolchain: ["1.85.0", stable]}\n',
                 ["1.85.0", "stable"]),
                ("    steps:\n      - uses: actions/checkout@v4\n", [])):
            with self.subTest(block=block):
                self.assertEqual(legs(block), running)
        with self.assertRaisesRegex(AssertionError, "never closes"):
            legs("    strategy:\n      matrix:\n        toolchain: [\n")

    def test_a_step_proves_the_workspace_however_its_command_is_written(self):
        """One command written inline, folded, under `run: |`, in a flow-mapping step, on two
        lines of a block scalar, split across lines with a trailing backslash, or with the
        step's `name` after it, is one command. Measured: with the entry's own words matched
        one at a time instead of joined, the `run: |` form finds no proving job at all, and
        before `entries` read a shell continuation the backslash form found none either —
        each reding the handoff's case for a workflow that proves exactly what it always did.
        """
        for written in ("      - run: cargo test --workspace --locked\n",
                        "      - run: |\n          cargo test --workspace --locked\n",
                        "      - run: >\n          cargo test --workspace --locked\n",
                        "      - {run: cargo test --workspace --locked}\n",
                        "      - run: |\n          cargo test\n          --workspace --locked\n",
                        "      - run: cargo test \\\n          --workspace --locked\n",
                        "      - run: cargo test \\\n          --workspace \\\n"
                        "          --locked\n",
                        "      - run: cargo test \\\n          --workspace --locked\n"
                        "        name: proofs\n",
                        "      - run: |\n          cargo test --workspace --locked\n"
                        "        name: proofs\n"):
            with self.subTest(written=written):
                self.assertEqual(len(workspace_jobs(f"jobs:\n  proofs:\n    steps:\n{written}")), 1,
                                 "a step that runs the workspace's own tests proves the workspace")
        for other in ("      - run: cargo clippy --workspace --all-targets --locked\n",
                      "      - run: cargo clippy --workspace \\\n          --all-targets\n",
                      "      - run: cargo test --locked\n"):
            with self.subTest(other=other):
                self.assertEqual(workspace_jobs(f"jobs:\n  proofs:\n    steps:\n{other}"), [],
                                 "only one entry stating `cargo test` and `--workspace` proves it")


if __name__ == "__main__":
    unittest.main()
