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
    directory,
    floor,
    gated,
    jobs,
    legs,
    manifests,
    named_workspace,
    pairs,
    version,
    workspace_jobs,
)


def relative(path: pathlib.Path) -> str:
    return str(path.relative_to(ROOT))


def unrunnable(block: str) -> list[str]:
    """The legs a job's block gates steps on and does not itself run: this rule's comparison."""
    return [leg for leg in gated(block) if leg not in legs(block)]


def floors_dropped(workflow: str, declared: str) -> list[str]:
    """The jobs of a workflow that do not run that floor: this rule's comparison, per job."""
    return [name for name, block in jobs(workflow).items() if version(declared) not in legs(block)]


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
            with self.subTest(workflow=workflow, job=job):
                self.assertEqual(unrunnable(block), [],
                                 f"{workflow}: the job {job} gates steps on a leg it does not "
                                 f"run {legs(block)}, so those steps would never run")

    def test_a_gate_written_over_two_lines_is_the_gate_the_rule_checks(self):
        """Measured before the gate was read through the reader: the single-line spelling was
        seen and the folded one was not, so a job gating on a leg it does not run passed.

        The condition is read off the lines the reader states the entry spans, not off the words
        it states: those are stripped of their quotes (`'stable'` reads `stable`), which loses
        the closing quote the pattern needs — the reading that reds this case and the live one.
        One boundary is stated rather than closed: a line inside a command's own block scalar
        that is written `if: matrix.toolchain == 'stable'` is read as a gate (measured), where a
        parser reads it as part of the command — the reader reads text, and the raw text read
        this shape too, so nothing widened here.
        """
        matrix = "    strategy:\n      matrix:\n        toolchain: [1.85.0]\n"
        for written in ("        if: matrix.toolchain == 'stable'\n",
                        "        if: matrix.toolchain ==\n          'stable'\n"):
            with self.subTest(written=written):
                block = matrix + "    steps:\n      - run: cargo test --workspace\n" + written
                self.assertEqual(unrunnable(block), ["stable"],
                                 "the job gates steps on the stable leg and runs "
                                 f"{legs(block)}, so those steps would never run")


