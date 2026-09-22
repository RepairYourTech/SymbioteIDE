#!/usr/bin/env python3
"""Every refusal the licensing, trust and data-ownership record must survive, in both directions.

Each case mutates a copy of the committed record — or gives the reader a tree that states the fact
the case is about — and requires the refusal to land naming its subject, with an honest edit of the
same shape staying green. The record is never written to and no case reaches the network: the
products come from #171's committed registry, the program from the generated registry, and the tests
read this tree.

    python3 -m unittest discover -s planning/policy -p 'test_*.py' -v
"""
from __future__ import annotations

import copy
import pathlib
import re
import sys
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))

import policy  # noqa: E402

README = HERE / "README.md"


def document() -> dict:
    return copy.deepcopy(policy.record())


def check(record: dict, tree: policy.Tree | None = None) -> list[str]:
    return policy.problems(record, tree or policy.Tree(), policy.program_issues(),
                           policy.universe_sources())


def row(entries: list[dict], subject: str, key: str = "id") -> dict:
    for entry in entries:
        if entry.get(key) == subject:
            return entry
    raise AssertionError(f"no row named {subject!r} in the committed record")


def obligation(record: dict, name: str) -> dict:
    return row(record["obligations"], name)


class TheCommittedRecord(unittest.TestCase):
    """The record as committed survives every rule, and the two readings the issue's own criteria
    are about are the file's own facts rather than prose about them."""

    def test_the_committed_record_survives_every_rule(self):
        self.assertEqual(check(document()), [])

    def test_the_declared_dependencies_are_the_ones_the_tree_declares(self):
        """The dependency table is read back from the manifests: every third-party dependency a
        crate declares is recorded once, under the class the widest table it appears in implies, and
        the record carries none the tree does not declare."""
        tree = policy.Tree()
        declared: dict[str, str] = {}
        for manifest in tree.manifests().values():
            for name, kind in manifest["declared"].items():
                if name in declared and policy.WIDEST.index(declared[name]) <= policy.WIDEST.index(kind):
                    continue
                declared[name] = kind
        rows = policy.record()["supply_chain"]["dependencies"]
        recorded = {entry["name"]: entry["class"] for entry in rows}
        self.assertEqual(sorted(recorded), sorted(declared),
                         "the dependency table is not the set of third-party dependencies this "
                         "workspace declares")
        for name, kind in sorted(declared.items()):
            self.assertEqual(recorded[name], kind,
                             f"{name} is recorded under {recorded[name]} and the manifests declare "
                             f"it as {kind}")
        self.assertTrue(rows, "no dependency is recorded, so the boundary binds nothing")

    def test_the_declared_artifacts_are_written_by_programs_this_tree_carries(self):
        """Every generated-artifact row names a path this tree carries, a generator that is a file,
        a re-derivation command whose path tokens exist, and a case a file declares."""
        tree = policy.Tree()
        rows = policy.record()["supply_chain"]["generated"]
        self.assertTrue(rows, "no generated artifact is declared")
        for entry in rows:
            for path in entry["paths"]:
                self.assertTrue(tree.exists(path), f"{path} is declared and this tree lacks it")
            self.assertTrue(tree.at(entry["generator"]).is_file(),
                            f"{entry['generator']} is named as a generator and is not a file")
            self.assertIn(entry["check"], tree.cases(),
                          f"{entry['check']} is named as the holding case and no file declares it")

    def test_the_readme_states_the_numbers_this_rule_measures(self):
        """The README's figures are the record's, read from the file rather than restated: a
        boundary added, a dependency dropped or an obligation built moves both or neither."""
        record = policy.record()
        text = README.read_text()
        figures = {
            "artifact classes": len(record["classes"]),
            "obligations": len(record["obligations"]),
            "contract documents": len(record["stability"]["contracts"]),
            "dependencies": len(record["supply_chain"]["dependencies"]),
            "generated artifacts": len(record["supply_chain"]["generated"]),
            "universe products": len(record["runtime_assumptions"]),
            # The transitive closure is the lock's own fact: the entries it records with a source
            # are the registry and git packages, and the rest are this workspace's own.
            "packages": sum(1 for entry in re.findall(r"\[\[package\]\]\n(.*?)(?=\n\[\[|\Z)",
                                                    (ROOT / "Cargo.lock").read_text(),
                                                    flags=re.S)
                            if "\nsource = " in entry),
        }
        for what, count in figures.items():
            self.assertRegex(text, rf"\*\*{count}\*\*\s+{re.escape(what)}",
                             f"the README does not state {count} {what}, which is what the record "
                             f"holds")


