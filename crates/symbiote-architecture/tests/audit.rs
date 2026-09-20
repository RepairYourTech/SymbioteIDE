//! This repository's own archival record (#173): the history its decisions were
//! reached through, the immutable copy of what they rest on, and the issue links
//! they carry.
//!
//! The real-tree tests assert the committed history, store and links against the
//! ledger. The fixture tests build a throwaway tree that is valid as written and
//! break exactly one thing at a time, so every refusal below is proved by the
//! rule it belongs to rather than by the suite as a whole.

use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use symbiote_architecture::checks::{Problem, governance};
use symbiote_architecture::journal::{GENESIS, JOURNAL_PATH, Journal, Transition};
use symbiote_architecture::ledger::{LEDGER_PATH, Ledger};
use symbiote_architecture::repository::digest;
use symbiote_architecture::roadmap::{Links, REGISTRY_PATH, Roadmap, links};
use symbiote_architecture::store::{STORE_DIR, cited, names};
use symbiote_architecture::workspace_root;

/// 2026-09-08 UTC, the day ADR-0001 and its ledger were recorded.
const NOW: u64 = 1_788_825_600;
const ID: &str = "TEST/HOST-STACK";
const RECORD: &str = "docs/architecture/record.md";
const EVIDENCE: &str = "docs/architecture/evidence.md";
const MEMBER: &str = "crates/member";

fn ledger() -> Ledger {
    Ledger::read(&workspace_root().join(LEDGER_PATH)).expect("the committed decision ledger")
}

fn committed_journal() -> Journal {
    Journal::read(&workspace_root().join(JOURNAL_PATH)).expect("the committed history")
}

#[test]
fn the_committed_history_store_and_links_pass_every_rule() {
    assert_eq!(
        governance(&ledger(), &workspace_root()),
        Vec::new(),
        "this repository's own archival record"
    );
}

#[test]
fn the_committed_history_recovers_what_the_ledger_publishes() {
    let ledger = ledger();
    let journal = committed_journal();
    let recovered = journal
        .replay(&ledger)
        .expect("the history recovers the published states");
    for record in &ledger.decisions {
        let id = record.draft.id.as_str();
        let reached = recovered.decision(id).expect("a recovered decision");
        assert_eq!(reached.state, record.published.state, "{id}");
        assert_eq!(reached.revision, record.published.revision, "{id}");
        let entries = journal.entries_for(id);
        assert_eq!(
            entries.first().map(|entry| entry.transition),
            Some(Transition::Proposed),
            "{id} is proposed before anything else happens to it"
        );
        assert_eq!(
            entries.len(),
            2,
            "{id} has one recorded transition per step"
        );
        assert_eq!(entries[0].from, 0);
        assert_eq!(entries[1].from, entries[0].to);
    }
    assert_eq!(
        journal.entries[0].prev, GENESIS,
        "the history begins the chain"
    );
    assert_eq!(
        journal.entries.last().expect("a head").to,
        2,
        "the committed head is the last transition recorded"
    );
}

#[test]
fn the_committed_store_holds_the_content_every_record_rests_on() {
    let root = workspace_root();
    let cited = cited(&ledger());
    assert!(!cited.is_empty(), "there is evidence to store");
    for (artifact, recorded) in &cited {
        let stored = std::fs::read(root.join(STORE_DIR).join(recorded)).expect("a stored copy");
        assert_eq!(&digest(&stored), recorded, "{artifact} is stored by digest");
        assert_eq!(
            digest(&std::fs::read(root.join(artifact)).expect("the working copy")),
            *recorded,
            "{artifact} is stored as the bytes it names"
        );
    }
    assert_eq!(
        names(&root).len(),
        cited.len(),
        "the store is exactly what is cited"
    );
}

#[test]
fn the_committed_links_resolve_in_the_issue_program() {
    let ledger = ledger();
    let roadmap =
        Roadmap::read(&workspace_root().join(REGISTRY_PATH)).expect("the generated registry");
    for record in &ledger.decisions {
        for issue in &record.draft.issue_refs {
            assert!(roadmap.issue(*issue).is_some(), "#{issue} is carried");
        }
        for key in &record.draft.requirement_refs {
            assert!(roadmap.requirement(key).is_some(), "{key} is carried");
        }
    }
    let shell: Links = links(&ledger, 38);
    assert_eq!(
        shell.decisions,
        vec![
            "ADR-0001/HOST-STACK".to_string(),
            "ADR-0001/DESKTOP-SHELL".to_string()
        ]
    );
    assert_eq!(shell.artifacts, ledger.artifacts.len());
    assert!(
        links(&ledger, 900).is_empty(),
        "an unlinked issue links nothing"
    );
}