class EveryWayAJobInheritsItsBlock(unittest.TestCase):
    """A job or a step stated as an alias is that job or step, and a merge states the keys it
    merges — read the way both parsers read them, and refused by name where this reader cannot
    resolve them.

    Measured before this: an alias job named its job and read **no block at all** (its legs, its
    steps and its floor unchecked), and a merge read as the key it is, so a job that inherited its
    proving step was not counted as proving the workspace. Both were silent. Every state below is
    driven through this rule's own comparison, so a spelling that stops being read reds by name
    rather than passing quietly. A forward reference, another document's anchor and a stream of two
    documents are refused by name, which is what both parsers do with them.
    """

    STEPS = "      - run: cargo test --workspace\n"

    def job(self, block: str, name: str = "build") -> str:
        return "name: w\non: push\njobs:\n" + block

    def test_a_job_stated_as_an_alias_runs_what_its_anchor_runs(self):
        anchored = ("  other: &o\n    strategy:\n      matrix:\n        toolchain: [1.85.0]\n"
                    "    steps:\n" + self.STEPS)
        workflow = self.job(anchored + "  build: *o\n")
        self.assertEqual(list(jobs(workflow)), ["other", "build"], "the alias names a job")
        self.assertEqual(legs(jobs(workflow)["build"]), ["1.85.0"],
                         "a job that inherits a 1.85.0 step runs it")
        self.assertIn(jobs(workflow)["build"], workspace_jobs(workflow),
                      "and its inherited step proves the workspace")
        self.assertEqual(floors_dropped(workflow, "1.85.0"), [], "nothing is dropped here")

    def test_an_alias_job_whose_anchor_drops_the_floor_is_reported(self):
        anchored = ("  other: &o\n    strategy:\n      matrix:\n        toolchain: [stable]\n"
                    "    steps:\n" + self.STEPS)
        workflow = self.job(anchored + "  build: *o\n")
        self.assertEqual(floors_dropped(workflow, "1.85.0"), ["other", "build"],
                         "both the anchored job and the alias of it run only stable, "
                         "and both are seen")

    def test_a_merge_states_the_keys_it_merges_and_the_jobs_own_key_wins(self):
        base = ("  base: &base\n    strategy:\n      matrix:\n        toolchain: [1.85.0]\n"
                "    runs-on: x\n")
        merged = self.job(base + "  build:\n    <<: *base\n    steps:\n" + self.STEPS)
        self.assertEqual(legs(jobs(merged)["build"]), ["1.85.0"], "the merged leg is run")
        self.assertIn(jobs(merged)["build"], workspace_jobs(merged),
                      "a job that inherits its proving step proves the workspace")
        overridden = self.job(base + "  build:\n    <<: *base\n    strategy:\n      matrix:\n"
                                     "        toolchain: [stable]\n")
        self.assertEqual(legs(jobs(overridden)["build"]), ["stable"],
                         "the job's own key wins over the merge, as both parsers read it")
        self.assertEqual(floors_dropped(overridden, "1.85.0"), ["build"],
                         "so the floor it drops is the one it states, and it is seen")

    def test_the_first_merge_of_a_list_wins(self):
        workflow = self.job(
            "  a: &a\n    strategy:\n      matrix:\n        toolchain: [1.85.0]\n"
            "  b: &b\n    strategy:\n      matrix:\n        toolchain: [stable]\n"
            "  build:\n    <<: [*a, *b]\n    steps:\n" + self.STEPS)
        self.assertEqual(legs(jobs(workflow)["build"]), ["1.85.0"],
                         "the earlier mapping's key wins, as both parsers read it")

    def test_a_step_stated_as_a_merge_is_the_step_it_merges(self):
        workflow = self.job("  build:\n    runs-on: x\n    steps:\n      - <<: *s\n"
                            ) .replace("jobs:\n", "x-step: &s\n  run: cargo test --workspace\njobs:\n")
        self.assertIn(jobs(workflow)["build"], workspace_jobs(workflow),
                      "the merged step is the run the rule reads")

    def test_a_job_stated_as_an_alias_of_a_one_line_mapping_is_named(self):
        """Measured: an alias whose anchored mapping is written on one line named its job and read
        no block, and then named no job at all, where both parsers read the mapping — a job that
        runs only `stable` went unreported. The line naming the job states the block it is, so the
        reading reaches the aliased job however briefly its anchor is written.
        """
        one = self.job("  other: &o\n    strategy: {matrix: {toolchain: [stable]}}\n"
                       "  build: *o\n")
        self.assertEqual(list(jobs(one)), ["other", "build"], "both jobs are named")
        self.assertEqual(legs(jobs(one)["build"]), ["stable"], "and the alias runs its anchor's leg")
        self.assertEqual(floors_dropped(one, "1.85.0"), ["other", "build"],
                         "so a job that drops the floor is reported, not lost")

    def test_an_alias_with_a_trailing_comment_is_the_job_it_repeats(self):
        """A comment after an alias is what a parser reads past, and what this reader read as no
        alias: measured, the job named there ran no leg of its own and was reported as dropping
        the floor both parsers read it running.
        """
        anchored = ("  other: &o\n    strategy:\n      matrix:\n        toolchain: [1.85.0]\n"
                    "    steps:\n" + self.STEPS)
        workflow = self.job(anchored + "  build: *o  # the same job\n")
        self.assertEqual(legs(jobs(workflow)["build"]), ["1.85.0"],
                         "the alias runs the leg its anchor states")
        self.assertIn(jobs(workflow)["build"], workspace_jobs(workflow),
                      "and the step it inherits proves the workspace")
        self.assertEqual(floors_dropped(workflow, "1.85.0"), [],
                         "so a job that drops nothing is not reported as dropping it")

    def test_a_merge_of_a_mapping_that_itself_merges_is_resolved(self):
        """Measured: a merge was resolved from the anchored text as written, so a base whose own
        block merges another base — and an anchor written before `jobs:` that merges one — was
        refused by name where both parsers read the keys, reding a workflow that proves what it
        always did.
        """
        nested = self.job("  a: &a\n    strategy:\n      matrix:\n        toolchain: [1.85.0]\n"
                          "  b: &b\n    <<: *a\n    runs-on: x\n"
                          "  build:\n    <<: *b\n    steps:\n" + self.STEPS)
        self.assertEqual(legs(jobs(nested)["b"]), ["1.85.0"],
                         "the job that merges a base runs the leg it inherits")
        self.assertEqual(legs(jobs(nested)["build"]), ["1.85.0"],
                         "and so does the job that merges that job")
        self.assertIn(jobs(nested)["build"], workspace_jobs(nested),
                      "with the proving step it inherits through both")
        outside = ("x-a: &a\n  strategy:\n    matrix:\n      toolchain: [1.85.0]\n"
                   "x-b: &b\n  <<: *a\n  runs-on: x\njobs:\n"
                   "  build:\n    <<: *b\n    steps:\n" + self.STEPS)
        self.assertEqual(legs(jobs(outside)["build"]), ["1.85.0"],
                         "an anchor written before `jobs:` resolves its own merge too")

    def test_a_merge_or_an_alias_inside_the_block_it_names_states_nothing(self):
        """A self-merge and a self-alias are what both parsers read as the mapping being written:
        the keys a merge would bring are that mapping's own, and the value an alias names is the
        one it is part of — neither states a key anew, and neither is refused.
        """
        merged = self.job("  build: &b\n    <<: *b\n    runs-on: x\n    steps:\n" + self.STEPS)
        self.assertEqual(list(jobs(merged)), ["build"], "the job is named")
        self.assertIn(jobs(merged)["build"], workspace_jobs(merged),
                      "and runs the step it states beside the merge")
        aliased = self.job("  build: &b\n    runs-on: x\n    steps: *b\n")
        self.assertEqual(list(jobs(aliased)), ["build"], "a self-alias states no step")
        self.assertNotIn(jobs(aliased)["build"], workspace_jobs(aliased),
                         "so the job proves nothing")

    def test_a_job_stated_as_a_scalar_alias_is_named(self):
        """Measured: `build: *v` naming a one-line scalar read as no job where both parsers read
        the job's key with a scalar — a job key that names no job is a job no rule checks.
        """
        text = "name: w\non: push\nx: &v 1.85.0\njobs:\n  build: *v\n"
        self.assertEqual(list(jobs(text)), ["build"], "the job is named")
        self.assertEqual(legs(jobs(text)["build"]), [], "and states no legs of its own")
        self.assertNotIn(jobs(text)["build"], workspace_jobs(text), "nor a step that runs one")

    def test_an_anchor_or_a_key_stated_twice_is_refused_by_name(self):
        """Measured: a name stated twice is refused by PyYAML and ruamel alike, and a key stated
        twice in one mapping is refused by ruamel while PyYAML warns `found duplicate key` and
        keeps the last. Reading both values is reading a document no workflow can be, so both are
        refused here by name rather than read.
        """
        for block, named in (("  base: &v\n    runs-on: x\n  build: &v\n    runs-on: y\n",
                              "anchor &v twice"),
                             ("  build:\n    runs-on: x\n    runs-on: y\n",
                              "states runs-on twice")):
            with self.subTest(named=named):
                with self.assertRaisesRegex(AssertionError, named):
                    jobs(self.job(block))

    def test_an_anchor_before_jobs_states_what_its_alias_and_its_merge_inherit(self):
        held = ("x-base: &b\n  strategy:\n    matrix:\n      toolchain: [1.85.0]\n"
                "jobs:\n  build:\n    <<: *b\n    steps:\n" + self.STEPS)
        self.assertEqual(legs(jobs(held)["build"]), ["1.85.0"],
                         "an anchor written before `jobs:` is one both parsers resolve")

    def test_an_alias_or_a_merge_this_reader_cannot_resolve_is_refused_by_name(self):
        for block, named in (("  build: *o\n  other: &o\n    runs-on: x\n", "no anchor named o"),
                             ("  build: *nope\n", "no anchor named nope"),
                             ("  build:\n    <<: *later\n  later: &later\n    runs-on: x\n",
                              "no anchor named later")):
            with self.subTest(block=block):
                with self.assertRaisesRegex(AssertionError, named):
                    jobs(self.job(block))
        across = ("x: &o\n  runs-on: x\n---\njobs:\n  build: *o\n")
        with self.assertRaisesRegex(AssertionError, "more than one document"):
            jobs(across)


