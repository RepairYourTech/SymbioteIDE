"""Run: python3 planning/integrity/test_source_record.py."""

import os
import tempfile
import unittest
from pathlib import Path

from source_record import (
    ENVIRONMENT_PREFIX,
    EXCLUSIONS,
    HASH_LENGTH,
    MALFORMED_REMEDY,
    MARKS,
    RECORD_ENCODING,
    RECORD_END,
    RECORD_START,
    TOOLCHAIN_VARIABLE,
    UNASKED_CARGO,
    UNDECODABLE_REMEDY,
    UNFRAMED_REMEDY,
    UNKNOWN_REMEDY,
    UNNAMED_RESOLUTION,
    UNSET,
    WIRE,
    WIRE_FILE,
    UndecodableRecord,
    UnframedRecord,
    check,
    classify,
    declares_workspace,
    embedded_record,
    is_hash,
    mark_locator,
    named_workspace,
    nearest_workspace_root,
    driven_unit,
    parse_dep_info,
    read_record,
    read_wire,
    undecodable_text,
    workspace_roots,
)


class Fixture:
    """A workspace with one package built into one binary that carries a record.

    Small enough to be read whole, and real enough to drive the checker: the
    dep-info files are the shape rustc writes, the metadata is the shape
    `cargo metadata` emits, and the record is the shape the binaries embed.
    """

    def __init__(self, directory, target="demo", target_dir="target/debug"):
        """A workspace whose binary is driven out of `target_dir`.

        `target_dir` is a path relative to the workspace root, because that is what
        a check is given and the directory is a fact about the build rather than a
        name: a test that is about *which* directory is excused drives one of its
        own, and the default is cargo's.
        """
        self.root = Path(directory)
        self.target_name = target
        self.target = self.root / target_dir
        (self.target / "deps").mkdir(parents=True)
        self.package = self.root / "crates" / "demo"
        self.write("Cargo.toml", "[workspace]\nmembers = [\"crates/demo\"]\n")
        self.write("Cargo.lock", "version = 4\n")
        self.write("crates/demo/Cargo.toml", "[package]\nname = \"demo\"\n")
        self.main = self.write("crates/demo/src/main.rs", "fn main() {}\n")

    def write(self, relative, content):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        return path

    def fingerprint(self, package, hash, kind="bin", target=None):
        """Cargo's own record of a unit: which package compiles it, and its target.

        One directory per unit, named after the package that compiles it and the
        unit's metadata hash — the same hash the unit's dep-info file carries — and
        one pair of files per target the unit compiles, named after that target:
        the fingerprint itself (`bin-demo`) and its JSON (`bin-demo.json`). A
        target defaults to the one this fixture's binary is built from, which is
        what a normal target directory holds a unit for.
        """
        directory = self.target / ".fingerprint" / f"{package}-{hash}"
        directory.mkdir(parents=True, exist_ok=True)
        named = f"{kind}-{target or self.target_name}"
        (directory / named).write_text("")
        (directory / f"{named}.json").write_text("")
        return directory

    def unit(self, stem, reads, artifact=None, package="demo", kind="bin", target=None, link=False):
        """A dep-info file named after the unit, describing the artifact it names.

        The unit's package and target are the ones cargo's fingerprint of its hash
        names, so a test that is about *which* unit a dep-info is writes those too:
        a stem's crate name is shared by every package carrying a target of that
        name, and by one package's library, its docs and its test harnesses.

        `link` writes the artifact this unit produced *as* the binary, which is
        what cargo does where it can: it writes the unit's output into `deps` and
        links it into the profile directory under the artifact name, so the two are
        one file. That relation is how the check knows which unit produced the
        binary — see `driven_unit` — so a test that is about *that* passes it, and
        one that is about the identity cargo's relation cannot give leaves it out.

        Each read is written the way the test names it, which is the way the build
        was given it: measured on real builds, rustc spells a package's own sources
        relative to the directory cargo ran in (`probe/src/main.rs`) and cargo's own
        generated paths absolute, however the directory holding them is named. So a
        test that means a source names it as the workspace spells it, and one that
        means a file under the target directory names it absolutely — which is what
        `test_source_record_real_cargo` compares these spellings against, on dep-info
        cargo wrote. Returns the unit's own dep-info file.
        """
        output = self.target / "deps" / (artifact or stem)
        body = f"{output}: " + " ".join(reads) + "\n"
        dep_info = self.target / "deps" / f"{stem}.d"
        dep_info.write_text(body)
        self.fingerprint(package, stem.rsplit("-", 1)[-1], kind, target)
        if link:
            output.write_bytes(b"")
            os.link(output, self.target / self.target_name)
        return dep_info

    def binary(self, record, name=None):
        """A binary carrying `record`, as `locator -> hash`."""
        return self.binary_of_lines(
            [f"{hash}\t{locator}" for locator, hash in record.items()], name
        )

    def binary_of_lines(self, lines, name=None):
        """A binary carrying a record spelled line by line.

        A test that is about what a *line* is writes the line itself, since the
        record's own writer would only write the shapes it means to.
        """
        path = self.target / (name or self.target_name)
        body = "".join(f"{line}\n" for line in lines)
        path.write_bytes(b"ELF\x00" + RECORD_START + body.encode() + RECORD_END)
        return path

    def metadata(self):
        identifier = f"demo 0.1.0 (path+file://{self.package})"
        return {
            "workspace_root": str(self.root),
            "packages": [
                {
                    "name": "demo",
                    "id": identifier,
                    "manifest_path": str(self.package / "Cargo.toml"),
                    "source": None,
                    "targets": [{"name": self.target_name, "kind": ["bin"]}],
                }
            ],
            "resolve": {"nodes": [{"id": identifier, "deps": []}]},
        }

    def problems(self, record, metadata=None):
        self.binary(record)
        return self.judged(metadata)

    def problems_of_lines(self, lines, metadata=None):
        self.binary_of_lines(lines)
        return self.judged(metadata)

    def judged(self, metadata=None):
        found, _ = check(
            self.root, self.target, [("demo", self.target_name)], metadata or self.metadata()
        )
        return found


def with_dependency(fixture, manifest):
    """`fixture`'s metadata with one more local package at `manifest`."""
    metadata = fixture.metadata()
    identifier = f"dep 0.1.0 (path+file://{manifest.parent})"
    metadata["packages"].append(
        {
            "name": "dep",
            "id": identifier,
            "manifest_path": str(manifest),
            "source": None,
            "targets": [{"name": "dep", "kind": ["lib"]}],
        }
    )
    metadata["resolve"]["nodes"][0]["deps"].append(
        {"pkg": identifier, "dep_kinds": [{"kind": None}]}
    )
    metadata["resolve"]["nodes"].append({"id": identifier, "deps": []})
    return metadata