#[test]
fn a_history_that_is_not_in_the_tree_is_refused() {
    let tree = Tree::new();
    std::fs::remove_file(tree.root.join(JOURNAL_PATH)).expect("the history is deleted");
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, JOURNAL_PATH);
    assert!(found[0].detail.contains("No such file"), "{:?}", found[0]);
}

#[test]
fn an_entry_edited_after_it_was_recorded_is_refused() {
    let tree = Tree::new();
    let mut journal = tree.journal();
    journal.entries[1].content_sha256 = journal.entries[0].content_sha256.clone();
    tree.write_journal(&journal);
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, JOURNAL_PATH);
    assert!(
        found[0]
            .detail
            .contains("was changed after it was recorded"),
        "{:?}",
        found[0]
    );
}

#[test]
fn an_entry_removed_from_the_middle_of_the_chain_is_refused() {
    let tree = Tree::new();
    let mut journal = tree.journal();
    journal.entries.remove(0);
    journal.entries[0].seq = 1;
    tree.write_journal(&journal);
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].detail.contains("does not continue the chain"),
        "{:?}",
        found[0]
    );
}

#[test]
fn entries_out_of_sequence_are_refused() {
    let tree = Tree::new();
    let mut journal = tree.journal();
    journal.entries[1].seq = 3;
    tree.write_journal(&journal);
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].detail.contains("out of order"), "{:?}", found[0]);
}

#[test]
fn a_proposal_whose_recorded_content_the_ledger_does_not_hold_is_refused() {
    let tree = Tree::new();
    let mut journal = tree.journal();
    journal.entries[0].content_sha256 = "0".repeat(64);
    journal.entries[0].sha256 = journal.entries[0].digest();
    journal.entries[1].prev = journal.entries[0].sha256.clone();
    journal.entries[1].sha256 = journal.entries[1].digest();
    tree.write_journal(&journal);
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, JOURNAL_PATH);
    assert!(
        found[0].detail.contains("holds different content for it"),
        "{:?}",
        found[0]
    );
}

#[test]
fn an_acceptance_recorded_at_another_time_than_the_ledger_holds_is_refused() {
    let tree = Tree::new();
    let mut journal = tree.journal();
    let last = journal.entries.len() - 1;
    journal.entries[last].at = Some(NOW + 1);
    journal.entries[last].sha256 = journal.entries[last].digest();
    tree.write_journal(&journal);
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].detail.contains("records the acceptance at"),
        "{:?}",
        found[0]
    );
}

#[test]
fn an_accepted_state_with_no_recorded_acceptance_is_refused() {
    let tree = Tree::new();
    let mut journal = tree.journal();
    journal.entries.truncate(1);
    tree.write_journal(&journal);
    let found = tree.audit();
    assert!(
        found.iter().any(|problem| problem
            .detail
            .contains("records no acceptance that reached it")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|problem| problem.detail.contains("does not recover")),
        "{found:?}"
    );
}

#[test]
fn a_transition_for_a_decision_the_ledger_does_not_hold_is_refused() {
    let tree = Tree::new();
    let mut journal = tree.journal();
    let mut extra = journal.entries[0].clone();
    extra.subject = "TEST/UNRECORDED".into();
    extra.seq = 3;
    extra.prev = journal.entries[1].sha256.clone();
    extra.sha256 = extra.digest();
    journal.entries.push(extra);
    tree.write_journal(&journal);
    let found = tree.audit();
    assert!(
        found
            .iter()
            .any(|problem| problem.detail.contains("does not hold")),
        "{found:?}"
    );
}

#[test]
fn a_decision_with_no_recorded_transition_is_refused() {
    let tree = Tree::new();
    tree.add_record("TEST/SECOND", json!([173]), false);
    let found = tree.audit();
    assert!(
        found.iter().any(|problem| problem
            .detail
            .contains("records no transition that proposed it")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|problem| problem.detail.contains("does not recover")),
        "{found:?}"
    );
}

