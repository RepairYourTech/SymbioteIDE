"""Run: python3 planning/integrity/test_source_record.py."""

import tempfile
import unittest
from pathlib import Path

from source_record import (
    RECORD_END,
    RECORD_START,
    check,
    declares_workspace,
    package_workspace,
    parse_dep_info,
    read_record,
    workspace_roots,
)


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
        self.write("Cargo.toml", "[workspace]\nmembers = [\"crates/demo\"]\n")
        self.write("Cargo.lock", "version = 4\n")
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
    two workspace directories above it. Cargo resolves such a package to the
    workspace the build was invoked in, which can be the outer one; the nearer
    manifest is never loaded, so the nearest-ancestor rule named it and left the
    manifest cargo read unrecorded.
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
    return {
        "crates/demo/src/main.rs": "a",
        "crates/demo/Cargo.toml": "b",
        "Cargo.toml": "d",
        "Cargo.lock": "e",
        "env:CARGO": "c",
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
            record[str(manifest)] = "f"
            record[str(second / "Cargo.toml")] = "g"
            record[str(second / "Cargo.lock")] = "h"
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
            record[str(manifest)] = "f"
            record[str(second / "Cargo.toml")] = "g"
            record[str(second / "Cargo.lock")] = "h"
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
            record[str(manifest)] = "f"
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
            record[str(manifest)] = "f"
            record[str(outer / "Cargo.toml")] = "g"
            record[str(nested / "Cargo.toml")] = "h"
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
            record[str(manifest)] = "f"
            record[str(manifest.parent / "Cargo.lock")] = "g"
            record[str(workspace / "Cargo.toml")] = "h"
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
                sorted({outer.resolve(), nested.resolve(), manifest.parent.resolve()}),
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
            self.assertEqual(package_workspace(manifest), "../other")
            self.assertEqual(
                workspace_roots(package),
                sorted({package.resolve(), target.resolve()}),
                "the workspace the package names is a root even where no ancestor is one",
            )

    def test_a_workspace_key_that_is_not_a_string_or_not_the_packages_is_not_followed(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / "Cargo.toml"
            manifest.write_text('[package]\nname = "app"\nworkspace = 3\n')
            self.assertIsNone(package_workspace(manifest))
            manifest.write_text(
                '[package]\nname = "app"\n\n[package.metadata]\nworkspace = "../other"\n'
            )
            self.assertIsNone(package_workspace(manifest))
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
