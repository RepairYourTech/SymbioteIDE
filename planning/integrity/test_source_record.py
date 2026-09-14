"""Run: python3 planning/integrity/test_source_record.py."""

import tempfile
import unittest
from pathlib import Path

from source_record import RECORD_END, RECORD_START, check, parse_dep_info, read_record


class Fixture:
    """A workspace with one package built into one binary that carries a record.

    Small enough to be read whole, and real enough to drive the checker: the
    dep-info files are the shape rustc writes, the metadata is the shape
    `cargo metadata` emits, and the record is the shape the binaries embed.
    """

    def __init__(self, directory, target="demo"):
        self.root = Path(directory)
        self.target_name = target
        self.target = self.root / "target" / "debug"
        (self.target / "deps").mkdir(parents=True)
        self.package = self.root / "crates" / "demo"
        self.write("crates/demo/Cargo.toml", "[package]\nname = \"demo\"\n")
        self.main = self.write("crates/demo/src/main.rs", "fn main() {}\n")

    def write(self, relative, content):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        return path

    def unit(self, stem, reads, artifact=None):
        """A dep-info file named after the unit, describing the artifact it names."""
        target = self.target / "deps" / (artifact or stem)
        body = f"{target}: " + " ".join(str(self.root / read) for read in reads) + "\n"
        (self.target / "deps" / f"{stem}.d").write_text(body)

    def binary(self, record, name=None):
        """A binary carrying `record`, as `locator -> hash`."""
        lines = "".join(f"{hash}\t{locator}\n" for locator, hash in record.items())
        path = self.target / (name or self.target_name)
        path.write_bytes(
            b"ELF\x00" + RECORD_START + b"\n" + lines.encode() + RECORD_END
        )
        return path

    def metadata(self):
        identifier = f"demo 0.1.0 (path+file://{self.package})"
        return {
            "packages": [
                {
                    "name": "demo",
                    "id": identifier,
                    "manifest_path": str(self.package / "Cargo.toml"),
                    "targets": [{"name": self.target_name, "kind": ["bin"]}],
                }
            ],
            "resolve": {"nodes": [{"id": identifier, "deps": []}]},
        }

    def problems(self, record):
        self.binary(record)
        found, _ = check(self.root, self.target, [("demo", self.target_name)], self.metadata())
        return found


def complete(fixture):
    """A record that covers what `fixture`'s unit reads, and its configuration."""
    return {"crates/demo/src/main.rs": "a", "crates/demo/Cargo.toml": "b", "env:CARGO": "c"}


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

    def test_a_read_in_an_excluded_directory_is_not_reported(self):
        # A test target's inputs are outside the record by its own rule, and a
        # refusal there could not be cleared by any rebuild.
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.write("crates/demo/tests/process.rs", "// a test\n")
            fixture.unit("demo-9a", ["crates/demo/src/main.rs", "crates/demo/tests/process.rs"])
            self.assertEqual(fixture.problems(complete(fixture)), [])

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
            self.assertEqual(fixture.problems(complete(fixture)), [])

    def test_a_hyphenated_unit_is_matched_by_its_file_name(self):
        # cargo spells a unit's dep-info file with underscores
        # (`demo_launch-9a.d`) and the artifact inside it with the target's own
        # hyphens (`demo-launch-9a`); only the file name identifies the unit.
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
