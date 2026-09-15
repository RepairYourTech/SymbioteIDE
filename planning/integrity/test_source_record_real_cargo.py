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
* **the spellings the hand-built fixtures decide for themselves.**
  `test_source_record.py` writes each unit's dep-info and fingerprint by hand, so a
  spelling cargo does not write is invisible from that suite alone. Measured, its
  fixture wrote every input absolute while rustc writes a package's own sources
  relative to the directory cargo ran in — so dropping the checker's relative
  resolution failed none of that suite's tests then, and five of them now.
  `HandFixtureAgreementTests` puts the same shapes in front of the build below and
  asserts the fixture's letters are cargo's, so a drift is a failure here rather
  than a rule nobody measured.

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
    parse_dep_info,
    unit_dep_info,
    unit_fingerprints,
    unit_hash,
    workspace_reads,
)
from test_source_record import Fixture

# The workspace the cases build: a package whose binary includes a file its own
# build script generates (so the build reads under its target directory), a
# library of the same package (a second unit of the same crate name), and a
# package whose library carries the driven binary's crate name (a unit another
# package compiles, spelled the same way). A fourth package carries a hyphenated
# binary target and a source whose name holds a space, which is what the hand
# fixtures decide for themselves: the dep-info file underscored from the crate
# name, the fingerprint and artifact under the target's own hyphens, and the
# space escaped in the dep-info's bytes.
WORKSPACE = {
    "Cargo.toml": (
        '[workspace]\nmembers = ["probe", "other", "hyphen-demo"]\nresolver = "2"\n'
    ),
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
    "hyphen-demo/Cargo.toml": (
        '[package]\nname = "hyphen-demo"\nversion = "0.0.0"\nedition = "2021"\n'
    ),
    "hyphen-demo/src/main.rs": (
        '#[path = "a b.rs"]\nmod part;\n\nfn main() {\n'
        '    println!("{}", part::VALUE);\n}\n'
    ),
    "hyphen-demo/src/a b.rs": 'pub const VALUE: &str = "part";\n',
}


def write_workspace(root):
    """The scratch workspace, under `root`."""
    for relative, contents in WORKSPACE.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents)


def runnable_cargo():
    """The first of `CARGO` and PATH that answers `cargo --version`, or `None`."""
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


def cargo_or_skip():
    """A cargo that can be run, reachable by name, or a skip with the reason.

    `CARGO` is what a test run through cargo sets, and it is not proof that the
    binary it names is there: measured, `CARGO=/nonexistent` with a cargo on PATH
    made these cases fail with `FileNotFoundError` out of `setUpClass` rather than
    skip, which is the one thing a case that needs a real build must not do where
    there is none to be had. So the cargo is asked for its version before
    anything is built, and a candidate that cannot answer is passed over.

    `cargo_metadata` runs `cargo` by name, so a cargo reached only through the
    variable that names it has to be on PATH for it.
    """
    cargo = runnable_cargo()
    if cargo is None:
        raise unittest.SkipTest(
            "no cargo that can be run, from CARGO or from PATH, so no real "
            "build can be made and these cases would prove nothing"
        )
    if shutil.which("cargo") is None:
        os.environ["PATH"] = (
            str(Path(cargo).parent) + os.pathsep + os.environ.get("PATH", "")
        )
    return cargo


def offline_environment():
    """`CARGO_NET_OFFLINE` set for the build, and the value to put back."""
    previous = os.environ.get("CARGO_NET_OFFLINE")
    os.environ["CARGO_NET_OFFLINE"] = "true"
    return previous


def restore_offline(previous):
    if previous is None:
        os.environ.pop("CARGO_NET_OFFLINE", None)
    else:
        os.environ["CARGO_NET_OFFLINE"] = previous


