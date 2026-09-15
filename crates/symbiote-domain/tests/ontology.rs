//! The canonical vocabulary and the criteria of #36 that are structural.
//!
//! Every check here is a rule over the inventory this crate compiles and the
//! artifact it publishes, so a noun that gains a second owner, an identity that
//! arrives with no owner, an entity that is declared but not transportable, a
//! record no noun owns, a canonical type nobody owns, or a published document
//! that no longer matches the tree each fails a named test rather than a review.
//! The rules range over the committed artifact, not over a document assembled in
//! the test, so what they hold true is what was published. The criteria this
//! layer cannot check are asserted to say so, with the issue that owns them.

use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;

macro_rules! id {
    ($kind:ident, $value:expr) => {
        $kind::new($value).unwrap()
    };
}

fn provenance() -> Provenance {
    Provenance {
        created_at: Timestamp(1),
        updated_at: Timestamp(2),
        actor: Actor::User(id!(UserId, "human")),
        external_references: Vec::new(),
    }
}

/// The committed artifact, as the document the rules are checked against.
fn artifact() -> serde_json::Value {
    let text = std::fs::read_to_string(workspace_root().join(ONTOLOGY_SCHEMA_PATH))
        .expect("the committed ontology");
    serde_json::from_str(&text).expect("the committed ontology is JSON")
}

fn published() -> BTreeSet<String> {
    published_names(&artifact())
}

fn records() -> BTreeSet<String> {
    recorded_names(&artifact())
}

fn violations(vocabulary: &[Noun], outstanding: &[Outstanding]) -> Vec<String> {
    problems(vocabulary, outstanding, IDENTITY_NAMES, &artifact())
}

fn definition(name: &str) -> serde_json::Value {
    ontology_schema()["$defs"][name].clone()
}

fn properties(name: &str) -> BTreeSet<String> {
    definition(name)["properties"]
        .as_object()
        .expect("a closed object")
        .keys()
        .cloned()
        .collect()
}

fn required(name: &str) -> BTreeSet<String> {
    definition(name)["required"]
        .as_array()
        .expect("a record requires its fields")
        .iter()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect()
}

const fn entity(noun: &'static str, identity: &'static str, owner: &'static str) -> Noun {
    Noun {
        noun,
        identity: Some(identity),
        owner,
        shape: Shape::Entity,
        lifecycle: None,
    }
}

/// #36's first acceptance criterion, plus the rules that keep the vocabulary from
/// passing while empty or while pointing at types that do not exist.
#[test]
fn every_noun_maps_to_one_identity_and_one_published_owner() {
    let found = violations(VOCABULARY, OUTSTANDING);
    assert!(found.is_empty(), "{found:#?}");
    // A rule with nothing to check is not evidence, so the shapes it ranges over
    // are asserted to be the size the inventory actually is.
    assert!(VOCABULARY.len() >= 60, "{}", VOCABULARY.len());
    assert!(IDENTITY_NAMES.len() >= 55, "{}", IDENTITY_NAMES.len());
    assert!(published().len() >= 150, "{}", published().len());
    assert!(records().len() >= 50, "{}", records().len());
    // The round trip, stated as an equality rather than left to the two rules: the
    // entity nouns' owners and the records the envelope carries are one set, so a
    // type added on either side alone fails.
    let owners: BTreeSet<String> = VOCABULARY
        .iter()
        .filter(|entry| entry.shape == Shape::Entity)
        .map(|entry| entry.owner.to_owned())
        .collect();
    assert_eq!(owners, records(), "the entity nouns and the records differ");
    assert!(owners.len() >= 50, "{}", owners.len());
    // Each excused name is the identity of exactly one noun and owns none: that is
    // the only thing the exemption admits, so it cannot become a parking place.
    let claimed: BTreeSet<&str> = VOCABULARY
        .iter()
        .filter_map(|entry| entry.identity)
        .collect();
    for (name, why) in IDENTITY_ONLY {
        assert!(!owners.contains(*name), "{name} owns a noun");
        assert!(claimed.contains(*name), "{name} is no noun's identity");
        assert!(!IDENTITY_ONLY.is_empty() && !why.is_empty());
    }
    // And the artifact is the one committed, so the vocabulary is not checked
    // against a document that differs from the tree.
    let committed = std::fs::read_to_string(workspace_root().join(ONTOLOGY_SCHEMA_PATH))
        .expect("the committed ontology");
    assert_eq!(
        committed,
        ontology_text(),
        "the committed ontology drifted from the tree; regenerate it with \
         `cargo run -p symbiote-domain --example ontology_schema -- --write`"
    );
}

