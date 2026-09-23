//! The scope table `configuration.md` states, held against the declarations that apply it.
//!
//! The document's table names every scope with its specificity, storage classification and
//! selection. Nothing read it: the crate's own cases drove precedence and classification for a few
//! scopes, so moving another scope's class, reordering the enum, or dropping a table row left every
//! case green. This holds the table at the crate that owns the scopes: the scope set comes from
//! `enum Scope`'s own JSON schema rather than a list kept beside the enum, the specificity from the
//! order the resolver applies (`Ord`), and the classification from `scope_storage`, so each of
//! those moving fails here by name.
//!
//! What this does not hold: the human labels after each identifier, the paragraphs below the table,
//! and the selection column. This case reads the table; the crate's other cases drive what a
//! selection means.

use serde_json::{Value, json};
use symbiote_config::{
    Context, Layer, MachineBindings, Scope, SecretValue, Settings, StorageClass, resolve,
    scope_storage,
};

const CONTRACT: &str = include_str!("../../../docs/contracts/configuration.md");

/// The scope rows of the document's table: scope, specificity, classification and selection cells.
fn table_rows() -> Vec<[String; 4]> {
    let body = symbiote_contract_read::region(
        CONTRACT,
        "| Scope | Specificity | Storage classification | Selection |",
        "\nUnspecified preferences inherit.",
    );
    body.lines()
        .filter(|line| line.starts_with("| ") && !line.contains("---"))
        .map(|line| {
            let cells: Vec<String> = line
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect();
            assert_eq!(cells.len(), 4, "a scope row states four cells: {line:?}");
            [
                cells[0].clone(),
                cells[1].clone(),
                cells[2].clone(),
                cells[3].clone(),
            ]
        })
        .collect()
}

/// The canonical identifier a scope cell writes in backticks beside its human label.
fn named_scope(cell: &str) -> Option<&str> {
    let start = cell.find('`')? + 1;
    let rest = &cell[start..];
    Some(&rest[..rest.find('`')?])
}

/// The table's own vocabulary for each storage class.
fn phrase(class: StorageClass) -> &'static str {
    match class {
        StorageClass::VersionControlled => "Version controlled",
        StorageClass::UserSynchronized => "User synchronized",
        StorageClass::LocalOnly => "Local only",
        StorageClass::ProhibitedFromDisk => "Prohibited from config disk serialization",
    }
}

/// Every name a schema states, wherever it states one: an `enum` array's members and a `const`
/// alternative alike. A schema writer may hoist a documented variant out of the `enum` into its own
/// `const` arm, so reading one key would make a doc comment on a variant look like a contract move.
fn schema_names(value: &Value, names: &mut Vec<String>) {
    match value {
        Value::Object(fields) => {
            for (key, child) in fields {
                if key == "enum" {
                    names.extend(
                        child
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_str)
                            .map(str::to_string),
                    );
                } else if key == "const" {
                    if let Some(name) = child.as_str() {
                        names.push(name.to_string());
                    }
                }
                schema_names(child, names);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| schema_names(item, names)),
        _ => {}
    }
}

/// The scopes `enum Scope` declares, read from the declaration's generated schema rather than a
/// list written beside the enum — a variant added or removed moves this set.
fn declared_scopes() -> Vec<(String, Scope)> {
    let schema = serde_json::to_value(schemars::schema_for!(Scope)).unwrap();
    let mut names = Vec::new();
    schema_names(&schema, &mut names);
    assert!(
        !names.is_empty(),
        "the Scope schema names its variants: {schema}"
    );
    names
        .iter()
        .map(|name| {
            let scope = serde_json::from_value(json!(name)).unwrap();
            (name.clone(), scope)
        })
        .collect()
}

/// A layer at `scope` that applies to `target`, carrying the selector the scope requires.
fn layer(name: &str, scope: Scope, target: &Context) -> Layer {
    let global = |root, repository, role| Context {
        project: target.project.clone(),
        root,
        repository,
        role,
        controller: target.controller.clone(),
    };
    let context = match scope {
        Scope::Application | Scope::User => None,
        Scope::Project => Some(global(None, None, None)),
        Scope::WorkspaceRoot => Some(global(target.root.clone(), None, None)),
        Scope::Repository => Some(global(None, target.repository.clone(), None)),
        Scope::Role => Some(global(None, None, target.role.clone())),
        Scope::LocalOverride => Some(target.clone()),
    };
    Layer {
        schema_version: 1,
        scope,
        source: name.into(),
        revision: "1".into(),
        context,
        settings: Settings {
            telemetry: Some(true),
            ..Settings::default()
        },
    }
}

#[test]
fn the_contract_states_the_specificity_and_storage_of_every_scope() {
    // The declared scopes in the order the resolver applies them: the table's numbers are that
    // order, so reordering the enum or moving a number reds here.
    let mut ordered = declared_scopes();
    ordered.sort_by_key(|(_, scope)| *scope);

    let rows = table_rows();
    let numbered: Vec<&[String; 4]> = rows
        .iter()
        .filter(|row| row[1] != "Outside inheritance")
        .collect();
    let named: Vec<&str> = numbered
        .iter()
        .map(|row| {
            named_scope(&row[0])
                .unwrap_or_else(|| panic!("a scope row names its schema identifier: {row:?}"))
        })
        .collect();
    assert_eq!(
        named,
        ordered
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        "the table's scopes, in the order the resolver applies them, are the ones enum Scope declares"
    );

    for (index, row) in numbered.iter().enumerate() {
        let (name, scope) = &ordered[index];
        assert_eq!(
            row[1],
            (index + 1).to_string(),
            "the table gives {name} specificity {}, and the resolver applies it {}-th",
            row[1],
            index + 1
        );
        assert_eq!(
            row[2],
            phrase(scope_storage(*scope)),
            "the table calls {name}'s storage {}, and this crate classifies it {}",
            row[2],
            phrase(scope_storage(*scope))
        );
    }

    // The rows outside inheritance are the machine bindings and the secrets, with their own
    // classifications and nothing else: a scope that leaves inheritance says where it is stored.
    let mut outside: Vec<&str> = rows
        .iter()
        .filter(|row| row[1] == "Outside inheritance")
        .map(|row| row[2].as_str())
        .collect();
    let mut expected = vec![
        phrase(MachineBindings::STORAGE),
        phrase(SecretValue::STORAGE),
    ];
    outside.sort_unstable();
    expected.sort_unstable();
    assert_eq!(
        outside,
        expected,
        "the rows outside inheritance are the machine bindings and the secrets: {:?}",
        rows.iter()
            .filter(|row| row[1] == "Outside inheritance")
            .collect::<Vec<_>>()
    );

    // The table's precedence is the resolver's, not a sentence beside it: over the layers the
    // table names, each prefix resolves to the scope the table gives the highest specificity.
    let target = Context {
        project: "project".into(),
        root: Some("root".into()),
        repository: Some("repository".into()),
        role: Some("role".into()),
        controller: "controller".into(),
    };
    let layers: Vec<Layer> = ordered
        .iter()
        .map(|(name, scope)| layer(name, *scope, &target))
        .collect();
    for prefix in 1..=layers.len() {
        let resolved = resolve(&layers[..prefix], &target).unwrap();
        assert_eq!(resolved.settings.telemetry, Some(true));
        assert_eq!(
            resolved.provenance["telemetry"].last().unwrap().scope,
            ordered[prefix - 1].1,
            "the {prefix}-th layer in the table's order is the most specific so far"
        );
    }
}