def build(cargo, workspace, target):
    """Cargo's build of the workspace into `target`, offline.

    `--offline` rather than `--locked`: the workspace is generated, so it has no
    committed lockfile to hold cargo to, and there is nothing in it to fetch —
    a fixture with no dependencies cannot reach the network, and the build is
    what writes the lockfile the checker's own `cargo metadata --locked` then
    reads.
    """
    completed = subprocess.run(
        [
            cargo,
            "build",
            "--offline",
            "-p",
            "probe",
            "-p",
            "other",
            "-p",
            "hyphen-demo",
        ],
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
        cargo = cargo_or_skip()
        cls.directory = tempfile.TemporaryDirectory(
            prefix="symbiote-source-record-cargo-"
        )
        cls.workspace = Path(cls.directory.name) / "ws"
        write_workspace(cls.workspace)
        cls.offline = offline_environment()
        build(cargo, cls.workspace, "target")
        # The same build into a directory of its own inside the workspace, so the
        # excusal has something to excuse that is not called `target`.
        build(cargo, cls.workspace, "build-output")
        cls.metadata = cargo_metadata(cls.workspace)
        cls.packages = closure_package_ids(cls.metadata, "probe")

    @classmethod
    def tearDownClass(cls):
        restore_offline(cls.offline)
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


class HandFixtureAgreementTests(unittest.TestCase):
    """The hand-built fixture's own letters, against the ones cargo wrote.

    `test_source_record.py` decides for itself how a unit's dep-info is spelled,
    where its fingerprint sits, what it is called and which file is the unit's
    artifact. A fixture that decides wrongly pins a layout the real cargo does not
    write, and the suite it lives in cannot notice, because the fixture is the
    thing being trusted: one such drift already happened (a fingerprint's
    extensionless file with no ``.json``), and another was found this way and
    fixed (`Fixture.unit` wrote every input absolute, where rustc writes a
    package's own sources relative to the directory cargo ran in).

    What is compared is the *spelling* of the same shapes, not the names — cargo's
    own names differ and are pinned in `RealCargoOutputTests` — and the two rows of
    each comparison are asserted to have the same shape rather than described as
    agreeing.
    """

    @classmethod
    def setUpClass(cls):
        cargo = cargo_or_skip()
        cls.directory = tempfile.TemporaryDirectory(
            prefix="symbiote-source-record-fixture-"
        )
        cls.workspace = Path(cls.directory.name) / "ws"
        write_workspace(cls.workspace)
        cls.offline = offline_environment()
        build(cargo, cls.workspace, "target")
        cls.target = cls.workspace / "target" / "debug"

    @classmethod
    def tearDownClass(cls):
        restore_offline(cls.offline)
        cls.directory.cleanup()

    def test_the_hand_fixtures_dep_info_is_spelled_the_way_cargo_spells_one(self):
        """A source relative to the build's directory, cargo's own paths absolute.

        The fixture spells both kinds itself, and it spelled both absolute: the
        branch every real build's sources take (a relative token, resolved against
        the workspace) was reached by none of the hand suite's tests, which is
        measured — dropping that branch from the checker failed no test of it, and
        five with the fixture spelling them as cargo does.
        """
        # Cargo's own letters, from the unit whose source includes a generated
        # file: the one dep-info here that names a build output.
        generating = [
            entry
            for entry in sorted((self.target / "deps").glob("probe-*.d"))
            if "out/generated.rs" in entry.read_text()
        ]
        self.assertEqual(
            len(generating),
            1,
            f"one unit of the driven package reads a generated file: {generating}",
        )
        real_text = generating[0].read_text()
        real = parse_dep_info(real_text)
        # Both of cargo's spellings, read out of its own bytes rather than written
        # here: the package's own source, and the file its build script generated.
        # Each appears in several of the dep-info's rules and is spelled one way.
        sources = {token for token in real if token.endswith("main.rs")}
        outputs = {token for token in real if token.endswith("out/generated.rs")}
        self.assertEqual(len(sources), 1, f"rustc spells a source one way: {real}")
        self.assertEqual(len(outputs), 1, f"and a generated file one way: {real}")
        source, output = sources.pop(), outputs.pop()
        self.assertEqual(
            source,
            "probe/src/main.rs",
            "the path cargo was given the source by, in cargo's own bytes",
        )
        self.assertFalse(source.startswith("/"), f"a source, relative: {real}")
        self.assertTrue(output.startswith("/"), f"cargo's own output, absolute: {real}")

        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            generated = fixture.write(
                "target/debug/build/demo-9a/out/generated.rs", "// generated\n"
            )
            dep_info = fixture.unit(
                "demo-9a", ["crates/demo/src/main.rs", str(generated)]
            )
            written = parse_dep_info(dep_info.read_text())
            # The two rows: how far each of the fixture's two inputs is from being
            # absolute, against how far cargo's two are. The names differ and are
            # each their own; the shape is what has to agree.
            self.assertEqual(
                [token.startswith("/") for token in written[1:]],
                [source.startswith("/"), output.startswith("/")],
                f"the fixture spells {written[1:]} the way cargo spells "
                f"{[source, output]}",
            )
            self.assertEqual(
                written[1],
                "crates/demo/src/main.rs",
                "the fixture writes a source the way the test names it, which is how "
                "a build is given it",
            )
            self.assertEqual(
                written[2],
                str(generated),
                "and a file under its own output directory absolutely, as cargo "
                "passes one",
            )

        # And the shapes the hand parser's own fixtures feed it, in cargo's bytes:
        # the env-dep comment, a bare rule line, and a path with a space escaped.
        self.assertIn("# env-dep:OUT_DIR=", real_text)
        self.assertIn(f"{source}:", real_text)
        spaced = sorted((self.target / "deps").glob("hyphen_demo-*.d"))
        self.assertEqual(len(spaced), 1, f"the hyphenated unit's dep-info: {spaced}")
        self.assertIn(
            "hyphen-demo/src/a\\ b.rs",
            spaced[0].read_text(),
            "rustc escapes a space in a path the hand parser is tested on",
        )
        self.assertIn(
            "hyphen-demo/src/a b.rs",
            parse_dep_info(spaced[0].read_text()),
            "and the parser's own escaping rule is what recovers it",
        )

    def test_the_hand_fixtures_fingerprint_and_identity_spellings_are_cargos(self):
        """An underscored dep-info file name, hyphens in the fingerprint and artifact.

        One target, three names, and cargo spells them two ways: the dep-info file
        with the crate name's underscores (``hyphen_demo-<hash>.d``), the artifact
        and the fingerprint's file with the target's own hyphens
        (``bin-hyphen-demo``). The hand fixture encodes that relation in
        `test_a_hyphenated_unit_is_matched_by_its_file_name` and writes it here, so
        this is where the two are compared.
        """
        dep_infos = sorted((self.target / "deps").glob("hyphen_demo-*.d"))
        self.assertEqual(len(dep_infos), 1, f"the hyphenated unit: {dep_infos}")
        cargo_dep_info = dep_infos[0]
        fingerprint = self.target / ".fingerprint" / (
            f"hyphen-demo-{unit_hash(cargo_dep_info)}"
        )
        self.assertTrue(
            fingerprint.is_dir(),
            "cargo places a unit under the package that compiles it and the hash "
            f"its dep-info is spelled with: {fingerprint}",
        )
        cargo_names = sorted(entry.name for entry in fingerprint.iterdir())
        self.assertIn("bin-hyphen-demo", cargo_names)
        self.assertIn(
            "bin-hyphen-demo.json",
            cargo_names,
            f"the fingerprint's JSON is named after the target: {cargo_names}",
        )

        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory, target="demo-launch")
            written = fixture.unit(
                "demo_launch-9a", ["crates/demo/src/main.rs"], artifact="demo-launch-9a"
            )
            placed = fixture.fingerprint("other", "25d0")
            names = sorted(entry.name for entry in placed.iterdir())
            # The two rows, each three names of one unit: cargo's and the
            # fixture's, spelled the same way.
            self.assertEqual(
                [
                    (
                        cargo_dep_info.name.split("-")[0],
                        "bin-hyphen-demo",
                        "hyphen-demo",
                    ),
                    (written.name.split("-")[0], names[0], "demo-launch"),
                ],
                [
                    ("hyphen_demo", "bin-hyphen-demo", "hyphen-demo"),
                    ("demo_launch", "bin-demo-launch", "demo-launch"),
                ],
                f"the fixture writes {[written.name, names]}; cargo wrote "
                f"{[cargo_dep_info.name, cargo_names]}",
            )
            self.assertEqual(
                names,
                ["bin-demo-launch", "bin-demo-launch.json"],
                "the fingerprint is the pair cargo writes, JSON included — the "
                "file an earlier fixture left out",
            )
            self.assertEqual(
                placed.name,
                "other-25d0",
                "and a unit of a same-named target in another package is placed "
                "under that package, as cargo places it",
            )
            # The uplift relation, the fixture's half of what `RealCargoOutputTests`
            # measures on cargo's output: the artifact and the binary are one file.
            linked = fixture.unit("demo-9a", ["crates/demo/src/main.rs"], link=True)
            self.assertTrue(
                os.path.samefile(
                    fixture.target / "deps" / "demo-9a",
                    fixture.target / fixture.target_name,
                ),
                f"the fixture links the unit's output into the profile directory as "
                f"cargo does: {linked}",
            )


if __name__ == "__main__":
    unittest.main()