#[test]
fn a_history_that_records_no_transition_at_all_is_refused() {
    let tree = Tree::new();
    tree.write_journal(&Journal {
        schema_version: 1,
        entries: Vec::new(),
    });
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].detail.contains("records no transition"),
        "{:?}",
        found[0]
    );
}

#[test]
fn a_journal_of_an_unknown_schema_is_refused() {
    let tree = Tree::new();
    let mut journal = tree.journal();
    journal.schema_version = 2;
    tree.write_journal(&journal);
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0]
            .detail
            .contains("unsupported decision journal schema"),
        "{:?}",
        found[0]
    );
}

#[test]
fn a_journal_with_a_field_it_does_not_hold_is_refused() {
    let tree = Tree::new();
    let mut value = tree.journal_value();
    value["records"] = json!([]);
    tree.write_json(JOURNAL_PATH, &value);
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].detail.contains("unknown field"), "{:?}", found[0]);
}

#[test]
fn a_cited_artifact_the_store_holds_no_copy_of_is_refused() {
    let tree = Tree::new();
    let (_, recorded) = tree.cited()[0].clone();
    std::fs::remove_file(tree.root.join(STORE_DIR).join(recorded)).expect("the copy is deleted");
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].detail.contains("holds no copy of"),
        "{:?}",
        found[0]
    );
}

#[test]
fn a_stored_copy_that_is_not_the_content_its_name_claims_is_refused() {
    let tree = Tree::new();
    let (_, recorded) = tree.cited()[0].clone();
    std::fs::write(tree.root.join(STORE_DIR).join(recorded), b"other bytes\n")
        .expect("the copy is replaced");
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].detail.contains("not the content its name claims"),
        "{:?}",
        found[0]
    );
}

#[test]
fn a_store_entry_no_record_cites_is_refused() {
    let tree = Tree::new();
    std::fs::write(tree.root.join(STORE_DIR).join("1".repeat(64)), b"orphan\n")
        .expect("an uncited entry");
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].detail.contains("no recorded decision cites"),
        "{:?}",
        found[0]
    );
}

#[test]
fn an_issue_the_program_does_not_carry_is_refused() {
    let tree = Tree::new();
    tree.add_decision("TEST/SECOND", json!([999]));
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, "TEST/SECOND: issue #999");
    assert!(
        found[0].detail.contains("carries no such issue"),
        "{:?}",
        found[0]
    );
}

#[test]
fn a_requirement_the_program_does_not_carry_is_refused() {
    let tree = Tree::new();
    let mut registry = tree.registry();
    registry["entries"][0]["key"] = json!("ZZZZ");
    tree.write_registry(&registry);
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, format!("{ID}: requirement A04"));
    assert!(
        found[0].detail.contains("carries no such requirement"),
        "{found:?}"
    );
}

#[test]
fn a_link_to_a_reference_entry_rather_than_the_work_it_points_at_is_refused() {
    let tree = Tree::new();
    let mut registry = tree.registry();
    registry["entries"][0]["kind"] = json!("reference");
    registry["entries"][0]["canonical_issue"] = json!(173);
    tree.write_registry(&registry);
    let found = tree.audit();
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found
            .iter()
            .all(|problem| problem.detail.contains("links the canonical")),
        "{found:?}"
    );
}

#[test]
fn a_record_waiting_on_a_closed_issue_is_refused() {
    let tree = Tree::new();
    tree.blocked_artifact(MEMBER, 173);
    let mut registry = tree.registry();
    registry["entries"][0]["state"] = json!("closed");
    tree.write_registry(&registry);
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, MEMBER);
    assert!(found[0].detail.contains("records closed"), "{:?}", found[0]);
}

#[test]
fn a_record_waiting_on_an_issue_the_program_does_not_carry_is_refused() {
    let tree = Tree::new();
    tree.blocked_artifact(MEMBER, 900);
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].detail.contains("does not carry"), "{:?}", found[0]);
}

#[test]
fn an_issue_program_that_cannot_be_read_is_refused_rather_than_passed() {
    let tree = Tree::new();
    std::fs::remove_file(tree.root.join(REGISTRY_PATH)).expect("the registry is deleted");
    let found = tree.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, REGISTRY_PATH);
    let empty = Tree::new();
    empty.write_registry(&json!({ "schema_version": 1, "entries": [] }));
    let found = empty.audit();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].detail.contains("carries no entry"),
        "{:?}",
        found[0]
    );
}

