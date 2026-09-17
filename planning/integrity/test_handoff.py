"""Hold the engineering handoff's citations to the tree.

`docs/engineering-handoff.md` is the summary a maintainer reads first, and until this
file nothing read it: its links, the commands it prints and the one figure it states
were prose, so a moved file or a renamed example target would have left the document
naming something this tree does not have.

What is held, all of it the decidable part of that: every link it writes — inline,
titled, or reference-style, whose path lives in the definition and is held whether or
not a label uses it — resolves inside this repository; every code span that begins a
command names targets that exist, wherever the document writes it; every code span
naming a repository path exists; and the minimum toolchain it names is the one
`Cargo.toml` declares and CI's matrix runs. The document must still carry citations at
all, so an emptied one cannot pass by naming nothing.

What nothing here decides, and nothing in this repository can: whether a sentence's
meaning is true, which issue owns what work, any claim about a thread that lives off
this tree, and a second copy of these facts in another document — the rule reads this
one. The document's own convention that it cites no case name is prose here too, and
not holdable as this file holds things: refusing a span that is a name a suite's
source defines would refuse `apply` and `document`, two English words the handoff
writes and two suites define as helpers, so the rule would bite the words rather than
the citation it means.
"""
from __future__ import annotations

import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
DOCUMENT = ROOT / "docs/engineering-handoff.md"
COMMAND = re.compile(r"(cargo|python3?|symbiote)\b")


def text() -> str:
    return DOCUMENT.read_text()


def code_spans(document: str) -> list[str]:
    """Every code span the document writes, in order; the one reader the others share."""
    return re.findall(r"`([^`\n]+)`", document)


def link_targets(document: str) -> list[str]:
    """Every path the document cites, in either link form: no scheme, no anchor.

    An inline target may carry a title (`(path "title")`) or angle brackets; a
    reference link's path is its definition, so every definition is held whether or
    not a label uses it. A bracketed word with no definition is not a link in
    markdown, so it is not one here.
    """
    found = []
    for inside in re.findall(r"\]\(([^)]*)\)", document):
        found.append(inside[1:].split(">")[0] if inside.startswith("<")
                     else inside.split()[0])
    found += re.findall(r"^\[[^\]]+\]:\s*(\S+)", document, re.M)
    return [target for target in found
            if not re.match(r"[a-z][a-z0-9+.-]*:", target) and not target.startswith("#")]


def commands(document: str) -> list[str]:
    """Every command the document prints: a fenced line or a code span, wherever written."""
    fenced = [line.strip() for block in re.findall(r"```(?:sh)?\n(.*?)```", document, re.S)
              for line in block.strip().splitlines()]
    return [line for line in fenced + code_spans(document) if COMMAND.match(line)]


def path_spans(document: str) -> list[str]:
    """The code spans naming a repository path: a `/`, no space or placeholder, not absolute."""
    return sorted({span for span in code_spans(document)
                   if "/" in span and not any(c in span for c in " …<>*'\"")
                   and not span.startswith("/")})


def example_target(package: str, target: str) -> bool:
    """Whether a package carries that example: a manifest that names it, or a file of that name.

    The name is not always the file's: `constitution_report` lives in `examples/report.rs`
    and only the manifest says so, so a file-name-only reading would refuse a command
    that works. Measured both ways while writing this: the handoff's three `--example`
    commands exist, one of them by declaration rather than by file name.
    """
    manifest = ROOT / "crates" / package / "Cargo.toml"
    if manifest.is_file() and f'name = "{target}"' in manifest.read_text():
        return True
    return (ROOT / "crates" / package / "examples" / f"{target}.rs").is_file()


class HandoffCitations(unittest.TestCase):
    def test_every_local_link_resolves_inside_this_repository(self):
        links = link_targets(text())
        self.assertTrue(links, "the handoff names no path at all")
        for target in links:
            with self.subTest(target=target):
                resolved = (DOCUMENT.parent / target).resolve()
                self.assertTrue(resolved.is_relative_to(ROOT), f"{target} leaves the repository")
                self.assertTrue(resolved.exists(), f"{target} names nothing in this tree")

    def test_every_path_it_names_in_a_code_span_exists(self):
        spans = path_spans(text())
        self.assertTrue(spans, "the handoff names no path in a code span")
        for span in spans:
            with self.subTest(span=span):
                self.assertTrue((ROOT / span).exists(), f"{span} names nothing in this tree")

    def test_every_command_it_prints_names_something_that_exists(self):
        printed = commands(text())
        self.assertTrue(printed, "the handoff prints no command")
        for command in printed:
            with self.subTest(command=command):
                for script in re.findall(r"python3? (?:-m \S+ )?([\w./-]+\.py)", command):
                    self.assertTrue((ROOT / script).is_file(), f"{command}: {script} is not a file")
                for package, target in re.findall(r"cargo run -p (\S+) --example (\S+)", command):
                    self.assertTrue(example_target(package, target), f"{command}: no such example")
                for directory, pattern in re.findall(r"discover -s (\S+) -p (\S+)", command):
                    where = ROOT / directory
                    self.assertTrue(where.is_dir(), f"{command}: {directory} is not a directory")
                    self.assertTrue(list(where.glob(pattern.strip("'"))), f"{command}: matches nothing")
                for directory in re.findall(r"--check (\S+)", command):
                    self.assertTrue((ROOT / directory).exists(), f"{command}: {directory} is absent")

    def test_the_minimum_toolchain_it_names_is_the_declared_and_proven_one(self):
        named = re.search(r"Rust (\d+\.\d+)", text())
        self.assertIsNotNone(named, "the handoff no longer names a minimum toolchain")
        manifest = (ROOT / "Cargo.toml").read_text()
        declared = re.search(r'rust-version = "([^"]+)"', manifest)
        self.assertIsNotNone(declared, "Cargo.toml declares no rust-version, so the handoff's "
                                       "minimum has nothing to be held against")
        self.assertEqual(named.group(1), declared.group(1),
                         "the handoff's minimum is not the one Cargo.toml declares")
        matrix = re.search(r"toolchain: \[([^\]]+)\]",
                           (ROOT / ".github/workflows/rust-contracts.yml").read_text())
        self.assertIsNotNone(matrix, "CI no longer declares a toolchain matrix")
        proven = [entry.strip().strip('"') for entry in matrix.group(1).split(",")]
        self.assertIn(f"{declared.group(1)}.0", proven,
                      f"CI does not run the declared minimum: {proven}")
        self.assertIn("stable", proven, f"CI does not run stable: {proven}")


if __name__ == "__main__":
    unittest.main()