/// The vocabulary with one noun removed, so the type it owned is left to the
/// rules that hold a canonical type to a noun.
fn without(noun: &str) -> Vec<Noun> {
    VOCABULARY
        .iter()
        .copied()
        .filter(|entry| entry.noun != noun)
        .collect()
}

/// The committed artifact with one indexed name, one record or one definition
/// changed, so a document that disagrees with the inventory can be refused.
fn indexed_with(name: &str, document: &serde_json::Value) -> serde_json::Value {
    let mut changed = document.clone();
    changed["entities"]
        .as_object_mut()
        .expect("an index")
        .insert(
            name.to_owned(),
            serde_json::json!({ "$ref": format!("#/$defs/{name}") }),
        );
    changed
}

fn without_indexed(name: &str) -> serde_json::Value {
    let mut changed = artifact();
    changed["entities"]
        .as_object_mut()
        .expect("an index")
        .remove(name);
    changed
}

fn carrying(name: &str) -> serde_json::Value {
    let mut changed = artifact();
    changed["envelope_records"]
        .as_array_mut()
        .expect("a record list")
        .push(serde_json::json!(name));
    changed
}

fn defining(name: &str) -> serde_json::Value {
    let mut changed = artifact();
    changed["$defs"]
        .as_object_mut()
        .expect("definitions")
        .insert(name.to_owned(), serde_json::json!({ "type": "string" }));
    changed
}