class EveryJobAWorkflowWrites(unittest.TestCase):
    """A job key read as no job is a job no rule checks, silently, and that is what each state
    below measured against PyYAML and ruamel, which agree on every text here: a quoted key, a
    dotted key, a key with a trailing comment and a key carrying an anchor were each read as no
    job, and so were a whole `jobs:` mapping written on the key's line, a job whose mapping is
    written on its own key's line, a quoted `jobs:` key, and a step list written in flow form.

    What stays outside a reader of text, each measured: a job key indented with a tab reads as no
    job while both parsers refuse the workflow outright, so nothing a parser can read goes
    unchecked there — loud, twice over; a `jobs:` mapping written across two lines is refused by
    name where both parsers read it — loud; an anchor inside a flow mapping, and an anchor on an
    item's own value, are refused by name where both parsers resolve them — loud, and no workflow
    in this tree writes one. A job whose value is not a mapping — a list, or a scalar an alias
    names — is named here and states no block, where both parsers read the key with a list or a
    scalar: a job GitHub would refuse to run, so nothing it would run goes unchecked. A name or a
    key stated twice is refused by name, which is what ruamel reads and PyYAML reads as
    `found duplicate key` and a last value kept.

    An alias is the job it repeats and a merge states the keys it merges, read where the anchor is
    written and resolved however deeply it merges or names itself — `EveryWayAJobInheritsItsBlock`
    holds each of those, including an alias whose anchored mapping is written on one line and one
    carrying a trailing comment.
    """

    def test_a_jobs_mapping_written_on_one_line_states_the_jobs_in_it(self):
        workflow = ("name: w\non: push\njobs: {build: {runs-on: x, strategy: {matrix: "
                    "{toolchain: [stable]}}, steps: [{run: cargo test --workspace}]}}\n")
        self.assertEqual(list(jobs(workflow)), ["build"], "the mapping is the jobs it holds")
        self.assertEqual(legs(jobs(workflow)["build"]), ["stable"])
        self.assertEqual(workspace_jobs(workflow), [jobs(workflow)["build"]],
                         "its flow step list runs the workspace's own tests")
        self.assertEqual(floors_dropped(workflow, "1.85.0"), ["build"],
                         "it runs only stable, so the declared minimum is dropped and seen")

    def test_a_job_written_on_its_key_line_is_the_same_job(self):
        workflow = ("name: w\non: push\njobs:\n  build: {runs-on: x, strategy: {matrix: "
                    "{toolchain: [stable]}}, steps: [{run: cargo test --workspace}]}\n")
        self.assertEqual(list(jobs(workflow)), ["build"], "the mapping is written on the key")
        keeps = workflow.replace("toolchain: [stable]", "toolchain: [1.85.0]")
        self.assertEqual(floors_dropped(keeps, "1.85.0"), [],
                         "the leg it runs is the floor it is checked against")
        self.assertEqual(floors_dropped(workflow, "1.85.0"), ["build"],
                         "and the floor it drops is seen rather than passed over")

    def test_a_step_list_in_flow_form_states_the_steps_a_parser_reads(self):
        proving = ("name: w\non: push\njobs:\n  build:\n    runs-on: x\n    steps: [{run: "
                   "cargo test --workspace}]\n")
        self.assertTrue(workspace_jobs(proving), "the one step runs the workspace's own tests")
        self.assertEqual(workspace_jobs(proving.replace("cargo test", "cargo clippy")), [],
                         "a clippy step proves nothing")

    def test_a_quoted_jobs_key_and_a_quoted_job_name_still_name_the_job(self):
        workflow = ('name: w\non: push\n"jobs":\n  "build":\n    runs-on: x\n    strategy:\n'
                    "      matrix:\n        toolchain: [stable]\n")
        self.assertEqual(list(jobs(workflow)), ["build"])
        self.assertEqual(floors_dropped(workflow, "1.85.0"), ["build"],
                         "a quoted key names the same job, whose dropped floor is seen")

    def test_a_job_key_is_read_however_yaml_writes_one(self):
        for written, name in (("  build:\n", "build"),
                              ("  build_one:\n", "build_one"),
                              ("  build-one:\n", "build-one"),
                              ("  build.one:\n", "build.one"),
                              ('  "build":\n', "build"),
                              ("  build: # why\n", "build"),
                              ("  build: &anchor\n", "build")):
            with self.subTest(written=written):
                workflow = "name: w\non: push\njobs:\n" + written + "    runs-on: ubuntu-latest\n"
                self.assertEqual(list(jobs(workflow)), [name],
                                 "a job key YAML writes, read as no job when it is not read")
                self.assertEqual(jobs(workflow)[name], "    runs-on: ubuntu-latest\n",
                                 "the key's own block is the lines under it")