def dependency_in_its_own_workspace(fixture, directory):
    """A closure package that belongs to a second workspace outside the fixture.

    Returns the metadata with the edge added, the dependency's own manifest and
    that workspace's root directory: the record must name all three, and the
    root's lockfile too, since the walk over the dependency's directory reaches
    neither the manifest above it nor the lockfile beside that.
    """
    second = Path(directory)
    (second / "dep" / "src").mkdir(parents=True)
    (second / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["dep"]\n\n[workspace.package]\nversion = "0.1.0"\n'
    )
    (second / "Cargo.lock").write_text("version = 4\n")
    manifest = second / "dep" / "Cargo.toml"
    manifest.write_text('[package]\nname = "dep"\nversion.workspace = true\n')
    (second / "dep" / "src" / "lib.rs").write_text("// dep\n")
    return with_dependency(fixture, manifest), manifest, second


def dependency_below_another_workspace_manifest(fixture, directory):
    """A closure package under a nearer `[workspace]` manifest than its root.

    Returns the metadata with the edge added, the dependency's manifest and the
    two workspace directories above it. The package names `../..` as its
    workspace, so that is the root cargo reads and the nearer `[workspace]`
    manifest is never loaded; a name settles it, which is why the nearer
    directory is not a root of this package.
    """
    outer = Path(directory) / "outer"
    nested = outer / "inner"
    (nested / "dep" / "src").mkdir(parents=True)
    (outer / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["inner/dep"]\n\n[workspace.package]\nversion = "1.1.1"\n'
    )
    (nested / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["dep"]\n\n[workspace.package]\nversion = "2.2.2"\n'
    )
    manifest = nested / "dep" / "Cargo.toml"
    manifest.write_text('[package]\nname = "dep"\nversion.workspace = true\nworkspace = "../.."\n')
    (nested / "dep" / "src" / "lib.rs").write_text("// dep\n")
    return with_dependency(fixture, manifest), manifest, outer, nested


def excluded_dependency(fixture, directory):
    """A closure package the workspace above it excludes.

    Returns the metadata with the edge added and the package's own directory.
    Cargo does not claim such a package — measured, its search reads the
    excluding manifest and walks past it, so with nothing above the package is
    a workspace of one — and its own lockfile is then what cargo reads for it,
    while the excluding manifest is still read to learn that.
    """
    workspace = Path(directory) / "ws"
    (workspace / "dep" / "src").mkdir(parents=True)
    (workspace / "Cargo.toml").write_text('[workspace]\nmembers = ["app"]\nexclude = ["dep"]\n')
    (workspace / "dep" / "Cargo.toml").write_text('[package]\nname = "dep"\n')
    (workspace / "dep" / "src" / "lib.rs").write_text("// dep\n")
    (workspace / "dep" / "Cargo.lock").write_text("version = 4\n")
    manifest = workspace / "dep" / "Cargo.toml"
    return with_dependency(fixture, manifest), manifest, workspace


def complete(fixture):
    """A record that covers what `fixture`'s unit reads, its cargo inputs, and
    its configuration."""
    # Hashes the shape a real record holds: 64 hexadecimal characters, which is
    # also what tells a record's inputs from a line that says the record cannot
    # be stood behind (`source_record.classify`).
    return {
        "crates/demo/src/main.rs": "a" * 64,
        "crates/demo/Cargo.toml": "b" * 64,
        "Cargo.toml": "d" * 64,
        "Cargo.lock": "e" * 64,
        "env:CARGO": "c" * 64,
    }


