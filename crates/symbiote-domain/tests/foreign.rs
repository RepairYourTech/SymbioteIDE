//! #189 foreign runtime session/task references: what a link may record, what
//! it may not, and the bounds the contract document states.
use std::collections::BTreeSet;
use symbiote_contract_read::{bytes, figure, region};
use symbiote_domain::*;

fn link(kind: ForeignItemKind, foreign_id: &str, session: Option<&str>) -> ForeignTaskLink {
    ForeignTaskLink {
        system: ExternalSystem::Harness,
        kind,
        foreign_id: foreign_id.to_owned(),
        foreign_session_id: session.map(str::to_owned),
        foreign_status: ForeignStatus::Complete,
        observed_at: Timestamp(1),
    }
}

fn set(links: Vec<ForeignTaskLink>) -> ForeignTaskLinks {
    ForeignTaskLinks {
        project_id: ProjectId::new("alpha").unwrap(),
        task_id: TaskId::new("owner").unwrap(),
        links: links.into_iter().collect(),
    }
}

#[test]
fn a_link_records_a_foreign_systems_own_identifiers_and_nothing_else() {
    let links = set(vec![
        link(ForeignItemKind::Session, "codex-session-01", None),
        link(
            ForeignItemKind::Task,
            "codex-todo-7",
            Some("codex-session-01"),
        ),
    ]);
    assert_eq!(links.validate(), Ok(()));
    // The whole record round-trips through the wire form, and the wire form
    // adds no field a caller could use to assert canonical state.
    let text = serde_json::to_string(&links).unwrap();
    assert_eq!(
        serde_json::from_str::<ForeignTaskLinks>(&text).unwrap(),
        links
    );
    let fields: BTreeSet<String> = serde_json::from_str::<serde_json::Value>(&text)
        .unwrap()
        .get("links")
        .and_then(|set| set.as_array())
        .unwrap()
        .iter()
        .flat_map(|entry| {
            entry
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<String>>()
        })
        .collect();
    assert_eq!(
        fields,
        BTreeSet::from([
            "foreign_id".to_owned(),
            "foreign_session_id".to_owned(),
            "foreign_status".to_owned(),
            "kind".to_owned(),
            "observed_at".to_owned(),
            "system".to_owned(),
        ]),
        "a link carries identifiers, a foreign status and an observation time — no canonical status"
    );
    // A field a caller invented is refused, not ignored: a `task_state` smuggled
    // onto the wire is not a link with an unknown extra.
    let forged = text.replace(
        "\"kind\":\"session\"",
        "\"kind\":\"session\",\"task_state\":\"completed\"",
    );
    assert!(serde_json::from_str::<ForeignTaskLinks>(&forged).is_err());
}

#[test]
fn every_named_refusal_is_refused_by_name() {
    let refused = |link: ForeignTaskLink, why: &str| {
        assert_eq!(
            link.validate(),
            Err(DomainError::InvalidForeignReference),
            "{why}"
        );
    };
    refused(
        link(ForeignItemKind::Session, "session", Some("another-session")),
        "a session in a session",
    );
    let mut wrong_system = link(ForeignItemKind::Session, "session", None);
    wrong_system.system = ExternalSystem::Github;
    refused(
        wrong_system,
        "a GitHub reference is not a harness's own session",
    );
    refused(
        link(ForeignItemKind::Task, "todo", Some("   ")),
        "a blank session identifier",
    );
    refused(
        link(ForeignItemKind::Session, "ses\u{7}sion", None),
        "a control character in an identifier",
    );
    let mut long = link(
        ForeignItemKind::Session,
        &"s".repeat(MAX_FOREIGN_ID_BYTES + 1),
        None,
    );
    assert_eq!(long.validate(), Err(DomainError::ResourceLimit));
    long.foreign_id = "s".repeat(MAX_FOREIGN_ID_BYTES);
    assert_eq!(long.validate(), Ok(()), "exactly the bound is held");
    let mut long_session = link(ForeignItemKind::Task, "todo", Some("s"));
    long_session.foreign_session_id = Some("s".repeat(MAX_FOREIGN_ID_BYTES + 1));
    assert_eq!(long_session.validate(), Err(DomainError::ResourceLimit));

    // One item named twice with two observations is a set a reader could not
    // resolve, so it is refused rather than silently collapsed.
    let mut later = link(ForeignItemKind::Session, "session", None);
    later.foreign_status = ForeignStatus::Active;
    later.observed_at = Timestamp(2);
    assert_eq!(
        set(vec![link(ForeignItemKind::Session, "session", None), later]).validate(),
        Err(DomainError::ForeignObservationConflict)
    );

    // The set bound, driven at its far edge: exactly the bound is held, and
    // one more is refused as a set rather than truncated to the bound.
    let full: Vec<ForeignTaskLink> = (0..MAX_FOREIGN_LINKS)
        .map(|index| link(ForeignItemKind::Session, &format!("session-{index}"), None))
        .collect();
    assert_eq!(set(full.clone()).validate(), Ok(()));
    let mut over = full;
    over.push(link(ForeignItemKind::Session, "one-too-many", None));
    assert_eq!(set(over).validate(), Err(DomainError::ResourceLimit));
}