class EverySpellingOfOneStatement(unittest.TestCase):
    """Each state below was measured against a reader that read it wrong before it was held
    here: a crate named only as `--manifest-path=X` was not found at all, a floor written
    `rust-version="1.85.0"` read as none, an inherited floor came from the wrong manifest's
    table, a matrix written as a block sequence read as running no toolchain, a commented
    table header read as no table, one step's directory decided every other step, a templated
    one silently meant the repository root,    a step installing a toolchain beside a matrix hid
    it, and a step's command written under `run: |` or split across lines with a trailing
    backslash was read as proving nothing at all. The last case here drives those two one
    level out: the live workflow contains neither shape, which is why they are written in the
    case rather than taken from the tree. A gate written over two lines was read as no gate at
    all, and a job key YAML writes — quoted, dotted, commented, anchored — as no job.

    Whether a value is a command decides three of these readings, all measured against PyYAML
    6.0.3 and ruamel, which agree on every text below. `run` is the only entry a shell runs, and
    it is read the way a shell reads it: a trailing `\\` is a continuation both parsers keep and
    the shell drops (`'cargo test \\ --workspace --locked'` is `cargo test --workspace --locked`
    to a shell), and a blank line ends the command, since a shell runs the lines either side
    separately (`'cargo test\n--workspace --locked'`). Every value the rules read on its own — a
    matrix leg, a `working-directory` — is read the way a parser states it: the marker is
    the literal character YAML says it is (`'1.85.0 \\ stable'` and `'crates/ \\ examples'`, both
    exactly what a parser reads, and each one value rather than a list of words), and a blank
    line is a paragraph break the value goes on past. A `--manifest-path` is a token *of* a
    command, so it is read where a shell passes it: the marker ends the token, and
    `--manifest-path=crates/ \\` with `examples/Cargo.toml` under it reads `crates/` — the word a
    shell passes, and cargo fails to find a manifest for either reading.

    One residual is stated rather than closed: a paragraph break reads as a space here, where
    both parsers carry a newline (`'crates/ \\nexamples'`), so a directory or a toolchain folded
    over a blank line is one value either way but not one string.

    The other direction is measured too, because a row asserting a reading for a text no parser
    accepts looks like coverage and is not. Five texts no workflow can contain are read anyway,
    silently, since this reads text rather than parsing it — a block scalar whose content sits at
    its key's own column (the entry states no word), and a comment line inside a folded plain
    scalar, a folded line carrying `: `, a tab-indented continuation and a continuation less
    indented than its key (each ends the entry where it sits); PyYAML 6.0.3 and ruamel both
    refuse all five. No row below asserts a reading for one: sweeping every text this file
    writes through the reader the suite drives finds exactly one text both parsers refuse — the
    unclosed `toolchain: [` that this file asserts is *refused*, not read — and no text where
    the two parsers disagree.
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

    def test_a_root_a_crate_names_itself_is_read_as_cargo_reads_it(self):
        """The key is TOML, so it is read as TOML: an escaped name and a multi-line string
        each name the root they state, where the text scan this replaced read a sibling that
        does not exist and the crate's own directory. A manifest cargo refuses is refused by
        name rather than read as naming no root — the direction that matters, a name settling
        where a crate's workspace is. Measured on the tree: no name in it moves either way.
        """
        with tempfile.TemporaryDirectory() as directory:
            nested = pathlib.Path(directory) / "nested"
            (nested / "root").mkdir(parents=True)
            (nested / "root" / "Cargo.toml").write_text('[workspace]\nmembers = []\n')
            member = nested / "member" / "Cargo.toml"
            member.parent.mkdir(parents=True)
            named_root = (nested / "root").resolve()
            for spelling, stated in (("a plain name", 'workspace = "../root"'),
                                     ("a name with a comment",
                                      'workspace = "../root" # the root'),
                                     ("a single-quoted name", "workspace = '../root'"),
                                     ("a name in a table spelled with spaces",
                                      'workspace = "../root"'),
                                     ("an escaped name", 'workspace = "../r\\u006fot"'),
                                     ("a multi-line name", 'workspace = """../root"""')):
                with self.subTest(named=spelling):
                    header = "[ package ]" if spelling.endswith("spaces") else "[package]"
                    member.write_text(f'{header}\nname = "member"\n{stated}\n')
                    self.assertEqual(named_workspace(member), named_root,
                                     f"cargo reads this name as TOML: {stated}")
            for spelling, stated in (("a name that is not a string", "workspace = 3"),
                                     ("a name under another table",
                                      '[package.metadata]\nworkspace = "../root"'),
                                     ("a name only in a comment",
                                      '# workspace = "../root"')):
                with self.subTest(named=spelling):
                    member.write_text(f'[package]\nname = "member"\n{stated}\n')
                    self.assertIsNone(named_workspace(member),
                                      "cargo spells the root as a plain string under the "
                                      "package's own table, so nothing else is a name to follow")
            with self.subTest(named="a manifest cargo refuses"):
                member.write_text('[package]\nname = "member"\nworkspace = "../root"\n'
                                  'name = "dup"\n')
                with self.assertRaisesRegex(AssertionError, "is not TOML"):
                    named_workspace(member)
            with self.subTest(named="a root that holds no manifest"):
                member.write_text('[package]\nname = "member"\nworkspace = "../nowhere"\n'
                                  'rust-version.workspace = true\n')
                with self.assertRaisesRegex(AssertionError, "holds no Cargo.toml"):
                    floor(member)

    def test_a_directory_is_read_from_the_whole_value_it_states(self):
        """Measured against the reader before this held it: a `working-directory` written over
        two lines resolved its first word alone, marker and all (`crates/ \\`), while the same
        entry read by the rule that finds the proving job was joined — two consumers of one
        stated value disagreeing about what it says, in the one that decides which crate a
        step builds. A path is not a command, so it is read the way both PyYAML and ruamel state
        it: a plain scalar runs onto the lines deeper than its key, a quoted one states itself,
        a `\\` in it is the literal character YAML says it is — the marker row below reads
        `crates/ \\ examples`, which is exactly what both parsers read — and a blank line is a
        paragraph break the value goes on past, where both parsers read
        `'crates/ \\\nexamples'`: the same value with a newline where this joins lines with a
        space, which is the one residual this reads unlike a parser.
        """
        for written, resolved in (("      - working-directory: crates/ \\\n          examples\n",
                                  ROOT / r"crates/ \ examples"),
                                 ("      - working-directory: crates/\n          examples\n",
                                  ROOT / "crates/ examples"),
                                 ('      - working-directory: "crates/ examples"\n',
                                  ROOT / "crates/ examples")):
            with self.subTest(written=written):
                self.assertEqual(directory(f"    steps:\n{written}        run: cargo test\n",
                                           "a job"),
                                 resolved,
                                 "every word the entry states, as its own form reads it")
        blank = ("    steps:\n      - working-directory: crates/ \\\n\n"
                 "          examples\n        run: cargo test\n")
        self.assertEqual(directory(blank, "a job"), ROOT / r"crates/ \ examples",
                         "a blank line is the paragraph a value goes on past, not its end: both "
                         "parsers read `crates/ \\\nexamples`")

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
                # a toolchain is not a command: a `\` there is the literal YAML says it is, and
                # the one value both parsers read is one leg rather than its words
                (matrix + "        toolchain: 1.85.0 \\\n          stable\n",
                 [r"1.85.0 \ stable"]),
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
        """One command written inline, folded onto the next line (with or without a backslash),
        under `run: |`, in a flow-mapping step, on two lines of a block scalar, split across
        lines with a trailing backslash, or with the step's `name` after it, is one command.
        A block scalar is the lines deeper than its key's own column rather than the dash's, so
        a step's own `name`, `if` or `with` is never part of its command, while a line of the
        scalar's *content* that looks like a key stays content: measured, PyYAML 6.0.3 and
        ruamel both read `make: all` under `run: |` as part of the command and a sibling `name`
        as a key of its own, and before the key's column was read instead of the dash's, a
        sibling `name` carrying the command made a job that only echoes count as proving the
        workspace. Refused rather than read: a flow mapping written across lines (`- {run:
        cargo test,` then `name: proofs}`), which both parsers accept and this reader names as
        "a mapping this reader does not read across lines" — loud, and no workflow in this tree
        writes one.
        Measured: with the entry's own words matched one at a time instead of joined, the
        `run: |` form finds no proving job; before `entries` read a shell continuation the
        backslash form found none; and before it read a plain scalar's folded continuation a
        command written over two lines found none — each reding the handoff's case for a
        workflow that proves exactly what it always did.
        """
        for written in ("      - run: cargo test --workspace --locked\n",
                        "      - run: cargo test --workspace\n          --locked\n",
                        "      - run: cargo test\n          --workspace --locked\n",
                        "      - run: |\n          cargo test --workspace --locked\n",
                        "      - run: >\n          cargo test --workspace --locked\n",
                        "      - {run: cargo test --workspace --locked}\n",
                        "      - run: |\n          make: all\n"
                        "          cargo test --workspace --locked\n",
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
                      "      - run: cargo test --locked\n",
                      # a command is a shell's: a blank line runs the lines either side
                      # separately, so the second is not part of the command that proves it
                      "      - run: cargo test\n\n          --workspace --locked\n",
                      "      - run: |\n          echo not the workspace tests\n"
                      "        name: cargo test --workspace --locked\n"):
            with self.subTest(other=other):
                self.assertEqual(workspace_jobs(f"jobs:\n  proofs:\n    steps:\n{other}"), [],
                                 "only one entry stating `cargo test` and `--workspace` proves it")


    def test_a_run_a_job_states_is_not_a_step_that_proves_the_workspace(self):
        """Measured: a `run` key written at a job's own level — which a merge of a step sequence,
        or an alias to one, writes there — and a job whose own block is a sequence of mappings are
        each read by both parsers as a `run` key and no `steps` at all: a job that runs no step,
        so it proves nothing here. Only a job's own steps are asked for.
        """
        for block, why in (("  build:\n    <<: *s\n    runs-on: x\n",
                             "a merged step sequence states no step"),
                            ("  build: *s\n", "neither does an alias to one"),
                            ("  build:\n    - run: cargo test --workspace --locked\n",
                             "nor a job whose block is a list")):
            with self.subTest(why=why):
                text = ("name: w\non: push\nx: &s\n  - run: cargo test --workspace --locked\n"
                        "jobs:\n" + block)
                self.assertEqual(workspace_jobs(text), [], why)


if __name__ == "__main__":
    unittest.main()