#[test]
fn the_writer_appends_one_entry_and_refuses_what_the_ledger_cannot_support() {
    let tree = Tree::new();
    let ledger = tree.ledger();
    let mut journal = tree.journal();
    let before = journal.entries.last().expect("a head").sha256.clone();
    let entry = journal
        .append(&ledger, Transition::Revised, ID, 2)
        .expect("an appended transition");
    assert_eq!(entry.seq, 3);
    assert_eq!(entry.to, 3);
    assert_eq!(entry.prev, before, "the new entry chains the old head");
    let refused = journal
        .replay(&ledger)
        .expect_err("an accepted decision cannot be revised");
    assert!(
        refused.0.contains("immutable"),
        "the replay refuses what the chain recorded: {refused}"
    );
    let unknown = journal
        .append(&ledger, Transition::Proposed, "TEST/UNKNOWN", 0)
        .expect_err("the ledger holds no such decision");
    assert!(unknown.0.contains("holds no decision"), "{unknown}");
    let tree = Tree::new();
    tree.add_record("TEST/UNCONFIRMED", json!([173]), false);
    let ledger = tree.ledger();
    let mut journal = tree.journal();
    let refused = journal
        .append(&ledger, Transition::Accepted, "TEST/UNCONFIRMED", 1)
        .expect_err("the ledger records no acceptance to record");
    assert!(refused.0.contains("records no acceptance"), "{refused}");
    journal
        .append(&ledger, Transition::Revised, "TEST/UNCONFIRMED", 1)
        .expect("an open choice can be revised");
}

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A throwaway tree holding one accepted decision whose record and evidence are
/// in the tree and in the store, a history that recovers it, and an issue
/// program that carries its links. Valid as written; each test breaks one thing.
struct Tree {
    root: PathBuf,
}