class TheFiveBoundaries(unittest.TestCase):
    """A licence is either selected with the tree carrying it, or pending with an owner and a
    reason; a boundary states what it covers and what it does not."""

    def test_a_class_selecting_a_licence_the_tree_does_not_carry_is_refused(self):
        record = document()
        core = row(record["classes"], "core")
        core["status"], core["license"] = "selected", "MIT"
        self.assertTrue(any("carries no LICENSE" in one for one in check(record)),
                        "a class selecting MIT with no LICENSE file is not refused")

    def test_a_tree_carrying_a_licence_no_class_selects_is_refused(self):
        with tempfile.TemporaryDirectory() as where:
            tree = pathlib.Path(where)
            (tree / "LICENSE").write_text("MIT License\n")
            self.assertTrue(any("does not account for" in one for one in check(document(), policy.Tree(tree))),
                            "a LICENSE file no class selects is not refused")

    def test_a_class_with_no_boundary_is_refused(self):
        record = document()
        row(record["classes"], "extension")["boundary"] = "   "
        self.assertTrue(any("states no boundary" in one for one in check(record)),
                        "a class that states no exclusion is not refused")

    def test_a_pending_class_with_no_owner_or_reason_is_refused(self):
        record = document()
        core = row(record["classes"], "core")
        core["owners"], core["why"] = [], "  "
        found = check(record)
        self.assertTrue(any("names no owner" in one for one in found))
        self.assertTrue(any("states no reason" in one for one in found))

    def test_a_class_owned_by_an_issue_the_program_does_not_carry_is_refused(self):
        record = document()
        row(record["classes"], "core")["owners"] = [999999]
        self.assertTrue(any("#999999" in one for one in check(record)),
                        "a class owned by an issue the program lacks is not refused")

    def test_an_artifact_class_the_issue_names_and_the_record_omits_is_refused(self):
        record = document()
        record["classes"] = [entry for entry in record["classes"]
                             if entry["id"] != "catalog_metadata"]
        self.assertTrue(any("catalog_metadata" in one for one in check(record)),
                        "a class the issue names and the record omits is not refused")

    def test_an_honest_boundary_edit_stays_green(self):
        for mutation in ("reword", "reorder", "extra"):
            record = document()
            if mutation == "reword":
                row(record["classes"], "core")["covers"] = "The Host, the CLI and every crate."
            elif mutation == "reorder":
                record["classes"] = list(reversed(record["classes"]))
            else:
                row(record["classes"], "hosted_service")["covers"] += " It also covers the " \
                    "terms of use, which are separate from the code."
            self.assertEqual(check(record), [], f"an honest {mutation} of the boundaries reds")