#[test]
fn a_foreign_status_cannot_enter_canonical_state() {
    let schema = ontology_schema();
    let task: &serde_json::Value = &schema["$defs"]["Task"];
    let properties: BTreeSet<&str> = task["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert!(
        !properties.iter().any(|name| name.contains("foreign")),
        "the Task record must carry no field a foreign link could arrive in: {properties:?}"
    );
    // The link is a canonical value type, published and vocabulary-owned, and
    // never a top-level record in its own right: it only ever rides inside a
    // link set, so there is no envelope that could carry a foreign reference
    // as though it were a canonical record.
    assert!(schema["$defs"]["ForeignTaskLink"].is_object());
    let records: BTreeSet<&str> = schema["envelope_records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    assert!(!records.contains("ForeignTaskLink"));
    assert!(!records.contains("ForeignTaskLinks"));
    // The foreign vocabulary is its own: a `complete` is a ForeignStatus, not a
    // TaskState, so no value of one can be read as the other.
    let foreign: BTreeSet<&str> = schema["$defs"]["ForeignStatus"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(
        foreign,
        BTreeSet::from(["pending", "active", "complete", "failed", "unknown"])
    );
    let canonical: BTreeSet<&str> = schema["$defs"]["TaskState"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert!(
        !canonical.contains("complete"),
        "the canonical vocabulary has no status a foreign one could be read as"
    );
}

/// What `work-hierarchy.md` states about foreign references, held to the
/// declarations this crate and the store carry: the two bounds by figure, every
/// refusal by name, the operations and version by the sentences that name them,
/// and the store's own table and migration, so a document that drifts from the
/// code reds here rather than in prose nobody reads.
#[test]
fn the_contract_states_the_foreign_reference_bounds_and_refusals_enforced() {
    let hierarchy = include_str!("../../../docs/contracts/work-hierarchy.md");
    let store = include_str!("../../symbiote-store/src/foreign.rs");
    let protocol = include_str!("../../symbiote-protocol/src/lib.rs");
    let section = region(
        hierarchy,
        "## Foreign runtime session and task references",
        "## Task traceability and migration",
    );

    let identifier: usize = bytes(region(section, "an identifier over ", " bytes"));
    let links: usize = figure(region(section, "as is a set over ", " links"));
    assert_eq!(
        (identifier, links),
        (MAX_FOREIGN_ID_BYTES, MAX_FOREIGN_LINKS),
        "the document's bounds are the ones this crate enforces"
    );
    for refusal in [
        "InvalidForeignReference",
        "ResourceLimit",
        "ForeignObservationConflict",
        "NotFound",
    ] {
        assert!(section.contains(refusal), "the section must name {refusal}");
    }
    assert!(
        section.contains("integrity failure"),
        "the section must name the integrity refusal"
    );
    // Prose phrases are read from the section with its line breaks flattened,
    // so a rewrap does not red this case while a removed claim does. The claim
    // a client sends is the canonical record minus the time: the Host stamps
    // that, so the wire cannot say when a foreign claim was seen.
    let flat = section.split_whitespace().collect::<Vec<_>>().join(" ");
    for phrase in [
        "the claim a caller sends carries the identifiers and the foreign status only",
        "no client can say when it saw a foreign claim",
    ] {
        assert!(flat.contains(phrase), "the section must state: {phrase}");
    }
    for operation in ["set_task_foreign_links", "get_task_foreign_links"] {
        assert!(
            section.contains(operation),
            "the section must name {operation}"
        );
        // The wire name is the snake_case variant: what the document says a
        // client sends is the variant the protocol enum declares.
        let variant: String = operation
            .split('_')
            .map(|word| {
                let mut capitalized = String::with_capacity(word.len());
                for (index, character) in word.chars().enumerate() {
                    if index == 0 {
                        capitalized.extend(character.to_uppercase());
                    } else {
                        capitalized.push(character);
                    }
                }
                capitalized
            })
            .collect();
        assert!(
            protocol.contains(&format!("{variant} {{")),
            "the protocol must declare {variant} for {operation}"
        );
    }
    assert!(section.contains("protocol v1.26"));
    assert!(section.contains("Store schema v13"));
    // The store owns the table and the version the document states.
    assert!(store.contains("CREATE TABLE task_foreign_links"));
    assert!(store.contains("PRAGMA user_version=13;"));
    // The reference is an observation, not a dependency or a status: the
    // section says so, and the document no longer lists foreign links as
    // pending work.
    for phrase in [
        "never becomes a dependency edge",
        "`complete` is that harness's word",
        "the Task record carries no link field at all",
        "A foreign system is not a principal",
    ] {
        assert!(flat.contains(phrase), "the section must state: {phrase}");
    }
    assert!(
        !region(
            hierarchy,
            "## Task dependency graph",
            "## Foreign runtime session"
        )
        .contains("Foreign runtime links"),
        "foreign links are no longer listed as pending in the dependency section"
    );
}