/// Every way the inventory or the artifact can be untrue, each refused by name.
/// These are the rules the real vocabulary passes, exercised on inputs that fail
/// them — including the document-level ones, so a record no noun owns is refused
/// whether it arrives in the vocabulary or in the published artifact.
#[test]
fn a_vocabulary_or_an_artifact_that_breaks_any_rule_is_refused_by_name() {
    type Run = Box<dyn Fn() -> Vec<String>>;
    let held = |vocabulary: Vec<Noun>, document: serde_json::Value| -> Run {
        Box::new(move || problems(&vocabulary, OUTSTANDING, IDENTITY_NAMES, &document))
    };
    let plain = |vocabulary: Vec<Noun>, outstanding: Vec<Outstanding>| -> Run {
        Box::new(move || problems(&vocabulary, &outstanding, IDENTITY_NAMES, &artifact()))
    };
    let cases: Vec<(&str, Run, &str)> = vec![
        (
            "two nouns, one owner",
            plain(
                vec![
                    entity("user", "UserId", "User"),
                    entity("actor", "ActorId", "User"),
                ],
                Vec::new(),
            ),
            "type User owns 2 nouns",
        ),
        (
            "one noun, two owners",
            plain(
                vec![
                    entity("user", "UserId", "User"),
                    entity("user", "UserId2", "Fabric"),
                ],
                Vec::new(),
            ),
            "noun user maps to 2 owners",
        ),
        (
            "one identity, two nouns",
            plain(
                vec![
                    entity("user", "UserId", "User"),
                    entity("actor", "UserId", "Fabric"),
                ],
                Vec::new(),
            ),
            "identity UserId identifies 2 nouns",
        ),
        (
            "an identity this crate does not declare",
            plain(vec![entity("user", "InventedId", "User")], Vec::new()),
            "identity InventedId is not declared by this crate",
        ),
        (
            "a declared identity with no noun",
            plain(vec![entity("user", "UserId", "User")], Vec::new()),
            "identity HostId has no owning noun",
        ),
        (
            "an owner no schema declares",
            plain(vec![entity("user", "UserId", "NotAType")], Vec::new()),
            "owner NotAType is not in the published schema",
        ),
        (
            "a lifecycle no schema declares",
            plain(
                vec![Noun {
                    lifecycle: Some("NotAState"),
                    ..entity("user", "UserId", "User")
                }],
                Vec::new(),
            ),
            "lifecycle NotAState is not in the published schema",
        ),
        (
            "an entity the envelope cannot carry",
            plain(
                vec![entity("user", "UserId", "VersionedTaskContract")],
                Vec::new(),
            ),
            "entity VersionedTaskContract is not a record the envelope can carry",
        ),
        (
            "an entity with no identity",
            plain(
                vec![Noun {
                    identity: None,
                    ..entity("user", "UserId", "User")
                }],
                Vec::new(),
            ),
            "an entity must have an identity of its own",
        ),
        (
            "an outstanding criterion with no owner",
            plain(
                vec![entity("user", "UserId", "User")],
                vec![Outstanding {
                    criterion: "something",
                    issue: 0,
                    why: "because",
                }],
            ),
            "something: names no owning issue",
        ),
        (
            "an outstanding criterion with no reason",
            plain(
                vec![entity("user", "UserId", "User")],
                vec![Outstanding {
                    criterion: "something",
                    issue: 36,
                    why: "  ",
                }],
            ),
            "something: states no reason",
        ),
        (
            "a canonical type the artifact does not index",
            held(VOCABULARY.to_vec(), without_indexed("User")),
            "canonical type User is not indexed in the published artifact",
        ),
        (
            "an indexed name this crate does not publish",
            held(VOCABULARY.to_vec(), indexed_with("Nope", &artifact())),
            "the published artifact indexes Nope, which is not a canonical type this crate publishes",
        ),
        (
            "a canonical type owned by no noun",
            held(without("session"), artifact()),
            "canonical type Session is owned by no noun and is not listed as identity-only",
        ),
        (
            "an identity-only entry whose name is gone",
            held(VOCABULARY.to_vec(), without_indexed("HostId")),
            "the identity-only entry HostId is not a canonical type",
        ),
        (
            "an identity-only entry that owns a noun",
            held(vec![entity("host", "HostId", "HostId")], artifact()),
            "the identity-only entry HostId owns the noun host, so it is not identity-only",
        ),
        (
            "an identity-only entry that is no noun's identity",
            held(without("host"), artifact()),
            "the identity-only entry HostId is the identity of no noun",
        ),
        (
            "a record whose type is not a canonical type",
            held(VOCABULARY.to_vec(), carrying("Nope")),
            "the envelope carries Nope, which is not a canonical type this crate publishes",
        ),
        (
            "a record owned by no noun",
            held(VOCABULARY.to_vec(), carrying("HostId")),
            "the record HostId is owned by no noun",
        ),
        (
            "a record whose noun is a value object",
            held(VOCABULARY.to_vec(), carrying("VersionedTaskContract")),
            "the record VersionedTaskContract is not an entity: its noun task contract is a value object",
        ),
        (
            "a published name nothing indexes or references",
            held(VOCABULARY.to_vec(), defining("Nope")),
            "published name Nope is neither a canonical type nor referenced by one",
        ),
    ];
    for (label, run, expected) in cases {
        let found = run();
        assert!(
            found.iter().any(|line| line.contains(expected)),
            "{label}: expected {expected:?} in {found:#?}"
        );
    }
    // The unscoped checks are not vacuous either: the real inventory and the
    // committed artifact both pass.
    assert!(violations(VOCABULARY, OUTSTANDING).is_empty());
}

/// #36's additional acceptance for schemas, and the required scope item that the
/// runtime owner, provider, authentication and billing identities stay separate.
#[test]
fn the_inventory_separates_every_identity_that_must_not_merge() {
    let distinct = [
        "RuntimeProfile",
        "AgentRuntimeAdapterId",
        "InferenceProviderAdapterId",
        "ProviderConnection",
        "CredentialReference",
        "BillingEntitlement",
        "GoalRun",
        "NativeExecution",
        "ExecutionEpisode",
        "LearnedMethod",
        "Experiment",
    ];
    // The two adapter identities are identities rather than records: they name a
    // runtime or provider adapter, which is a value, not a stored entity.
    let identities_only = ["AgentRuntimeAdapterId", "InferenceProviderAdapterId"];
    let mut definitions: BTreeMap<&str, String> = BTreeMap::new();
    for name in distinct {
        assert!(published().contains(name), "{name} is not published");
        if !identities_only.contains(&name) {
            assert!(records().contains(name), "{name} is not a wire record");
        }
        definitions.insert(name, definition(name).to_string());
    }
    let unique: BTreeSet<&String> = definitions.values().collect();
    assert_eq!(
        unique.len(),
        definitions.len(),
        "two of these identities publish the same schema"
    );
}