class RecordReadingTests(unittest.TestCase):
    def test_a_binary_without_a_record_reads_as_none(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "pure"
            path.write_bytes(b"ELF\x00 no record here")
            self.assertIsNone(read_record(path))

    def test_a_truncated_record_reads_as_none(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "cut"
            path.write_bytes(b"ELF\x00" + RECORD_START + b"\nabc\tcrates/x.rs\n")
            self.assertIsNone(read_record(path))

    def test_a_record_whose_bytes_are_not_the_encoding_is_named_not_raised(self):
        """The wire states the encoding of the bytes it frames.

        A record whose bytes are not it is a refusal, not a crash and not a binary
        without a record: the crate refuses such a binary too, naming where the
        bytes stop being the encoding, so an artifact neither reader can decode
        gets the same answer from both. The bytes are inside a line, which is
        where a build's record puts them: the `end` that ends the record follows
        the LF the last line ends with.
        """
        offset = len(b"0000\tcrates/x.rs\n0000\tcrates/")
        body = b"0000\tcrates/x.rs\n0000\tcrates/\xff\xfe.rs\n"
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "not-utf8"
            path.write_bytes(b"ELF\x00" + RECORD_START + body + RECORD_END)
            with self.assertRaises(UndecodableRecord) as caught:
                embedded_record(path.read_bytes())
            self.assertEqual(caught.exception.offset, offset)
            with self.assertRaises(UndecodableRecord):
                read_record(path)

    def test_a_record_the_wire_cannot_decode_refuses_the_check(self):
        """`check` refuses it in the file's own text, offset filled in.

        The same text the crate prints, since neither holds it: the file spells
        both the encoding and the byte the bytes stop at, and this is the message
        that comes back when an artifact either reader cannot decode is put to it.
        """
        body = b"0000\tcrates/x.rs\n0000\tcrates/\xff\xfe.rs\n"
        offset = len(b"0000\tcrates/x.rs\n0000\tcrates/")
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            (fixture.target / fixture.target_name).write_bytes(
                b"ELF\x00" + RECORD_START + body + RECORD_END
            )
            message = fixture.judged()
            self.assertEqual(message, [f"demo: {undecodable_text(offset)}"])
            self.assertIn(f"at byte {offset} of the record", message[0])
            self.assertIn(RECORD_ENCODING, message[0])

    def test_a_record_that_spells_a_marker_is_refused_rather_than_cut_short(self):
        """The wire states that a record's own bytes hold neither marker.

        A `end` inside the record's own bytes does not end it — the marker that
        ends it follows one of its lines — so the record is refused instead of
        being read up to that marker, which would take a record missing every
        line after it for the record. Measured, a locator legally named after the
        end marker made both readers do exactly that, and report a binary as
        matching a tree it was not built from.
        """
        locator = b"crates/x.rs" + RECORD_END
        body = b"0000\t" + locator + b"\n0000\tcrates/y.rs\n"
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "spells-a-marker"
            path.write_bytes(b"ELF\x00" + RECORD_START + body + RECORD_END)
            with self.assertRaises(UnframedRecord):
                embedded_record(path.read_bytes())
            with self.assertRaises(UnframedRecord):
                read_record(path)

    def test_a_record_the_wire_cannot_frame_refuses_the_check(self):
        """`check` refuses it in the file's own remedy for one.

        The same sentence the crate prints, so an artifact whose own bytes spell a
        marker is refused by both rather than read short by either.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            body = b"0000\tcrates/x.rs" + RECORD_END + b"\n0000\tcrates/y.rs\n"
            (fixture.target / fixture.target_name).write_bytes(
                b"ELF\x00" + RECORD_START + body + RECORD_END
            )
            self.assertEqual(fixture.judged(), [f"demo: {UNFRAMED_REMEDY}"])

    def test_a_locator_holding_a_line_feed_is_read_as_two_lines(self):
        """What a record cannot survive in a locator, so no build stamps one.

        A record's lines are divided at each LF, so a locator holding one ends the
        line it is spelled in and the rest of it is read as a line of its own —
        here refused as malformed, since it holds no TAB. Measured, a package
        holding such a path built successfully and stamped a record both readers
        then refused, in a remedy asking for a rebuild that reproduces the same
        bytes; `build_stamp` now refuses to stamp one, and this is the reader half
        that makes that refusal necessary.
        """
        body = b"0" * 64 + b"\tcrates/a\nb.rs\n"
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "line-feed-in-a-locator"
            path.write_bytes(b"ELF\x00" + RECORD_START + body + RECORD_END)
            record = read_record(path)
            self.assertEqual(record.inputs, {"crates/a": "0" * 64})
            self.assertEqual(
                record.malformed,
                "b.rs",
                "the rest of the locator is a line of its own, which the wire cannot read",
            )

    def test_an_end_marker_that_ends_no_line_frames_no_record(self):
        """A record holds lines, so bytes no `end` closes one of hold none.

        The refusal is a binary carrying no record rather than an empty record,
        which a reader would compare and call clean.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            (fixture.target / fixture.target_name).write_bytes(
                b"ELF\x00" + RECORD_START + b"0000\tcrates/x.rs" + RECORD_END
            )
            self.assertIsNone(read_record(fixture.target / fixture.target_name))
            problems = fixture.judged()
            self.assertEqual(len(problems), 1, problems)
            self.assertIn("carries no record", problems[0])


class DepInfoParsingTests(unittest.TestCase):
    def test_comments_are_not_dependencies_and_lines_continue(self):
        text = (
            "# env-dep:CARGO=/usr/bin/cargo\n"
            "/w/target/debug/deps/demo-9a: /w/crates/demo/src/main.rs \\\n"
            "  /w/crates/demo/src/other.rs\n"
        )
        self.assertEqual(
            parse_dep_info(text),
            ["/w/target/debug/deps/demo-9a", "/w/crates/demo/src/main.rs", "/w/crates/demo/src/other.rs"],
        )

    def test_an_escaped_space_stays_in_one_path(self):
        self.assertEqual(
            parse_dep_info("/w/out: /w/a\\ b.rs\n"), ["/w/out", "/w/a b.rs"]
        )


class CompletenessTests(unittest.TestCase):
    def test_a_complete_record_passes(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            self.assertEqual(fixture.problems(complete(fixture)), [])

    def test_a_read_input_the_record_does_not_name_is_reported(self):
        # The whole point of the tool: the record is short, every proof that
        # compares the record against the tree stays green, and this names it.
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            record = complete(fixture)
            del record["crates/demo/src/main.rs"]
            problems = fixture.problems(record)
            self.assertEqual(len(problems), 1)
            self.assertEqual(
                problems[0],
                f"demo: the record does not name {fixture.main}, which "
                f"{fixture.target / 'deps' / 'demo-9a.d'} says was read",
            )

    def test_a_read_under_a_name_a_walk_skips_is_still_the_records_business(self):
        """A skip is the walk's rule, and a check requires what a unit actually read.

        A name is skipped by the walk only where skipping it cannot hide a file the
        build read, and a check is handed the evidence of what each unit read — so a
        read under one of those names is refused rather than excused. Measured on a
        probe package: `build.rs`'s `mod tests;` puts `tests/mod.rs` in the record,
        a check that excused the name reported the record clean with that line
        removed, and one that excuses nothing by name refused it, naming it as read
        by the build script's unit.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            under_git = fixture.write("crates/demo/.git/HEAD", "ref: refs/heads/main\n")
            installed = fixture.write("crates/demo/src/node_modules/dep/index.js", "// dep\n")
            test_target = fixture.write("crates/demo/tests/process.rs", "// a test\n")
            fixture.unit(
                "demo-9a",
                [
                    "crates/demo/src/main.rs",
                    "crates/demo/.git/HEAD",
                    "crates/demo/src/node_modules/dep/index.js",
                    "crates/demo/tests/process.rs",
                ],
            )
            source = fixture.target / "deps" / "demo-9a.d"
            self.assertEqual(
                fixture.problems(complete(fixture)),
                [
                    f"demo: the record does not name {under_git}, which {source} says was read",
                    f"demo: the record does not name {installed}, which {source} says was read",
                    f"demo: the record does not name {test_target}, which {source} says was read",
                ],
            )

    def test_the_files_the_build_wrote_under_the_directory_it_was_given_are_excused(self):
        """Cargo's own output is outside the record, and it is the *given* directory.

        A build script writes into `OUT_DIR`, under the profile directory the check
        is handed, and the record names the build script rather than what it wrote.
        Measured, a build driven into `<workspace>/build-output` left 353 such
        inputs under it, and a rule that excused the name `target` instead refused
        the binary for every one of them.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory, target_dir="build-output/debug")
            fixture.write(
                "build-output/debug/build/demo-9a/out/record.rs", "// written by the build\n"
            )
            fixture.unit(
                "demo-9a",
                ["crates/demo/src/main.rs", "build-output/debug/build/demo-9a/out/record.rs"],
            )
            self.assertEqual(fixture.problems(complete(fixture)), [])

    def test_a_directory_named_target_is_not_the_directory_the_check_was_given(self):
        """The excusal is the directory it was given, not the name cargo defaults to.

        So a record is not asked to name a file under some *other* build's output
        either, whatever that directory is called — the name decides nothing.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory, target_dir="build-output/debug")
            other = fixture.write("target/debug/build/demo-9a/out/other.rs", "// elsewhere\n")
            # The read under that other output directory is named absolutely, the
            # way cargo passes a generated path (`unit` above).
            fixture.unit("demo-9a", ["crates/demo/src/main.rs", str(other)])
            self.assertEqual(
                fixture.problems(complete(fixture)),
                [
                    f"demo: the record does not name {other}, which "
                    f"{fixture.target / 'deps' / 'demo-9a.d'} says was read"
                ],
            )

    def test_a_target_name_below_the_package_root_is_not_an_exclusion(self):
        # Cargo looks for its test, example and bench directories at the package
        # root alone. A file under `src/tests/` declared as `mod tests;` compiles
        # into the binary — measured, rustc reports errors inside it — so a
        # record that does not name it is short, which is the failure this
        # checker exists to catch rather than a change no rebuild could clear.
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            nested = fixture.write("crates/demo/src/tests/mod.rs", "// a module\n")
            fixture.unit(
                "demo-9a", ["crates/demo/src/main.rs", "crates/demo/src/tests/mod.rs"]
            )
            self.assertEqual(
                fixture.problems(complete(fixture)),
                [
                    f"demo: the record does not name {nested}, which "
                    f"{fixture.target / 'deps' / 'demo-9a.d'} says was read"
                ],
            )

    def test_a_dist_directory_is_not_an_exclusion(self):
        """`dist` is not a name any cargo build generates.

        Measured, a `mod dist;` compiling `src/dist/mod.rs` was dropped from the
        record while this checker reported the binary current, so the name is
        walked at a package root and below one alike — the file's rule skips only
        what a build cannot read a source from.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            nested = fixture.write("crates/demo/src/dist/mod.rs", "// a module\n")
            at_root = fixture.write("crates/demo/dist/tool.rs", "// a source\n")
            fixture.unit(
                "demo-9a",
                [
                    "crates/demo/src/main.rs",
                    "crates/demo/src/dist/mod.rs",
                    "crates/demo/dist/tool.rs",
                ],
            )
            self.assertEqual(
                fixture.problems(complete(fixture)),
                [
                    f"demo: the record does not name {at_root}, which "
                    f"{fixture.target / 'deps' / 'demo-9a.d'} says was read",
                    f"demo: the record does not name {nested}, which "
                    f"{fixture.target / 'deps' / 'demo-9a.d'} says was read",
                ],
            )

    def test_a_unit_of_another_package_is_not_one_of_the_driven_packages_own(self):
        """A green rests on a unit of the driven package, not on its name.

        Two packages can each carry a target of the same name, and both can be in
        the closure when one depends on the other — measured, `probe`'s binary
        `guard` and another package's binary `guard` wrote dep-info files differing
        in nothing but the hash. With the driven package's own dep-info gone — what
        `cargo clean -p <package>` leaves — a rule that reads crate names alone
        finds a unit of that name, reports the record covered, and has measured
        nothing of the build that made the binary.

        Here the dependency's unit is the one named `demo`, and everything it read
        is in the record, so nothing but the unit's *package* can refuse it.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            dependency = fixture.write("crates/dep/Cargo.toml", '[package]\nname = "dep"\n')
            metadata = with_dependency(fixture, dependency)
            for package in metadata["packages"]:
                if package["name"] == "dep":
                    package["targets"] = [{"name": "demo", "kind": ["bin"]}]
            record = complete(fixture)
            record["crates/dep/src/main.rs"] = "f" * 64
            fixture.unit("demo-9a", ["crates/dep/src/main.rs"], package="dep")
            self.assertEqual(
                fixture.problems(record, metadata),
                [
                    f"demo: {fixture.target} holds no dep-info for the unit that "
                    f"builds it, so nothing here measured what its build read — build it "
                    f"before checking its record"
                ],
            )

    def test_a_foreign_unit_is_not_compared_as_one_of_the_driven_binaries(self):
        """The other face of the same collision: a needless refusal.

        A unit of a package this build does not compile is not one of the closure,
        so what it read is not the driven binary's record to cover — measured, the
        foreign `guard-<hash>.d` was compared and its own source reported as an
        input the record does not name.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.write("crates/other/src/main.rs", "fn main() {}\n")
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            fixture.unit("demo-25d0", ["crates/other/src/main.rs"], package="other")
            self.assertEqual(fixture.problems(complete(fixture)), [])

    def test_the_unit_that_produced_the_binary_is_the_evidence_it_is_read_against(self):
        """Cargo's own uplift names that unit, so no other unit's evidence stands in.

        Cargo writes a unit's output into `deps` and links it into the profile
        directory under the artifact name, so the profile file *is* that unit's
        output, and the file in `deps` sharing it is the unit that produced the
        binary. Measured on this workspace, `symbioted` shares its file with exactly
        one output while 11 units name its `bin-symbioted` target — and those are
        not the same evidence: ten of them read 7 workspace files and one reads 10.
        Measured too, with the producing unit's dep-info gone and a same-target
        sibling's left, a rule satisfied by any unit of the target read 13 units and
        reported the record covered.

        The two shapes are the two cargo writes: a binary's output named after the
        crate it compiles (`demo-9a`), and a library's `libdemo-9a.rlib`, whose
        dep-info is still `demo-9a.d` — the hash, and not the output's own name, is
        what names the unit, which is measured by the entry-point test that drives
        a library artifact. Here the producing unit is linked to the binary and a
        sibling unit of the same package and target is present as well, so nothing
        but the failing unit's *identity* can refuse the sibling.
        """
        for artifact in (None, "libdemo-9a.rlib"):
            with self.subTest(artifact=artifact or "demo-9a"):
                with tempfile.TemporaryDirectory() as directory:
                    fixture = Fixture(directory)
                    own = fixture.unit(
                        "demo-9a",
                        ["crates/demo/src/main.rs"],
                        artifact=artifact,
                        link=True,
                    )
                    fixture.unit("demo-25d0", ["crates/demo/src/main.rs"])
                    record = complete(fixture)
                    self.assertEqual(fixture.problems(record), [])
                    own.unlink()
                    self.assertEqual(
                        fixture.problems(record),
                        [
                            f"demo: {fixture.target} holds no dep-info for the unit "
                            f"that builds it, so nothing here measured what its "
                            f"build read — build it before checking its record"
                        ],
                    )

    def test_where_no_output_is_the_binary_the_driven_target_is_the_evidence(self):
        """Cargo's relation is not always there to read, and then the target is.

        Cargo copies the output rather than linking it where the filesystem has no
        link to give, and a binary this target directory did not write has no output
        here at all. The tightest identity left is then the target the artifact is
        named after, which is still narrower than the package and than the crate
        name — and this is the path every other test in this class takes, since none
        of them links an output to the binary.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            self.assertEqual(fixture.problems(complete(fixture)), [])
            self.assertIsNone(
                driven_unit(fixture.target, fixture.target / fixture.target_name)
            )

    def test_another_unit_of_the_driven_package_is_not_the_binary_being_driven(self):
        """A green rests on the unit that compiles the binary, not on its package.

        One package compiles several units whose dep-info files are spelled with
        its binary's crate name — its library, its documentation, and the test
        harnesses anything that has run `cargo test` leaves. Measured on this
        workspace, 45 units satisfy a rule reading the crate name of `symbioted`
        and exactly one of them is the binary; the other 44 are its library, its
        documentation and its test harnesses, each of which leaves the same dep-info
        for a *different* compilation. So with the binary's own unit gone and any of
        those present the record read clean against evidence that never described
        the build, which is what a stale binary needs to pass.

        The three shapes here are the ones cargo actually writes for one bin target
        — `lib-demo`, `test-bin-demo`, `doc-bin-demo`, all carrying the hash of their
        own unit — and everything they read is in the record, so nothing but the
        unit's *target* can refuse them.
        """
        for kind, name in (("lib", "demo"), ("test", "bin-demo"), ("doc", "bin-demo")):
            with self.subTest(unit=f"{kind}-{name}"):
                with tempfile.TemporaryDirectory() as directory:
                    fixture = Fixture(directory)
                    metadata = fixture.metadata()
                    metadata["packages"][0]["targets"].append(
                        {"name": "demo", "kind": ["lib"]}
                    )
                    record = complete(fixture)
                    record["crates/demo/src/lib.rs"] = "f" * 64
                    fixture.unit(
                        "demo-9a", ["crates/demo/src/lib.rs"], kind=kind, target=name
                    )
                    self.assertEqual(
                        fixture.problems(record, metadata),
                        [
                            f"demo: {fixture.target} holds no dep-info for the unit that "
                            f"builds it, so nothing here measured what its build read — "
                            f"build it before checking its record"
                        ],
                    )

    def test_an_oracle_with_no_unit_of_the_driven_package_measures_nothing(self):
        """A green has to be earned by a dep-info of the driven package's own units.

        Measured: against a target directory holding no dep-info at all the check
        reported OK having read 0 units and 0 inputs, so a record nothing was
        compared with was called complete — and a `cargo clean` leaves exactly
        that directory behind.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            self.assertEqual(
                fixture.problems(complete(fixture)),
                [
                    f"demo: {fixture.target} holds no dep-info for the unit that "
                    f"builds it, so nothing here measured what its build read — build it "
                    f"before checking its record"
                ],
            )

    def test_a_dependencys_units_do_not_stand_in_for_the_driven_packages_own(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            dependency = fixture.write("crates/dep/Cargo.toml", '[package]\nname = "dep"\n')
            metadata = with_dependency(fixture, dependency)
            fixture.unit("dep-9a", ["crates/dep/src/lib.rs"])
            self.assertEqual(
                fixture.problems(complete(fixture), metadata),
                [
                    f"demo: {fixture.target} holds no dep-info for the unit that "
                    f"builds it, so nothing here measured what its build read — build it "
                    f"before checking its record"
                ],
            )

    def test_registry_and_generated_reads_are_not_the_records_business(self):
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryDirectory() as cache:
            fixture = Fixture(directory)
            # The registry cache is outside the workspace; the record cannot
            # name it, and the lockfile it does name pins it.
            registry = Path(cache) / "registry" / "serde" / "src" / "lib.rs"
            registry.parent.mkdir(parents=True)
            registry.write_text("// serde\n")
            generated = fixture.target / "build" / "demo-9a" / "out" / "record.rs"
            generated.parent.mkdir(parents=True)
            generated.write_text("// generated\n")
            (fixture.target / "deps" / "demo-9a.d").write_text(
                f"{fixture.target / 'deps' / 'demo-9a'}: {fixture.main} {registry} {generated}\n"
            )
            fixture.fingerprint("demo", "9a")
            self.assertEqual(fixture.problems(complete(fixture)), [])

    def test_a_hyphenated_unit_is_matched_by_its_file_name(self):
        # cargo spells a unit's dep-info file with underscores
        # (`demo_launch-9a.d`) and the artifact inside it with the target's own
        # hyphens (`demo-launch-9a`), and the fingerprint keeps the target's own
        # hyphens too (`bin-demo-launch`); only the file name identifies the unit
        # among the ones that name the target.
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory, target="demo-launch")
            fixture.unit("demo_launch-9a", ["crates/demo/src/main.rs"], artifact="demo-launch-9a")
            record = complete(fixture)
            self.assertEqual(fixture.problems(record), [])
            del record["crates/demo/src/main.rs"]
            self.assertEqual(len(fixture.problems(record)), 1)

    def test_a_missing_binary_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            found, _ = check(fixture.root, fixture.target, [("demo", "demo")], fixture.metadata())
            self.assertEqual(
                found, [f"{fixture.target / 'demo'} does not exist: build it before checking its record"]
            )


class MarkedRecordTests(unittest.TestCase):
    """A record whose build could not stand behind it.

    Each mark names the fact the build could not establish: the workspace the
    resolution was read in, where the readings did not establish it, or the
    questions cargo alone answers, where the cargo that ran the build could not
    be run to be asked. Either way the record may be missing the `[patch]` fork
    the binary compiled and only that build could have said which workspace to
    look in, so it is refused rather than compared, as `changed_sources` refuses
    it, and the refusal names what to do about it.
    """

    def test_a_marked_record_is_refused_rather_than_checked(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            record = complete(fixture)
            record[UNNAMED_RESOLUTION] = "unset"
            self.assertEqual(
                fixture.problems(record),
                [f"demo: {MARKS[UNNAMED_RESOLUTION]} (mark: {UNNAMED_RESOLUTION})"],
            )

    def test_a_record_marked_where_cargo_could_not_be_asked_is_refused(self):
        """`CARGO` named a program that could not be run.

        The two things only cargo answers — the workspace a run directory names
        and the closure it resolves for the package there — were never asked, so
        the record's roots and its closure are unchecked, and the refusal asks
        for a cargo that can be started rather than for a rebuild under a stale
        `PWD`.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            record = complete(fixture)
            record[UNASKED_CARGO] = "unset"
            self.assertEqual(
                fixture.problems(record),
                [f"demo: {MARKS[UNASKED_CARGO]} (mark: {UNASKED_CARGO})"],
            )
            self.assertIn("a cargo that can be started", fixture.problems(record)[0])

    def test_the_blocking_mark_is_reported_where_a_record_carries_more_than_one(self):
        """A record can hold both marks, and they are not equally blocking.

        The cargo that ran the build is one of the readings that can name the
        workspace its resolution was read in, so losing it can lose the
        resolution with it. Reporting the resolution would send the rebuild down
        the remedy that leaves the cargo unaskable, and the rebuild would be
        refused again; the cargo is the fact that has to be cleared first. The
        two lines are written in the record's own order, resolution first, so the
        choice is the marks' and not the lines'.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            record = complete(fixture)
            record[UNNAMED_RESOLUTION] = "unset"
            record[UNASKED_CARGO] = "unset"
            self.assertEqual(
                fixture.problems(record),
                [f"demo: {MARKS[UNASKED_CARGO]} (mark: {UNASKED_CARGO})"],
            )

    def test_a_mark_an_earlier_version_wrote_is_refused_by_its_shape(self):
        """The mark is recognised by its line's shape, not by its name.

        Measured, a record carrying the previous locator compared that line
        against a missing file — whose content is `unset` too — matched it, and
        was reported clean, so a build that could not stand behind its record
        passed whenever the crate renamed the mark. A mark this version does not
        know is still refused, and the refusal names it.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            record = complete(fixture)
            record["run-directory-not-named"] = "unset"
            self.assertEqual(
                fixture.problems(record),
                [f"demo: {UNKNOWN_REMEDY} (mark: run-directory-not-named)"],
            )

    def test_the_first_mark_the_record_spells_is_the_one_reported(self):
        """Where the wire names none of the marks a record carries, its own order
        decides.

        Each reader used to pick its own — measured, the crate the alphabetically
        first and this checker the first in the record — so one record could be
        described two ways. The alphabetically first mark is written *second*
        here, so the answer cannot be an ordering by name, and the crate's own
        test pins the same rule under the same two names.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            record = complete(fixture)
            record["zzz-old"] = "unset"
            record["aaa-old"] = "unset"
            self.assertEqual(
                fixture.problems(record),
                [f"demo: {UNKNOWN_REMEDY} (mark: zzz-old)"],
            )


class RecordLineRuleTests(unittest.TestCase):
    """What one record line is, by the rule the wire file states.

    The crate implements the same text (`Wire::line`, from the same file), so a
    shape cannot be a mark to one reader and an input, or an unread line, to the
    other. The rule itself, and what each reader used to decide for itself, is
    stated in the file rather than here.
    """

    def test_the_rule_reads_every_shape_the_way_the_wire_states(self):
        """The whole rule, shape by shape — the crate's own test pins the same
        table under the same name."""
        source = "crates/demo/src/main.rs"
        hash = "0" * 64
        for line, expected in [
            (f"{hash}\t{source}", ("input", source, hash)),
            (f"{hash}\tenv:CARGO", ("input", "env:CARGO", hash)),
            (f"{UNSET}\tenv:CARGO", ("input", "env:CARGO", UNSET)),
            (f"{hash}\t{UNASKED_CARGO}", ("named", UNASKED_CARGO, None)),
            (f"{UNSET}\t{UNASKED_CARGO}", ("named", UNASKED_CARGO, None)),
            (f"{UNSET}\trun-directory-not-named", ("unknown", "run-directory-not-named", None)),
            ("no tab at all", ("malformed", None, None)),
            ("", ("malformed", None, None)),
            (f"{hash}\t{source}\r", ("malformed", None, None)),
            ("\r", ("malformed", None, None)),
            (f"{hash}\t", ("malformed", None, None)),
            (f"abc\t{source}", ("malformed", None, None)),
            (f"{'g' * HASH_LENGTH}\t{source}", ("malformed", None, None)),
            (f"{'0' * (HASH_LENGTH - 1)}\t{source}", ("malformed", None, None)),
        ]:
            with self.subTest(line=line):
                self.assertEqual(classify(line), expected)

    def test_a_hash_is_the_shape_the_wire_spells(self):
        self.assertTrue(is_hash("0" * HASH_LENGTH))
        self.assertFalse(is_hash("0" * (HASH_LENGTH - 1)))
        self.assertFalse(
            is_hash("A" * HASH_LENGTH),
            "measured, the same digits in upper case were an input to one reader and a mark "
            "to the other, so the alphabet the file spells is the one both read",
        )

    def test_a_mark_whose_content_is_a_hash_is_read_as_that_mark(self):
        """The fail-open: this checker took such a line for a recorded file.

        The locator names a mark the wire knows, so the record cannot be stood
        behind whatever the content says — and measured, reading the content alone
        left this checker reporting the record clean while the crate refused it.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            record = complete(fixture)
            record[UNASKED_CARGO] = "0" * HASH_LENGTH
            self.assertEqual(
                fixture.problems(record),
                [f"demo: {MARKS[UNASKED_CARGO]} (mark: {UNASKED_CARGO})"],
            )

    def test_a_malformed_line_is_reported_ahead_of_a_mark(self):
        """The precedence the file states: a record the wire cannot read is one
        whose marks cannot be stood behind either, so the line is what is named
        rather than a mark the record also spells — the crate's own test pins the
        same record under the same rule."""
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            named = [f"{hash}\t{locator}" for locator, hash in complete(fixture).items()]
            self.assertEqual(
                fixture.problems_of_lines(
                    named + [f"{'0' * HASH_LENGTH}\t{UNASKED_CARGO}", "no tab at all"]
                ),
                [f"demo: {MALFORMED_REMEDY} The line is 'no tab at all'."],
            )

    def test_a_line_the_wire_does_not_read_refuses_the_record(self):
        """A line that is neither an input nor a mark is not guessed at.

        The refusal is the wire's own remedy for a line it cannot read, with the
        line named, so a record that lost a line — a blank one included, which
        both readers used to pass over — is refused rather than read short.
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            named = [f"{hash}\t{locator}" for locator, hash in complete(fixture).items()]
            for line in ["no tab at all", "", "abc\tcrates/demo/src/main.rs"]:
                with self.subTest(line=line):
                    self.assertEqual(
                        fixture.problems_of_lines(named + [line]),
                        [f"demo: {MALFORMED_REMEDY} The line is {line!r}."],
                    )


# The crate that writes the record this tool refuses. The wire the two share —
# the framing, the variable it pins, the content of an unset input, and each
# mark's locator, order and remedy — is one file both read, so the values cannot
# drift apart. What still needs pinning is that the file is whole, that this
# tool's values are its, and that the crate reads it rather than spelling the
# wire itself.
STAMP_CRATE = Path(__file__).resolve().parents[2] / "crates" / "symbiote-source-stamp"


class WireTests(unittest.TestCase):
    """The wire has one copy, and both readers take their values from it."""

    def test_the_checker_spells_the_wire_the_file_holds(self):
        scalars, marks, exclusions = read_wire(WIRE_FILE)
        self.assertEqual(RECORD_START.decode(), scalars["start"])
        self.assertEqual(RECORD_END.decode(), scalars["end"])
        self.assertEqual(ENVIRONMENT_PREFIX, scalars["prefix"])
        self.assertEqual(TOOLCHAIN_VARIABLE, scalars["environment"])
        self.assertEqual(UNSET, scalars["unset"])
        self.assertEqual(UNKNOWN_REMEDY, scalars["unknown"])
        self.assertEqual(MALFORMED_REMEDY, scalars["malformed"])
        self.assertEqual(UNDECODABLE_REMEDY, scalars["undecodable"])
        self.assertEqual(UNFRAMED_REMEDY, scalars["unframed"])
        self.assertIn(
            "{byte}",
            UNDECODABLE_REMEDY,
            "the file states where the offset it refuses a record for is spelled",
        )
        self.assertIn(
            "{encoding}",
            UNDECODABLE_REMEDY,
            "and where the encoding it names is, so no reader holds that sentence",
        )
        self.assertEqual(
            RECORD_ENCODING,
            "UTF-8",
            "the file states what a record's bytes are, so both readers decode them the "
            "same way",
        )
        self.assertEqual(
            WIRE["hash"],
            (64, "0123456789abcdef"),
            "a record gives an input a sha256, and what tells one from a mark is the "
            "file's to say rather than each reader's to decide",
        )
        self.assertEqual(
            list(MARKS.items()), [(locator, remedy) for _, locator, remedy in marks]
        )
        self.assertEqual(
            EXCLUSIONS,
            exclusions,
            "the walk's skip rule is the file's rather than this checker's own copy, "
            "and the file is refused unless it names both scopes a walk reads",
        )

    def test_the_marks_are_distinct_and_the_cargo_is_reported_first(self):
        _, marks, _ = read_wire(WIRE_FILE)
        roles = [role for role, _, _ in marks]
        locators = [locator for _, locator, _ in marks]
        self.assertEqual(len(roles), len(set(roles)), "a role is how the crate asks for a mark")
        self.assertEqual(len(locators), len(set(locators)))
        self.assertEqual(mark_locator("unasked-cargo"), UNASKED_CARGO)
        self.assertEqual(mark_locator("unnamed-resolution"), UNNAMED_RESOLUTION)
        self.assertEqual(
            list(MARKS)[0],
            UNASKED_CARGO,
            "the cargo is reported first: losing it can lose the resolution with it",
        )

    def test_a_wire_file_that_is_not_whole_is_refused_rather_than_defaulted(self):
        """Every value is one the crate spells the same record with.

        A framing this tool got wrong would make it read a record the crate did
        not write, so a missing keyword is an error rather than a fallback.
        """
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / "wire.txt"
            file.write_text("start\tone\nmark\trole\tlocator\tremedy\n")
            with self.assertRaises(ValueError):
                read_wire(file)

    def test_a_wire_file_naming_an_unknown_mark_order_this_checker_lacks_is_refused(self):
        """The order is the wire's, not a reader's own.

        Each reader used to order the marks the file does not name by itself —
        measured, the crate alphabetically and this checker by the record — so
        one record could be described two ways. A file that names an order this
        checker does not implement is refused rather than silently read the old
        way, and the file is the real one with only that word changed, so the
        refusal is the order's and nothing else's.
        """
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / "wire.txt"
            file.write_text(WIRE_FILE.read_text().replace("first-in-record", "last-in-record"))
            with self.assertRaises(ValueError):
                read_wire(file)

    def test_a_wire_file_naming_an_encoding_this_checker_lacks_is_refused(self):
        """The encoding is the wire's, not a reader's assumption.

        A file naming one this checker cannot decode a record by is refused rather
        than read as the default — the two readers have to decode the same bytes
        the same way — and the file is the real one with only that word changed, so
        the refusal is the encoding's and nothing else's.
        """
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / "wire.txt"
            file.write_text(WIRE_FILE.read_text().replace("encoding\tUTF-8", "encoding\tUTF-16"))
            with self.assertRaises(ValueError):
                read_wire(file)

    def test_the_crate_reads_the_wire_file_rather_than_spelling_it_itself(self):
        self.assertIn(
            'include_str!("wire.txt")',
            (STAMP_CRATE / "src" / "wire.rs").read_text(),
            "the crate embeds the one copy of the wire",
        )
        self.assertIn(
            "skips_directory",
            (STAMP_CRATE / "src" / "walk.rs").read_text(),
            "the walk skips what the wire file names rather than keeping its own list",
        )
        self.assertNotIn(
            RECORD_START.decode(),
            (STAMP_CRATE / "src" / "lib.rs").read_text(),
            "a copy of the wire in the library would be a second owner",
        )


class CargoInputTests(unittest.TestCase):
    """The inputs cargo reads rather than rustc, which no dep-info names.

    On a build-only checkout the completeness oracle above sees none of them,
    so without this half a record that stopped covering the workspace manifest
    would pass every proof and this check too.
    """

    def test_a_workspace_manifest_the_record_does_not_name_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            record = complete(fixture)
            del record["Cargo.toml"]
            self.assertEqual(
                fixture.problems(record),
                [
                    f"demo: {fixture.root / 'Cargo.toml'} is read by cargo to build this "
                    f"binary, but the record does not name it"
                ],
            )

    def test_a_lockfile_the_record_does_not_name_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            record = complete(fixture)
            del record["Cargo.lock"]
            self.assertEqual(
                fixture.problems(record),
                [
                    f"demo: {fixture.root / 'Cargo.lock'} is read by cargo to build this "
                    f"binary, but the record does not name it"
                ],
            )

    def test_a_closure_package_manifest_the_record_does_not_name_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            record = complete(fixture)
            del record["crates/demo/Cargo.toml"]
            self.assertEqual(
                fixture.problems(record),
                [
                    f"demo: {fixture.package / 'Cargo.toml'} is read by cargo to build this "
                    f"binary, but the record does not name it"
                ],
            )

    def test_an_absent_lockfile_is_not_required(self):
        # The record names what existed when it was built; a workspace without a
        # lockfile must not refuse a record for not naming one.
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            (fixture.root / "Cargo.lock").unlink()
            record = complete(fixture)
            del record["Cargo.lock"]
            self.assertEqual(fixture.problems(record), [])

    def test_a_registry_dependency_manifest_is_not_the_records_business(self):
        # A registry package is in the closure, but its sources are outside the
        # record by design — the lockfile it names pins them.
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryDirectory() as cache:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            registry = Path(cache) / "registry" / "serde" / "Cargo.toml"
            registry.parent.mkdir(parents=True)
            registry.write_text("[package]\nname = \"serde\"\n")
            metadata = fixture.metadata()
            identifier = f"serde 1.0.0 (registry+file://{registry.parent})"
            metadata["packages"].append(
                {
                    "name": "serde",
                    "id": identifier,
                    "manifest_path": str(registry),
                    "source": "registry+file:///registry",
                    "targets": [{"name": "serde", "kind": ["lib"]}],
                }
            )
            metadata["resolve"]["nodes"][0]["deps"].append(
                {"pkg": identifier, "dep_kinds": [{"kind": None}]}
            )
            metadata["resolve"]["nodes"].append({"id": identifier, "deps": []})
            self.assertEqual(fixture.problems(complete(fixture), metadata), [])

    def test_a_dependency_in_a_second_workspace_brings_its_root_manifest(self):
        # A path dependency may live in a workspace of its own, and then that
        # root manifest — not this workspace's — is where its `version.workspace`
        # and `edition.workspace` come from.
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryDirectory() as other:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            metadata, manifest, second = dependency_in_its_own_workspace(fixture, other)
            record = complete(fixture)
            record[str(manifest)] = "f" * 64
            record[str(second / "Cargo.toml")] = "1" * 64
            record[str(second / "Cargo.lock")] = "2" * 64
            self.assertEqual(fixture.problems(record, metadata), [])
            del record[str(second / "Cargo.toml")]
            self.assertEqual(
                fixture.problems(record, metadata),
                [
                    f"demo: {second / 'Cargo.toml'} is read by cargo to build this binary, "
                    f"but the record does not name it"
                ],
            )

    def test_a_dependency_in_a_second_workspace_brings_its_lockfile(self):
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryDirectory() as other:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            metadata, manifest, second = dependency_in_its_own_workspace(fixture, other)
            record = complete(fixture)
            record[str(manifest)] = "f" * 64
            record[str(second / "Cargo.toml")] = "1" * 64
            record[str(second / "Cargo.lock")] = "2" * 64
            del record[str(second / "Cargo.lock")]
            self.assertEqual(
                fixture.problems(record, metadata),
                [
                    f"demo: {second / 'Cargo.lock'} is read by cargo to build this binary, "
                    f"but the record does not name it"
                ],
            )

    def test_a_dependency_in_no_workspace_requires_only_its_manifest(self):
        # A package under no `[workspace]` manifest at all: nothing above it is
        # an input, so nothing above it may be required.
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryDirectory() as other:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            metadata, manifest, _ = dependency_in_its_own_workspace(fixture, other)
            (Path(other) / "Cargo.toml").write_text("[package]\nname = \"loose\"\n")
            record = complete(fixture)
            record[str(manifest)] = "f" * 64
            self.assertEqual(fixture.problems(record, metadata), [])

    def test_every_ancestor_workspace_manifest_is_required(self):
        # Cargo resolves a package to the workspace the build was invoked in,
        # which can be an ancestor of a nearer `[workspace]` manifest: measured,
        # a build invoked at the outer root read the outer manifest's
        # `[workspace.package]` values for both packages while the nearer
        # manifest was never loaded. The nearest rule named the nearer one and
        # left the manifest cargo read unrecorded.
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryDirectory() as other:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            metadata, manifest, outer, nested = dependency_below_another_workspace_manifest(
                fixture, other
            )
            record = complete(fixture)
            record[str(manifest)] = "f" * 64
            record[str(outer / "Cargo.toml")] = "1" * 64
            record[str(nested / "Cargo.toml")] = "2" * 64
            self.assertEqual(fixture.problems(record, metadata), [])
            del record[str(outer / "Cargo.toml")]
            self.assertEqual(
                fixture.problems(record, metadata),
                [
                    f"demo: {outer / 'Cargo.toml'} is read by cargo to build this binary, "
                    f"but the record does not name it"
                ],
            )

    def test_an_excluded_package_is_its_own_workspace_root(self):
        # Cargo does not claim a package its workspace `exclude`s — it reports
        # "failed to find a workspace root" for one that asks to inherit — so
        # the package's own directory is its workspace root and its own lockfile
        # is an input, while the excluding manifest is still read to learn that.
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryDirectory() as other:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            metadata, manifest, workspace = excluded_dependency(fixture, other)
            record = complete(fixture)
            record[str(manifest)] = "f" * 64
            record[str(manifest.parent / "Cargo.lock")] = "1" * 64
            record[str(workspace / "Cargo.toml")] = "2" * 64
            self.assertEqual(fixture.problems(record, metadata), [])
            del record[str(manifest.parent / "Cargo.lock")]
            self.assertEqual(
                fixture.problems(record, metadata),
                [
                    f"demo: {manifest.parent / 'Cargo.lock'} is read by cargo to build this "
                    f"binary, but the record does not name it"
                ],
            )

    def test_the_roots_of_a_package_are_what_cargo_can_resolve(self):
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryDirectory() as other:
            fixture = Fixture(directory)
            metadata, manifest, outer, nested = dependency_below_another_workspace_manifest(
                fixture, other
            )
            self.assertEqual(
                workspace_roots(manifest.parent),
                [outer.resolve()],
                "the named root is the only root a build of this package can read",
            )
            self.assertNotIn(
                nested.resolve(),
                workspace_roots(manifest.parent),
                "the nearer manifest the name settles past is not a root",
            )
            _, excluded, workspace = excluded_dependency(fixture, other)
            self.assertEqual(
                workspace_roots(excluded.parent),
                sorted({workspace.resolve(), excluded.parent.resolve()}),
                "the excluding manifest is read and the package is a workspace of one",
            )


class ManifestTests(unittest.TestCase):
    def test_the_workspace_key_is_read_as_the_scalar_cargo_spells(self):
        # `workspace = "../other"` in `[package]` names the root a package
        # inherits from, and cargo spells it as a plain string rather than an
        # inline table. `../other` is not an ancestor of the package, so only
        # the explicit key reaches it, and the header carries the spacing and
        # the comment TOML allows.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "app" / "crate"
            target = root / "app" / "other"
            (package / "src").mkdir(parents=True)
            (target / "src").mkdir(parents=True)
            (target / "Cargo.toml").write_text("[ workspace ] # the root\n")
            (target / "Cargo.lock").write_text("version = 4\n")
            manifest = package / "Cargo.toml"
            manifest.write_text(
                '[package]\nname = "app"\nversion = "0.1.0"\nworkspace = "../other" # the root\n'
            )
            self.assertEqual(
                workspace_roots(package),
                [target.resolve()],
                "the named root is the only root a build of this package can read",
            )
            self.assertEqual(
                nearest_workspace_root(package),
                target.resolve(),
                "and the base the record's locators are spelled against",
            )

    def test_the_root_a_crate_names_is_taken_from_the_one_reader_of_it(self):
        # The name a crate gives its root is one fact, and it has one owner: this
        # file used to carry its own scan of the key beside the toolchain rule's
        # reader, and the two answered differently on a name TOML resolves (an
        # escape, a multi-line string) and on a manifest cargo refuses. Measured
        # on the tree: no name in it read differently, so the unification is a
        # no-op today and only a second answer removed.
        # Whether a manifest *declares* a workspace of its own is still read by
        # the scan below: this file's two callers walk every ancestor, and their
        # tolerance of a manifest that cannot be parsed is what keeps a stray
        # manifest above the tree from refusing a record.
        import toolchains

        self.assertIs(named_workspace, toolchains.named_workspace,
                      "the source record takes the named root from the reader that owns it "
                      "rather than re-deriving it")
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            package = base / "app"
            (package / "src").mkdir(parents=True)
            (base / "root" / "src").mkdir(parents=True)
            (base / "root" / "Cargo.toml").write_text('[workspace]\nmembers = []\n')
            manifest = package / "Cargo.toml"
            # A name TOML resolves and a scan of the key does not: `\u006f` is the
            # escape for `o`, so the first name is `../root`, which the scan reads
            # as a sibling that does not exist; the multi-line string is read by
            # the scan as an empty name — the package's own directory.
            for spelling, written in (("an escape", 'workspace = "../r\\u006fot"'),
                                      ("a multi-line string",
                                       'workspace = """../root"""')):
                with self.subTest(named=spelling):
                    manifest.write_text(f'[package]\nname = "app"\n{written}\n')
                    self.assertEqual(nearest_workspace_root(package),
                                     (base / "root").resolve(),
                                     f"the base a record's locators are spelled against: {written}")
                    self.assertEqual(workspace_roots(package), [(base / "root").resolve()],
                                     f"and the only root a build of it can read: {written}")
            with self.subTest(named="a manifest cargo refuses"):
                manifest.write_text('[package]\nname = "app"\nworkspace = "../root"\n'
                                    'name = "dup"\n')
                with self.assertRaisesRegex(AssertionError, "is not TOML"):
                    nearest_workspace_root(package)

    def test_a_comment_naming_a_workspace_is_not_a_declaration(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / "Cargo.toml"
            manifest.write_text('[toolchain]\nchannel = "1.85.0" # [workspace]\n')
            self.assertFalse(declares_workspace(manifest.read_text()))


class ConfigurationTests(unittest.TestCase):
    def test_a_configuration_file_that_exists_and_is_unnamed_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            fixture.write("rust-toolchain.toml", "[toolchain]\nchannel = \"1.85.0\"\n")
            problems = fixture.problems(complete(fixture))
            self.assertEqual(len(problems), 1)
            self.assertIn(str(fixture.root / "rust-toolchain.toml"), problems[0])
            self.assertIn("the record does not name it", problems[0])

    def test_a_recorded_configuration_file_whose_content_moved_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            toolchain = fixture.write("rust-toolchain.toml", "[toolchain]\nchannel = \"1.85.0\"\n")
            record = complete(fixture)
            record["rust-toolchain.toml"] = "0" * 64
            problems = fixture.problems(record)
            self.assertEqual(
                problems, [f"demo: {toolchain} is not the content recorded"]
            )

    def test_a_recorded_configuration_file_that_has_gone_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            record = complete(fixture)
            record["rust-toolchain.toml"] = "0" * 64
            problems = fixture.problems(record)
            self.assertEqual(
                problems, [f"demo: {fixture.root / 'rust-toolchain.toml'} is recorded but no longer exists"]
            )

    def test_a_record_without_the_toolchain_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.unit("demo-9a", ["crates/demo/src/main.rs"])
            record = complete(fixture)
            del record["env:CARGO"]
            problems = fixture.problems(record)
            self.assertEqual(
                problems,
                ["demo: the record names no env:CARGO, so the toolchain that built it is not pinned"],
            )


if __name__ == "__main__":
    unittest.main()