class TheObligations(unittest.TestCase):
    """A claim of `built` needs the tree to show it and a case to exercise it; a deferral needs an
    owner and a reason; an inapplicable obligation needs a rationale."""

    def test_an_obligation_the_issue_names_and_no_row_accounts_for_is_refused(self):
        record = document()
        record["obligations"] = [entry for entry in record["obligations"]
                                 if entry["id"] != "extension.revocation"]
        self.assertTrue(any("extension.revocation" in one for one in check(record)),
                        "an obligation the issue names and no row accounts for is not refused")

    def test_a_built_obligation_with_no_evidence_in_the_tree_is_refused(self):
        record = document()
        obligation(record, "data.local-first-ownership")["evidence"] = "docs/contracts/absent.md"
        self.assertTrue(any("claims what nothing shows" in one for one in check(record)),
                        "a built obligation whose evidence path is absent is not refused")

    def test_a_built_obligation_naming_a_case_no_file_declares_is_refused(self):
        record = document()
        obligation(record, "data.local-first-ownership")["case"] = "a_case_that_does_not_exist"
        self.assertTrue(any("nothing exercises it" in one for one in check(record)),
                        "a built obligation naming a case no file declares is not refused")

    def test_a_deferred_obligation_with_no_owner_or_reason_is_refused(self):
        record = document()
        entry = obligation(record, "data.telemetry-opt-in")
        entry["owners"], entry["why"] = [], ""
        found = check(record)
        self.assertTrue(any("names no owner" in one for one in found))
        self.assertTrue(any("states no reason" in one for one in found))

    def test_a_deferral_naming_an_issue_the_program_does_not_carry_is_refused(self):
        record = document()
        obligation(record, "data.telemetry-opt-in")["owners"] = [999999]
        self.assertTrue(any("#999999" in one for one in check(record)),
                        "a deferral to an issue the program lacks is not refused")

    def test_an_obligation_that_is_both_built_and_deferred_is_refused(self):
        record = document()
        entry = obligation(record, "data.local-first-ownership")
        entry["owners"] = [174]
        self.assertTrue(any("deferred and done at once" in one for one in check(record)),
                        "an obligation claiming to be built and deferred is not refused")
        record = document()
        entry = obligation(record, "data.telemetry-opt-in")
        entry["evidence"] = "docs/contracts/storage.md"
        self.assertTrue(any("claims to be built while deferring it" in one for one in check(record)),
                        "a deferral citing evidence is not refused")

    def test_a_not_applicable_obligation_with_no_rationale_is_refused(self):
        record = document()
        entry = obligation(record, "data.telemetry-opt-in")
        entry.pop("owners")
        entry["why"] = ""
        entry["status"], entry["rationale"] = "not_applicable", ""
        self.assertTrue(any("states no rationale" in one for one in check(record)),
                        "an inapplicable obligation with no rationale is not refused")

    def test_an_obligation_with_no_requirement_or_area_is_refused(self):
        record = document()
        entry = obligation(record, "publishing.trademark")
        entry["requirement"], entry["area"] = "", "legal"
        found = check(record)
        self.assertTrue(any("states no requirement" in one for one in found))
        self.assertTrue(any("not an area of this issue" in one for one in found))

    def test_an_honest_obligation_edit_stays_green(self):
        for mutation in ("reword", "reorder", "defer more"):
            record = document()
            if mutation == "reword":
                obligation(record, "publishing.trademark")["requirement"] = \
                    "What may be called Symbiote and what a pack may claim."
            elif mutation == "reorder":
                record["obligations"] = list(reversed(record["obligations"]))
            else:
                entry = obligation(record, "publishing.fork")
                entry["status"], entry["owners"] = "pending", [174]
                entry["why"] = "still #174's own work"
                entry.pop("evidence", None)
                entry.pop("case", None)
            self.assertEqual(check(record), [], f"an honest {mutation} of the obligations reds")


class TheStability(unittest.TestCase):
    """Every public contract document carries a level, every level states its rule, and a promise of
    stability is not made by a version that has not shipped."""

    def test_a_contract_document_no_row_holds_is_refused(self):
        record = document()
        record["stability"]["contracts"] = [entry for entry in record["stability"]["contracts"]
                                            if entry["doc"] != "docs/contracts/protocol.md"]
        self.assertTrue(any("protocol.md" in one for one in check(record)),
                        "a contract document no row holds is not refused")

    def test_a_stability_row_for_a_document_the_tree_lacks_is_refused(self):
        record = document()
        record["stability"]["contracts"].append({"doc": "docs/contracts/absent.md",
                                                 "surface": "rpc"})
        self.assertTrue(any("does not carry" in one for one in check(record)),
                        "a stability row for a document the tree lacks is not refused")

    def test_a_stable_level_below_version_one_is_refused(self):
        record = document()
        record["stability"]["default_level"] = "stable"
        self.assertTrue(any("cannot support" in one for one in check(record)),
                        "a stable level under a 0.x workspace version is not refused")

    def test_a_level_with_no_rule_is_refused(self):
        record = document()
        row(record["stability"]["levels"], "experimental")["rule"] = ""
        self.assertTrue(any("states no rule" in one for one in check(record)),
                        "a level with no rule is not refused")

    def test_an_honest_stability_edit_stays_green(self):
        for mutation in ("reword", "move one contract", "reorder the contracts"):
            record = document()
            if mutation == "reword":
                row(record["stability"]["levels"], "experimental")["rule"] = \
                    "Anything may change; nothing is promised."
            elif mutation == "move one contract":
                row(record["stability"]["contracts"], "docs/contracts/cli.md", key="doc")["surface"] = \
                    "tooling"
            else:
                record["stability"]["contracts"] = \
                    list(reversed(record["stability"]["contracts"]))
            self.assertEqual(check(record), [], f"an honest {mutation} of the levels reds")