/// #36: Role, Workforce Binding, Runtime Profile, Workforce Runtime Contract,
/// Dispatch and native Session are structurally distinct and queryable — and a
/// Role carries no runtime identity, which is why re-staffing cannot change it.
#[test]
fn the_staffing_records_are_distinct_and_a_role_holds_no_runtime_identity() {
    let distinct = [
        "Role",
        "WorkforceBinding",
        "WorkforceRuntimeContract",
        "RuntimeProfile",
        "Dispatch",
        "Session",
    ];
    let mut definitions: BTreeMap<&str, String> = BTreeMap::new();
    for name in distinct {
        assert!(
            records().contains(name),
            "{name} is not a record the envelope can carry"
        );
        definitions.insert(name, definition(name).to_string());
    }
    assert_eq!(
        BTreeSet::from_iter(definitions.values()).len(),
        distinct.len()
    );
    let staffing_facts = ["binding_id", "runtime_profile_id", "model_id", "host_id"];
    for fact in staffing_facts {
        assert!(
            !properties("Role").contains(fact),
            "Role carries {fact}: re-staffing would change Role identity"
        );
    }
    // The dispatch, by contrast, pins the resolution it used.
    assert!(properties("Dispatch").contains("contract"));
}

/// #36: a new mutating chat creates its own Change Stream and dirty workspace
/// without duplicating Project state or sharing another stream's worktree.
#[test]
fn a_new_mutating_chat_opens_its_own_stream_and_worktree() {
    let project = id!(ProjectId, "project");
    let chat = |name: &str, stream: &str| Chat {
        id: id!(ChatId, name),
        revision: Revision(1),
        project_id: project.clone(),
        stream_id: Some(id!(ChangeStreamId, stream)),
        origin: ChatOrigin::Human,
        mutating: true,
        state: ChatState::Open,
        disposition: RecordDisposition::Active,
        provenance: provenance(),
    };
    let worktree = |name: &str, stream: &str| Worktree {
        id: id!(WorktreeId, name),
        revision: Revision(1),
        project_id: project.clone(),
        root_id: id!(RootId, "root"),
        stream_id: Some(id!(ChangeStreamId, stream)),
        branch: stream.to_owned(),
        path: format!("/worktrees/{stream}"),
        state: WorktreeState::Allocated,
        disposition: RecordDisposition::Active,
        provenance: provenance(),
    };
    let (first, second) = (chat("chat-a", "stream-a"), chat("chat-b", "stream-b"));
    let (first_tree, second_tree) = (
        worktree("tree-a", "stream-a"),
        worktree("tree-b", "stream-b"),
    );
    assert_ne!(first.stream_id, second.stream_id);
    assert_ne!(first_tree.stream_id, second_tree.stream_id);
    assert_ne!(first_tree.path, second_tree.path);
    // Sharing a stream would mean sharing its dirty workspace, and the worktree a
    // stream owns is a single one, so the two chats cannot be told apart by the
    // worktree alone: the stream link is what makes them independent.
    assert_eq!(first_tree.stream_id, first.stream_id);
    assert_eq!(second_tree.stream_id, second.stream_id);
    // Both chats and the project they belong to serialize together, and the
    // Project record is the same one either way: a stream is not Project state.
    let project_record = Project {
        id: project,
        revision: Revision(1),
        name: "project".into(),
        owner: id!(UserId, "human"),
        roots: BTreeSet::new(),
        lead: id!(RoleId, "lead"),
        disposition: RecordDisposition::Active,
        provenance: provenance(),
    };
    for record in [
        DomainRecord::Project(project_record.clone()),
        DomainRecord::Chat(first.clone()),
        DomainRecord::Chat(second.clone()),
        DomainRecord::Worktree(first_tree.clone()),
        DomainRecord::Worktree(second_tree.clone()),
    ] {
        let envelope = DomainEnvelope {
            schema_version: SCHEMA_VERSION,
            record,
        };
        let wire = serde_json::to_value(&envelope).unwrap();
        let restored = serde_json::from_value::<DomainEnvelope>(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(&restored).unwrap(), wire);
    }
    for absent in ["stream", "stream_id", "worktree", "chat"] {
        assert!(
            !properties("Project").contains(absent),
            "Project carries {absent}: a stream would duplicate Project state"
        );
    }
}

