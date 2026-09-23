use super::*;
use serde_json::{Value, json};

/// A store holding what a Project with its Roots and Roles, a Task and its Change Stream produce:
/// the canonical records, the journal and the work materialization.
fn populated(path: std::path::PathBuf) -> (Store, Project, Vec<Root>, Vec<Role>, Task) {
    let mut store = Store::open(path).unwrap();
    let (project, roots, roles) = register(&mut store, "one");
    let (task, stream) = task_records("one");
    store
        .create_fixture_task(id!(CommandId, "transfer-task"), task.clone(), stream)
        .unwrap();
    (store, project, roots, roles, task)
}

/// Every table this schema declares, read from the database itself rather than from a list here.
fn declared_tables(store: &Store) -> Vec<String> {
    let mut query = store
        .connection
        .prepare(
            "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' \
             ORDER BY name",
        )
        .unwrap();
    let rows = query.query_map([], |row| row.get::<_, String>(0)).unwrap();
    rows.map(std::result::Result::unwrap).collect()
}

fn document(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}

#[test]
fn export_restore_preserves_identity_and_relationships() {
    let temp = Temporary::new();
    let (store, project, roots, roles, task) = populated(temp.database());
    let document = store.export().unwrap();

    // The document names every table this schema declares: a migration that adds one is carried
    // without a second list to move, and one the reading skipped reds here.
    let parsed: Value = serde_json::from_str(&document).unwrap();
    let named: Vec<String> = parsed["tables"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(named, declared_tables(&store));

    // Export is a function of the state: the same store exports the same bytes twice.
    assert_eq!(store.export().unwrap(), document);

    let target = Temporary::new();
    let mut restored = Store::open(target.database()).unwrap();
    restored.restore_into(&document).unwrap();

    // Identity: the same stable identifiers resolve to the same records, and the journal is the
    // journal. A restore that re-keyed anything would fail each of these.
    assert_eq!(
        restored.project(&project.id).unwrap(),
        store.project(&project.id).unwrap()
    );
    assert_eq!(
        restored.task(task.id()).unwrap(),
        store.task(task.id()).unwrap()
    );
    assert_eq!(
        restored.task_stream(task.id()).unwrap(),
        store.task_stream(task.id()).unwrap()
    );
    for root in &roots {
        assert_eq!(
            restored.root(&root.id).unwrap(),
            store.root(&root.id).unwrap()
        );
    }
    assert_eq!(
        restored.events(&project.id, 0, MAX_EVENT_PAGE).unwrap(),
        store.events(&project.id, 0, MAX_EVENT_PAGE).unwrap()
    );
    assert_eq!(
        restored
            .registration_timestamp(&id!(CommandId, "register-one"))
            .unwrap(),
        store
            .registration_timestamp(&id!(CommandId, "register-one"))
            .unwrap()
    );

    // Relationships: the restored store passes the audit a reopened one does, and the document is
    // a fixed point of a second round trip.
    restored.integrity_check().unwrap();
    assert_eq!(restored.export().unwrap(), document);

    // A restored store is a live store: it deduplicates the retry the original would.
    let receipt = restored
        .register_project(
            id!(CommandId, "register-one"),
            project.clone(),
            roots,
            roles,
        )
        .unwrap();
    assert!(receipt.replayed);
}

#[test]
fn restore_refuses_a_store_that_holds_anything() {
    let temp = Temporary::new();
    let (mut store, _, _, _, _) = populated(temp.database());
    let before = store.export().unwrap();

    assert!(matches!(
        store.restore_into(&before),
        Err(StoreError::InvalidInitialState)
    ));
    assert_eq!(store.export().unwrap(), before);
    store.integrity_check().unwrap();
}

#[test]
fn restore_refuses_a_damaged_document_without_leaving_partial_state() {
    let temp = Temporary::new();
    let (store, project, _, _, _) = populated(temp.database());
    let good = store.export().unwrap();
    let empty = Store::open(Temporary::new().database())
        .unwrap()
        .export()
        .unwrap();

    // Every entry names the refusal that answers it and asserts the refusal's own kind, so a check
    // that stops answering reds here even when a constraint, a missing table or the foreign-key
    // check would have refused the same document by another route: the three envelope edits, the
    // emptied-table edit, the invented name and the omitted column must be refused by this adapter,
    // and a dangling relationship by the engine's own foreign-key check.
    let mut tampered: Vec<(String, Value, bool)> = Vec::new();
    for (label, key, value) in [
        ("format", "export_format", json!(2)),
        ("schema", "schema_version", json!(DATABASE_VERSION - 1)),
        ("application", "application_id", json!(0)),
    ] {
        let mut parsed: Value = serde_json::from_str(&good).unwrap();
        parsed[key] = value;
        tampered.push((label.into(), parsed, true));
    }
    let mut dropped: Value = serde_json::from_str(&good).unwrap();
    dropped["tables"]
        .as_object_mut()
        .unwrap()
        .remove("projects");
    tampered.push(("table-dropped".into(), dropped, true));
    // The entry the table-set refusal alone can catch: the table holds no rows, nothing references
    // it, and the audit reconstructs nothing from it, so a restore that read the set loosely would
    // accept this document and walk past every refusal below.
    let mut dropped_empty: Value = serde_json::from_str(&good).unwrap();
    let emptied = dropped_empty["tables"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, rows)| rows.as_array().map(Vec::is_empty).unwrap_or(false))
        .map(|(table, _)| table.clone())
        .expect("a table this fixture leaves empty");
    dropped_empty["tables"]
        .as_object_mut()
        .unwrap()
        .remove(&emptied);
    tampered.push((
        format!("table-dropped-empty {emptied}"),
        dropped_empty,
        true,
    ));
    let mut extra: Value = serde_json::from_str(&good).unwrap();
    extra["tables"]["invented"] = json!([]);
    tampered.push(("table-invented".into(), extra, true));
    let mut column: Value = serde_json::from_str(&good).unwrap();
    column["tables"]["projects"][0]
        .as_object_mut()
        .unwrap()
        .remove("body");
    tampered.push(("column-omitted".into(), column, true));
    let mut orphan: Value = serde_json::from_str(&good).unwrap();
    orphan["tables"]["tasks"][0]["project_id"] = json!("project-elsewhere");
    tampered.push(("relationship-dangling".into(), orphan, false));
    let mut rewritten: Value = serde_json::from_str(&good).unwrap();
    let body = rewritten["tables"]["projects"][0]["body"]
        .as_str()
        .unwrap()
        .replace("Project one", "Project rewritten");
    rewritten["tables"]["projects"][0]["body"] = json!(body);
    tampered.push(("record-differs-from-journal".into(), rewritten, true));

    for (label, value, ours) in tampered {
        let target = Temporary::new();
        let mut target = Store::open(target.database()).unwrap();
        let refusal = target.restore_into(&document(&value));
        assert!(refusal.is_err(), "{label} was accepted");
        if ours {
            assert!(
                matches!(
                    refusal.as_ref(),
                    Err(StoreError::Integrity(_)
                        | StoreError::UnsupportedVersion
                        | StoreError::InvalidInitialState)
                ),
                "{label} was refused by the engine rather than by its own check: {refusal:?}"
            );
        }
        assert_eq!(target.export().unwrap(), empty, "{label} left state behind");
        assert!(
            matches!(target.project(&project.id), Err(StoreError::NotFound)),
            "{label} left a partial record"
        );
        target.integrity_check().unwrap();
    }

    // The same store that refused every damaged document still restores the honest one, and
    // refuses a second restore rather than merging.
    let target = Temporary::new();
    let mut target = Store::open(target.database()).unwrap();
    target.restore_into(&good).unwrap();
    assert!(matches!(
        target.restore_into(&good),
        Err(StoreError::InvalidInitialState)
    ));
}
