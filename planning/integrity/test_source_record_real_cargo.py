"""Run: python3 planning/integrity/test_source_record_real_cargo.py.

The oracle's own functions against output only cargo writes.

`test_source_record.py` builds cargo's layout by hand, and one of those fixtures
already wrote it wrong: a fingerprint's extensionless file rather than the
``<kind>-<target>.json`` cargo writes, so a test pinned a layout the real thing
did not have. Those fixtures stay — most shapes a rule has to refuse are not
things a build can cheaply be made to produce — but the facts that *are* cargo's
to decide are taken from a real build here, because a hand-written copy of them
is exactly what drifts:

* **the producing unit.** Cargo writes a unit's output into ``deps`` and gives the
  profile directory a name for that same file, so the hash of the output the
  artifact shares its file with is the unit that produced it. Measured here, the
  profile binary shares its file with exactly one output, while three dep-info
  files are spelled ``probe-<hash>.d`` — one of them another package's — so the
  hash and not the name is what identifies the evidence.
* **which package a same-named unit belongs to.** ``deps/probe-<hash>.d`` for the
  other package sits under the fingerprint directory ``other-<hash>``, which names
  ``lib-probe``: a unit no build of the driven binary reaches, and a name-only
  reading credits it to the driven package.
* **a legitimate target directory that is not called ``target``.** A build driven
  into ``<workspace>/build-output`` leaves real reads under it — the generated
  file the binary includes among them — and they are excused by the directory the
  check was *given*, which a rule reading the name ``target`` cannot do.

Nothing here reads a record: what is asserted is what cargo wrote, which the
crate's real-build harness (`crates/symbiote-source-stamp/tests/record_guard.rs`)
cannot reach because it drives the checker end to end and asserts verdicts on a
record-bearing binary. Skipped, with the reason, where no cargo can be run.
"""

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from source_record import (
    cargo_metadata,
    closure_package_ids,
    driven_unit,
    unit_dep_info,
    unit_fingerprints,
    unit_hash,
    workspace_reads,
)

# The workspace the cases build: a package whose binary includes a file its own
# build script generates (so the build reads under its target directory), a
# library of the same package (a second unit of the same crate name), and a
# package whose library carries the driven binary's crate name (a unit another
# package compiles, spelled the same way).
WORKSPACE = {
    "Cargo.toml": '[workspace]\nmembers = ["probe", "other"]\nresolver = "2"\n',
    "probe/Cargo.toml": (
        '[package]\nname = "probe"\nversion = "0.0.0"\nedition = "2021"\n'
    ),
    "probe/build.rs": (
        "fn main() {\n"
        '    let out = std::env::var("OUT_DIR").expect("OUT_DIR");\n'
        "    std::fs::write(\n"
        '        std::path::Path::new(&out).join("generated.rs"),\n'
        '        "pub const GENERATED: &str = \\"generated\\";\\n",\n'
        "    )\n"
        '    .expect("the generated source");\n'
        "}\n"
    ),
    "probe/src/main.rs": (
        'include!(concat!(env!("OUT_DIR"), "/generated.rs"));\n'
        "\n"
        "fn main() {\n"
        '    println!("{}", GENERATED);\n'
        "}\n"
    ),
    "probe/src/lib.rs": 'pub const LIBRARY: &str = "probe";\n',
    "other/Cargo.toml": (
        '[package]\nname = "other"\nversion = "0.0.0"\nedition = "2021"\n\n'
        '[lib]\nname = "probe"\npath = "src/lib.rs"\n'
    ),
    "other/src/lib.rs": 'pub const OTHER: &str = "other";\n',
}


def write_workspace(root):
    """The scratch workspace, under `root`."""
    for relative, contents in WORKSPACE.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents)


def runnable_cargo():
    """A cargo that can be run — `CARGO` first, then PATH — or `None`.

    `CARGO` is what a test run through cargo sets, and it is not proof that the
    binary it names is there: measured, `CARGO=/nonexistent` with a cargo on PATH
    made these cases fail with `FileNotFoundError` out of `setUpClass` rather than
    skip, which is the one thing a case that needs a real build must not do where
    there is none to be had. So the cargo is asked for its version before
    anything is built, and a candidate that cannot answer is passed over.
    """
    for candidate in (os.environ.get("CARGO"), shutil.which("cargo")):
        if not candidate:
            continue
        try:
            completed = subprocess.run(
                [candidate, "--version"], capture_output=True, text=True
            )
        except OSError:
            continue
        if completed.returncode == 0:
            return candidate
    return None