/// #36: Developer Knowledge claims preserve provenance, confidence and freshness
/// independently of rendered Markdown and portal views.
#[test]
fn a_claim_keeps_its_confidence_and_a_projection_cannot_restate_it() {
    assert_eq!(Confidence::new(0).unwrap().basis_points(), 0);
    assert_eq!(Confidence::CERTAIN.basis_points(), 10_000);
    assert_eq!(
        Confidence::new(10_001),
        Err("confidence is 0..=10000 basis points of certainty")
    );
    let claim = KnowledgeClaim {
        id: id!(KnowledgeId, "claim"),
        project_id: id!(ProjectId, "project"),
        revision: Revision(1),
        statement: "the ledger records what ran".into(),
        audience: BTreeSet::from(["engineer".to_owned()]),
        epistemic_status: EpistemicStatus::Confirmed,
        freshness: Freshness::Current,
        confidence: Confidence::new(9_000).unwrap(),
        evidence: BTreeSet::from([id!(EvidenceId, "evidence")]),
        ancestry: BTreeSet::new(),
        source_anchors: vec![ExternalReference {
            system: ExternalSystem::SourceSymbol,
            locator: "crates/symbiote-domain/src/ontology.rs:1".into(),
        }],
        projections: BTreeSet::from([id!(ProjectionId, "docs")]),
        provenance: provenance(),
    };
    let envelope = DomainEnvelope {
        schema_version: SCHEMA_VERSION,
        record: DomainRecord::Knowledge(claim.clone()),
    };
    let wire = serde_json::to_value(&envelope).unwrap();
    assert_eq!(wire["record"]["entity"], "knowledge");
    assert_eq!(wire["record"]["record"]["confidence"], 9_000);
    let restored = serde_json::from_value::<DomainEnvelope>(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), wire);
    // A rendered view names the claim it renders and cannot carry the claim's own
    // facts, so Markdown and portal views stay projections rather than authority.
    let rendered = properties("Projection");
    for claim_only in [
        "statement",
        "confidence",
        "freshness",
        "epistemic_status",
        "evidence",
        "ancestry",
        "source_anchors",
    ] {
        assert!(
            !rendered.contains(claim_only),
            "Projection carries {claim_only}, which is the claim's own fact"
        );
    }
    assert!(rendered.contains("knowledge_id"));
}

/// #36: an external identifier can change without changing Symbiote identity.
#[test]
fn an_external_locator_changes_identity_nothing() {
    let identity = id!(TaskId, "task");
    let anchor = |locator: &str| ExternalReference {
        system: ExternalSystem::Github,
        locator: locator.to_owned(),
    };
    assert_ne!(anchor("issue/36"), anchor("issue/140"));
    assert_eq!(identity, id!(TaskId, "task"));
    // An external reference carries no internal identity of its own: it is a
    // system plus a locator, and nothing in it can stand in for a Task.
    assert_eq!(
        required("ExternalReference"),
        BTreeSet::from(["locator".to_owned(), "system".to_owned()])
    );
}

/// The records that carry no shared `Provenance`. Most carry the lineage they
/// have in record-specific fields — a history, a recorded-at and verified-by, a
/// compilation timestamp — and one, `CredentialReference`, holds an identity and
/// a vault locator and has no lineage to record. This list is the whole of them
/// and is held to be; unifying them with one value object is owned by the
/// persistent store, which decides the record shape that survives a restart. A
/// record that gains shared lineage must leave this list, and a new record that
/// has none cannot join it silently.
const LINEAGE_IN_RECORD_FIELDS: &[&str] = &[
    "BillingEntitlement",
    "ChangeStream",
    "CredentialReference",
    "Dispatch",
    "ExecutionEpisode",
    "Experiment",
    "GoalRun",
    "Host",
    "LearnedMethod",
    "NativeExecution",
    "ProviderConnection",
    "Role",
    "Root",
    "RuntimeProfile",
    "Session",
    "Task",
    "VerificationEvidence",
    "WorkforceBinding",
    "WorkforceRuntimeContract",
];

/// The issue that owns unifying lineage onto one value object (#43).
const LINEAGE_OWNER: u64 = 43;