impl Tree {
    fn new() -> Tree {
        let root = std::env::temp_dir().join(format!(
            "symbiote-audit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(root.join(STORE_DIR)).expect("a store");
        std::fs::create_dir_all(root.join("planning/integrity/generated"))
            .expect("a generated program");
        let tree = Tree {
            root: root.canonicalize().expect("a canonical root"),
        };
        tree.write(RECORD, b"the accepted text these records interpret\n");
        tree.write(EVIDENCE, b"the evidence a record rests on\n");
        tree.write_json(
            LEDGER_PATH,
            &json!({
                "schema_version": 1,
                "decisions": [ tree.record(ID, json!([173]), true) ],
                "facts": [],
                "artifacts": [
                    {
                        "pins": {
                            "artifact": MEMBER,
                            "decisions": [{ "id": ID, "revision": 2 }],
                            "compatibility": []
                        },
                        "status": "settled"
                    }
                ]
            }),
        );
        tree.write_registry(&json!({
            "schema_version": 1,
            "entries": [{ "number": 173, "key": "A04", "kind": "task", "state": "open" }]
        }));
        tree.write_store();
        let ledger = tree.ledger();
        let mut journal = Journal {
            schema_version: 1,
            entries: Vec::new(),
        };
        journal
            .append(&ledger, Transition::Proposed, ID, 0)
            .expect("the proposal");
        journal
            .append(&ledger, Transition::Accepted, ID, 1)
            .expect("the acceptance");
        tree.write_journal(&journal);
        tree
    }

    /// One record, valid as written, at the given issue refs: accepted when the
    /// caller says so, and otherwise an open choice that names what decides it.
    fn record(&self, id: &str, issues: Value, accepted: bool) -> Value {
        let (record_sha256, evidence_sha256) = self.hashes();
        let mut value = json!({
            "published": if accepted {
                json!({ "state": "accepted", "revision": 2 })
            } else {
                json!({ "state": "investigating", "revision": 2 })
            },
            "draft": {
                "schema_version": 1,
                "id": id,
                "revision": 1,
                "state": if accepted { "proposed" } else { "investigating" },
                "title": "A stack constraint",
                "constraints": ["Rust"],
                "alternatives": ["another language"],
                "consequences": ["the Host is Rust"],
                "security": "the client holds no canonical state",
                "reversal": "replacing it would rewrite the Host",
                "high_reversal_cost": false,
                "issue_refs": issues,
                "requirement_refs": ["A04"],
                "graph_refs": [],
                "deployment_refs": [],
                "compatibility": [],
                "evidence": [{
                    "artifact": EVIDENCE,
                    "sha256": evidence_sha256,
                    "method": "recorded",
                    "observed_at": NOW,
                    "reviewer": "client"
                }],
                "supersedes": null
            },
            "record": RECORD,
            "record_sha256": record_sha256
        });
        if accepted {
            value["acceptance"] = json!({
                "actor": "client",
                "authority": "client",
                "now": NOW,
                "client_exception": "the client accepted this by instruction"
            });
        } else {
            value["blocking_issue"] = json!(173);
        }
        value
    }

    /// A second decision, recorded in the ledger and in the history, so only the
    /// link under test can refuse.
    fn add_decision(&self, id: &str, issues: Value) {
        self.add_record(id, issues, true);
        let ledger = self.ledger();
        let mut journal = self.journal();
        journal
            .append(&ledger, Transition::Proposed, id, 0)
            .expect("the proposal");
        journal
            .append(&ledger, Transition::Accepted, id, 1)
            .expect("the acceptance");
        self.write_journal(&journal);
    }

    /// A second decision recorded in the ledger alone: the history does not
    /// reach it, which is what a decision nobody recorded looks like.
    fn add_record(&self, id: &str, issues: Value, accepted: bool) {
        let record = self.record(id, issues, accepted);
        let mut value = self.value();
        value["decisions"]
            .as_array_mut()
            .expect("decisions")
            .push(record);
        self.write_json(LEDGER_PATH, &value);
    }

    /// Give an artifact a blocking issue the ledger must carry.
    fn blocked_artifact(&self, member: &str, issue: u64) {
        let mut value = self.value();
        for artifact in value["artifacts"].as_array_mut().expect("artifacts") {
            if artifact["pins"]["artifact"] == json!(member) {
                artifact["status"] = json!("provisional");
                artifact["blocking_issue"] = json!(issue);
            }
        }
        self.write_json(LEDGER_PATH, &value);
    }

    fn hashes(&self) -> (String, String) {
        (
            digest(&std::fs::read(self.root.join(RECORD)).expect("the record")),
            digest(&std::fs::read(self.root.join(EVIDENCE)).expect("the evidence")),
        )
    }

    fn value(&self) -> Value {
        serde_json::from_slice(&std::fs::read(self.root.join(LEDGER_PATH)).expect("the ledger"))
            .expect("readable ledger")
    }

    fn journal_value(&self) -> Value {
        serde_json::from_slice(&std::fs::read(self.root.join(JOURNAL_PATH)).expect("the history"))
            .expect("readable history")
    }

    fn registry(&self) -> Value {
        serde_json::from_slice(&std::fs::read(self.root.join(REGISTRY_PATH)).expect("the registry"))
            .expect("readable registry")
    }

    fn cited(&self) -> Vec<(String, String)> {
        cited(&self.ledger())
    }

    fn ledger(&self) -> Ledger {
        Ledger::read(&self.root.join(LEDGER_PATH)).expect("the fixture ledger")
    }

    fn journal(&self) -> Journal {
        Journal::read(&self.root.join(JOURNAL_PATH)).expect("the fixture history")
    }

    /// Copy the record and the evidence into the store under their digests.
    fn write_store(&self) {
        for (artifact, recorded) in self.cited() {
            std::fs::copy(
                self.root.join(&artifact),
                self.root.join(STORE_DIR).join(recorded),
            )
            .expect("the stored copy");
        }
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        std::fs::write(path, bytes).expect("a fixture file");
    }

    fn write_json(&self, relative: &str, value: &Value) {
        self.write(
            relative,
            &serde_json::to_vec_pretty(value).expect("readable JSON"),
        );
    }

    fn write_journal(&self, journal: &Journal) {
        journal
            .write(&self.root.join(JOURNAL_PATH))
            .expect("the fixture history");
    }

    fn write_registry(&self, registry: &Value) {
        self.write_json(REGISTRY_PATH, registry);
    }

    fn audit(&self) -> Vec<Problem> {
        let ledger = self.ledger();
        governance(&ledger, &self.root)
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