def build(cargo, workspace, target):
    """Cargo's build of the workspace into `target`, offline.

    `--offline` rather than `--locked`: the workspace is generated, so it has no
    committed lockfile to hold cargo to, and there is nothing in it to fetch —
    a fixture with no dependencies cannot reach the network, and the build is
    what writes the lockfile the checker's own `cargo metadata --locked` then
    reads.
    """
    completed = subprocess.run(
        [cargo, "build", "--offline", "-p", "probe", "-p", "other"],
        cwd=workspace,
        env={**os.environ, "CARGO_TARGET_DIR": str(workspace / target)},
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise AssertionError(f"the scratch workspace builds: {completed.stderr}")


class RealCargoOutputTests(unittest.TestCase):
    """The oracle on one real build, in two target directories."""

    @classmethod
    def setUpClass(cls):
        cargo = runnable_cargo()
        if cargo is None:
            raise unittest.SkipTest(
                "no cargo that can be run, from CARGO or from PATH, so no real "
                "build can be made and these cases would prove nothing"
            )
        if shutil.which("cargo") is None:
            # `cargo_metadata` runs `cargo` by name, so a cargo reached only
            # through the variable that names it has to be on PATH for it.
            os.environ["PATH"] = (
                str(Path(cargo).parent) + os.pathsep + os.environ.get("PATH", "")
            )
        cls.directory = tempfile.TemporaryDirectory(
            prefix="symbiote-source-record-cargo-"
        )
        cls.workspace = Path(cls.directory.name) / "ws"
        write_workspace(cls.workspace)
        cls.offline = os.environ.get("CARGO_NET_OFFLINE")
        os.environ["CARGO_NET_OFFLINE"] = "true"
        build(cargo, cls.workspace, "target")
        # The same build into a directory of its own inside the workspace, so the
        # excusal has something to excuse that is not called `target`.
        build(cargo, cls.workspace, "build-output")
        cls.metadata = cargo_metadata(cls.workspace)
        cls.packages = closure_package_ids(cls.metadata, "probe")

    @classmethod
    def tearDownClass(cls):
        if cls.offline is None:
            os.environ.pop("CARGO_NET_OFFLINE", None)
        else:
            os.environ["CARGO_NET_OFFLINE"] = cls.offline
        cls.directory.cleanup()

    def profile(self, target):
        return self.workspace / target / "debug"

    def test_the_unit_that_produced_an_artifact_is_the_one_cargo_wired_to_it(self):
        """Cargo's uplift names the unit, and the hash off it names the evidence.

        A hand-built fixture decides a unit's hash for itself; cargo computes it,
        and the relation that ties the artifact to the unit that wrote it is
        cargo's own bookkeeping: the output in ``deps`` and the name the profile
        directory carries are the same file. Measured on this build, exactly one
        output shares the binary's file while three dep-info files are spelled
        ``probe-<hash>.d``, so a rule reading the name has three candidates and
        the relation has one.
        """
        target = self.profile("target")
        binary = target / "probe"
        self.assertTrue(binary.is_file(), f"cargo uplifts the binary to {binary}")
        uplifted = [
            entry
            for entry in sorted((target / "deps").iterdir())
            if entry.is_file() and os.path.samefile(entry, binary)
        ]
        self.assertEqual(
            len(uplifted),
            1,
            "the profile directory carries a name for exactly one unit's output: "
            f"{[entry.name for entry in uplifted]}",
        )
        produced = uplifted[0].name.rpartition("-")[2]
        self.assertEqual(
            driven_unit(target, binary),
            produced,
            "the hash of the output the artifact shares its file with is the unit "
            "that produced it",
        )

        dep_info = target / "deps" / f"probe-{produced}.d"
        self.assertTrue(
            dep_info.is_file(),
            f"the unit's dep-info is spelled with that hash: {dep_info.name}",
        )
        self.assertEqual(unit_hash(dep_info), produced)
        # The layout the hand-built fixture wrote wrong, as cargo writes it: one
        # JSON per target the unit compiles, named after that target.
        fingerprint = target / ".fingerprint" / f"probe-{produced}" / "bin-probe.json"
        self.assertTrue(
            fingerprint.is_file(),
            "cargo's fingerprint of the unit's hash names the target it compiles "
            f"as {fingerprint.parent.name}/{fingerprint.name}",
        )

        units = unit_dep_info(target, self.metadata, self.packages)
        self.assertIn(
            dep_info,
            units,
            "and that unit is what the check measures the record against: "
            f"{sorted(entry.name for entry in units)}",
        )
        self.assertEqual(units[dep_info].targets, frozenset({("bin", "probe")}))

        spelled_alike = sorted(entry.name for entry in (target / "deps").glob("probe-*.d"))
        self.assertGreater(
            len(spelled_alike),
            1,
            "the crate name is shared by other units, so it cannot be the identity: "
            f"{spelled_alike}",
        )

    def test_a_same_named_unit_of_another_package_is_not_this_packages_unit(self):
        """Cargo's fingerprint of a unit's hash is what places the unit.

        The other package's library carries the driven binary's crate name, so its
        dep-info is spelled ``probe-<hash>.d`` exactly as the binary's is and
        differs in nothing but the hash. Nothing in the file names says which
        package it belongs to; the fingerprint directory does, and a rule reading
        the name credits the other package's unit — and its reads — to this build.
        """
        target = self.profile("target")
        placed, targets = unit_fingerprints(target)
        foreign = [
            entry
            for entry in sorted((target / "deps").glob("probe-*.d"))
            if placed.get(unit_hash(entry)) == "other"
        ]
        self.assertEqual(
            len(foreign),
            1,
            "the workspace holds one unit of another package under the driven "
            "binary's crate name",
        )
        self.assertEqual(
            targets[unit_hash(foreign[0])],
            frozenset({("lib", "probe")}),
            "and cargo's fingerprint of its hash names the package and target that "
            "compiled it",
        )

        units = unit_dep_info(target, self.metadata, self.packages)
        self.assertNotIn(
            foreign[0],
            units,
            "a unit of a package this build does not compile is not evidence about "
            "this one, however its dep-info is spelled",
        )
        self.assertEqual(
            {unit.package for unit in units.values()},
            {"probe"},
            f"only this package's own units are counted: {sorted(units)}",
        )

    def test_a_target_directory_of_its_own_is_excused_where_it_is_given(self):
        """The excusal is the directory the check was given, not the name `target`.

        The build into ``<workspace>/build-output`` is the same build, and its
        units read files cargo wrote under that directory: the generated source
        the binary includes, the build script's own artifacts, the dep-info of a
        dependency. Measured, giving that directory excuses every one of them, and
        giving any other directory requires them — which is what a rule reading
        the name ``target`` did to a legitimate build.
        """
        out = self.profile("build-output")
        units = unit_dep_info(out, self.metadata, self.packages)
        self.assertTrue(
            units,
            "the directory of its own holds the units of the same build: "
            f"{sorted(entry.name for entry in units)}",
        )
        excused = workspace_reads(units, self.workspace, out)
        self.assertEqual(
            sorted(str(path.relative_to(self.workspace)) for path in excused),
            ["probe/build.rs", "probe/src/lib.rs", "probe/src/main.rs"],
            "what a real build read, with its own output excused",
        )
        self.assertEqual(
            [path for path in excused if out in path.parents],
            [],
            "nothing cargo wrote under that directory is required of the record",
        )

        # The control that makes the excusal the cause: given any other directory,
        # the same units' reads under this one are required, and the file the
        # binary includes is among them.
        required = workspace_reads(units, self.workspace, self.profile("target"))
        under = sorted(
            str(path.relative_to(self.workspace))
            for path in required
            if out in path.parents
        )
        self.assertTrue(
            under,
            "the directory holds reads that some other directory's check would "
            "require, so the acceptance above is the excusal rather than the build "
            "reading nothing there",
        )
        self.assertTrue(
            any(path.endswith("out/generated.rs") for path in under),
            f"the file the binary includes is one of them: {under}",
        )


if __name__ == "__main__":
    unittest.main()