/// The records whose identity is the record that owns them rather than a field of
/// their own, each with the reason. There is one: the compiled runtime contract is
/// immutable-per-Dispatch control state, so its identity *is* `Dispatch::contract_id`
/// and it cannot be re-identified apart from the dispatch that compiled it.
const IDENTIFIED_BY_OWNER: &[(&str, &str)] = &[(
    "WorkforceRuntimeContract",
    "its identity is Dispatch::contract_id: it is immutable-per-Dispatch compiled control state, so it cannot be re-identified apart from the dispatch that compiled it",
)];

/// #36: every entity carries identity, lineage and a disposition, and the
/// lifecycle state where the noun has one. Checked across the published
/// definitions rather than restated per record.
#[test]
fn every_published_entity_carries_identity_lineage_and_disposition() {
    let (mut checked, mut provenanced) = (0, 0);
    for noun in VOCABULARY.iter().filter(|noun| noun.shape == Shape::Entity) {
        let owner = noun.owner;
        let props = properties(owner);
        let required = required(owner);
        if required.contains("id") {
            assert!(
                !IDENTIFIED_BY_OWNER.iter().any(|(name, _)| *name == owner),
                "{owner} has an identity of its own and is still listed as borrowed"
            );
        } else {
            let stated = IDENTIFIED_BY_OWNER
                .iter()
                .find(|(name, _)| *name == owner)
                .map(|(_, why)| *why);
            assert!(stated.is_some(), "{owner} requires no identity of its own");
            assert!(
                stated.unwrap_or_default().len() >= 40,
                "{owner} states no reason"
            );
        }
        assert_eq!(
            definition(owner)["additionalProperties"],
            serde_json::json!(false),
            "{owner} accepts unknown fields"
        );
        // A disposition is only archival semantics if it is required: an
        // optional one lets a record be stored with no archival state at all.
        if props.contains("disposition") {
            assert!(
                required.contains("disposition"),
                "{owner} makes its disposition optional"
            );
        }
        if props.contains("provenance") {
            assert!(
                required.contains("provenance"),
                "{owner} makes its lineage optional"
            );
            assert!(
                !LINEAGE_IN_RECORD_FIELDS.contains(&owner),
                "{owner} carries shared lineage and is still listed as not carrying it"
            );
            provenanced += 1;
        } else {
            assert!(
                LINEAGE_IN_RECORD_FIELDS.contains(&owner),
                "{owner} carries no lineage and is not listed as carrying it in its own fields"
            );
        }
        if let Some(lifecycle) = noun.lifecycle {
            assert!(
                required.contains("state"),
                "{owner} has lifecycle {lifecycle} but requires no state"
            );
        }
        checked += 1;
    }
    assert!(checked >= 45, "{checked} entities checked");
    assert_eq!(
        provenanced,
        checked - LINEAGE_IN_RECORD_FIELDS.len(),
        "the exemption list and the records that use it disagree"
    );
    // The exemption is a minority with an owner, not the norm: if it ever covers
    // the majority of the inventory the rule has stopped meaning anything.
    assert!(
        LINEAGE_IN_RECORD_FIELDS.len() * 2 < checked,
        "{} of {checked} entities carry lineage in their own fields; unify them with #{} instead of widening this list",
        LINEAGE_IN_RECORD_FIELDS.len(),
        LINEAGE_OWNER
    );
}

/// The criteria of #36 that this layer cannot check say so, and each names the
/// issue that owns it, so nothing here is claimed as delivered.
#[test]
fn the_criteria_this_layer_cannot_check_name_their_owner() {
    assert!(OUTSTANDING.len() >= 5, "{}", OUTSTANDING.len());
    for outstanding in OUTSTANDING {
        assert!(outstanding.issue > 0, "{}", outstanding.criterion);
        assert!(outstanding.why.len() >= 40, "{}", outstanding.criterion);
    }
    let routed: BTreeSet<&str> = OUTSTANDING.iter().map(|entry| entry.criterion).collect();
    // Each of these criteria is either checked above or routed here; none is
    // silently absent from both.
    for criterion in [
        "Schema evolution includes forward/backward migration rules.",
        "Desktop, Host, mobile, plugin, adapter and documentation contract tests serialize the same entities.",
        "One local machine and a heterogeneous multi-Host Fabric use the same semantics.",
        "The same client request has external-only, native-only and hybrid conformance paths without changing canonical authority.",
        "No metric/prompt/native completion/learning candidate can weaken a required security, documentation, impact, review or delivery gate.",
    ] {
        assert!(routed.contains(criterion), "{criterion}");
    }
}