class TheDistributionBoundary(unittest.TestCase):
    """What the tree ships is permissive; what only builds or tests may be copyleft; and the class a
    dependency is filed under is the class its licence has to survive."""

    def test_a_dependency_the_manifests_do_not_declare_is_refused(self):
        record = document()
        record["supply_chain"]["dependencies"].append({"name": "leftpad", "license": "MIT",
                                                       "class": "runtime"})
        self.assertTrue(any("leftpad" in one for one in check(record)),
                        "a dependency the manifests do not declare is not refused")

    def test_a_dependency_the_manifests_declare_and_no_row_records_is_refused(self):
        record = document()
        record["supply_chain"]["dependencies"] = [
            entry for entry in record["supply_chain"]["dependencies"] if entry["name"] != "serde"]
        self.assertTrue(any("serde" in one for one in check(record)),
                        "a declared dependency with no licence recorded is not refused")

    def test_a_dependency_filed_under_a_narrower_class_is_refused(self):
        record = document()
        row(record["supply_chain"]["dependencies"], "tempfile", key="name")["class"] = "runtime"
        self.assertTrue(any("tempfile" in one and "manifests declare it as test" in one
                            for one in check(record)),
                        "a dev-only dependency filed as runtime is not refused")

    def test_a_copyleft_licence_reaching_the_runtime_class_is_refused(self):
        record = document()
        row(record["supply_chain"]["dependencies"], "tauri", key="name")["license"] = "GPL-3.0-only"
        self.assertTrue(any("tauri" in one and "runtime class" in one for one in check(record)),
                        "a copyleft licence on a shipped dependency is not refused")

    def test_a_denied_only_licence_is_refused(self):
        record = document()
        row(record["supply_chain"]["dependencies"], "serde", key="name")["license"] = "AGPL-3.0-only"
        self.assertTrue(any("serde" in one and "denied" in one for one in check(record)),
                        "a licence this boundary refuses outright is not refused")

    def test_an_unknown_licence_id_is_refused(self):
        record = document()
        row(record["supply_chain"]["dependencies"], "sha2", key="name")["license"] = "MIT-X"
        self.assertTrue(any("MIT-X" in one for one in check(record)),
                        "a licence id this reader does not know is not refused")

    def test_a_licence_both_permitted_and_denied_is_refused(self):
        record = document()
        record["supply_chain"]["denied"].append("MPL-2.0")
        self.assertTrue(any("both permitted and denied" in one for one in check(record)),
                        "a licence both permitted and denied is not refused")

    def test_a_class_no_licence_is_permitted_for_is_refused(self):
        record = document()
        del record["supply_chain"]["permitted"]["build"]
        self.assertTrue(any("no licence is permitted for the build" in one for one in check(record)),
                        "a dependency class with no permitted licence is not refused")

    def test_a_generated_artifact_the_tree_does_not_carry_is_refused(self):
        record = document()
        record["supply_chain"]["generated"][0]["paths"] = ["planning/integrity/generated/absent.json"]
        self.assertTrue(any("absent.json" in one for one in check(record)),
                        "a generated artifact the tree lacks is not refused")

    def test_a_generated_artifact_naming_a_generator_the_tree_lacks_is_refused(self):
        record = document()
        record["supply_chain"]["generated"][0]["generator"] = "planning/integrity/absent.py"
        self.assertTrue(any("not a file this tree carries" in one for one in check(record)),
                        "a generator the tree lacks is not refused")

    def test_a_generated_artifact_held_by_a_case_no_file_declares_is_refused(self):
        record = document()
        record["supply_chain"]["generated"][0]["check"] = "no_such_case_anywhere"
        self.assertTrue(any("no_such_case_anywhere" in one for one in check(record)),
                        "a generated artifact held by a case that does not exist is not refused")

    def test_a_rerun_command_naming_a_path_the_tree_lacks_is_refused(self):
        record = document()
        record["supply_chain"]["generated"][0]["rerun"] = \
            "python3 planning/integrity/absent.py --write"
        self.assertTrue(any("absent.py" in one for one in check(record)),
                        "a re-derivation command naming a path the tree lacks is not refused")

    def test_an_honest_supply_chain_edit_stays_green(self):
        for mutation in ("reorder", "reword a rerun", "license the same expression another way"):
            record = document()
            if mutation == "reorder":
                record["supply_chain"]["dependencies"] = \
                    list(reversed(record["supply_chain"]["dependencies"]))
            elif mutation == "reword a rerun":
                record["supply_chain"]["generated"][0]["rerun"] = \
                    "python3 planning/integrity/validate.py <snapshot> -o planning/integrity/generated"
            else:
                row(record["supply_chain"]["dependencies"], "serde", key="name")["license"] = \
                    "Apache-2.0 OR MIT"
            self.assertEqual(check(record), [], f"an honest {mutation} of the boundary reds")


class TheApplicability(unittest.TestCase):
    """Security, privacy, accessibility, performance and cross-platform, each marked once."""

    def test_a_concern_marked_nowhere_is_refused(self):
        record = document()
        record["applicability"] = [entry for entry in record["applicability"]
                                   if entry["concern"] != "performance"]
        self.assertTrue(any("performance" in one for one in check(record)),
                        "a concern marked nowhere is not refused")

    def test_a_required_concern_naming_an_obligation_that_does_not_exist_is_refused(self):
        record = document()
        row(record["applicability"], "security", key="concern")["obligation"] = "data.absent"
        self.assertTrue(any("data.absent" in one for one in check(record)),
                        "a required concern naming no obligation is not refused")

    def test_a_separately_tracked_concern_with_no_owner_is_refused(self):
        record = document()
        entry = row(record["applicability"], "accessibility", key="concern")
        entry["owners"] = [999999]
        self.assertTrue(any("#999999" in one for one in check(record)),
                        "a separately tracked concern owned by nothing real is not refused")

    def test_a_mark_outside_the_vocabulary_is_refused(self):
        record = document()
        row(record["applicability"], "privacy", key="concern")["mark"] = "ignored"
        self.assertTrue(any("ignored" in one for one in check(record)),
                        "an applicability mark outside the three the issue names is not refused")

    def test_an_honest_applicability_edit_stays_green(self):
        record = document()
        row(record["applicability"], "accessibility", key="concern")["owners"] = [358, 354]
        self.assertEqual(check(record), [], "an honest second owner of a tracked concern reds")


class TheRuntimeAssumptions(unittest.TestCase):
    """The licence, authentication and telemetry assumption about every product in #171's universe,
    stated: an unknown names its gap rather than standing in for an answer, and a value needs the
    registry to hold a source behind it."""

    def test_a_product_in_the_universe_with_no_assumption_is_refused(self):
        record = document()
        record["runtime_assumptions"] = [entry for entry in record["runtime_assumptions"]
                                         if entry["product"] != "cursor"]
        self.assertTrue(any("cursor" in one for one in check(record)),
                        "a universe product with no assumption recorded is not refused")

    def test_an_unstated_field_is_refused(self):
        record = document()
        row(record["runtime_assumptions"], "orca", key="product")["telemetry"] = ""
        self.assertTrue(any("unstated" in one for one in check(record)),
                        "a blank assumption is not refused")

    def test_an_unknown_with_no_gap_is_refused(self):
        record = document()
        entry = row(record["runtime_assumptions"], "orca", key="product")
        entry["gaps"] = ["license"]
        self.assertTrue(any("names no gap" in one for one in check(record)),
                        "an unknown with no gap named is not refused")

    def test_a_stated_value_the_registry_holds_no_source_for_is_refused(self):
        record = document()
        row(record["runtime_assumptions"], "cursor", key="product")["license"] = "MIT"
        self.assertTrue(any("no source" in one for one in check(record)),
                        "a licence asserted with no source behind it is not refused")

    def test_a_row_for_a_product_the_universe_lacks_is_refused(self):
        record = document()
        record["runtime_assumptions"].append({"product": "not-released", "license": "UNKNOWN",
                                              "authentication": "UNKNOWN", "telemetry": "UNKNOWN",
                                              "gaps": ["license", "authentication", "telemetry"]})
        self.assertTrue(any("not-released" in one for one in check(record)),
                        "an assumption about a product outside the universe is not refused")

    def test_an_honest_assumption_edit_stays_green(self):
        record = document()
        entry = row(record["runtime_assumptions"], "orca", key="product")
        entry["authentication"] = "UNKNOWN"
        entry["gaps"] = ["license", "authentication", "telemetry"]
        self.assertEqual(check(record), [], "a re-stated unknown reds")

    def test_the_audit_reading_is_the_records_own_debt(self):
        """`--audit` prints the fields still unknown per product and the obligations not built, so
        the unstated-assumption debt is readable rather than inferred."""
        report = policy.audit(policy.record(), policy.universe_sources())
        self.assertEqual(len(report["assumptions"]),
                         len(policy.record()["runtime_assumptions"]))
        self.assertEqual(report["unknown_fields"],
                         sum(len(one["unknown"]) for one in report["assumptions"]))
        self.assertTrue(report["unbuilt"], "the audit reports every obligation as built")
        self.assertEqual(sorted(one["obligation"] for one in report["unbuilt"]),
                         sorted(entry["id"] for entry in policy.record()["obligations"]
                                if entry["status"] != "built"))

    def test_the_class_reading_is_the_records_own_boundaries(self):
        report = policy.class_report(policy.record())
        self.assertEqual([one["class"] for one in report],
                         [entry["id"] for entry in policy.record()["classes"]])
        for one in report:
            self.assertTrue(one["covers"] and one["boundary"],
                            f"{one['class']} prints without a boundary")


if __name__ == "__main__":
    unittest.main()
