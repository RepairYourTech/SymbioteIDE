//! The portable manifest's field tables in `configuration.md`, held against the declarations.
//!
//! The document enumerated the manifest's intent in prose and nothing read it: measured on
//! `06988daf`, one edit at a time in a throwaway clone, a field added to `ProjectManifest`, a field
//! dropped from `Policies`, a field added to `Resources`, a preference renamed in `Settings` and two
//! field renames were **all silent** — the crate's own cases drove behaviour, never the declared
//! surface, so a field the document names could leave the declaration and the sentence stayed true.
//!
//! This holds both tables at the declaration that owns each field. Every row's field set is read
//! from the JSON schema `schemars` generates for that declaration, so a field added, dropped or
//! renamed in Rust moves the set the row is compared against. The row set is closed over what its
//! rows reference: a declaration reached from these fields is either a row of the first table, a
//! declaration of the second with the document that owns its fields — whose own references are that
//! document's business, so the walk stops there — or a declaration whose members are not fields, an
//! enum or a map of arbitrary payloads. A struct reached from these fields therefore cannot arrive
//! unheld, and a second-table row the manifest no longer reaches fails rather than sitting unused.
//!
//! What this does not hold: the prose beside the tables (what a field means, and the references that
//! deliberately resolve nothing), the owner cell's wording beyond the document it names existing,
//! and the field sets of declarations neither table names.

use std::path::Path;

use serde_json::Value;
use symbiote_config::{
    Command, ManagedField, Policies, ProjectManifest, Resources, RoleAssignment, Settings,
};

const CONTRACT: &str = include_str!("../../../docs/contracts/configuration.md");

/// The declarations the first table names, with the fields it names for each.
fn field_rows() -> Vec<(String, Vec<String>)> {
    let body = symbiote_contract_read::region(
        CONTRACT,
        "| Declaration | Fields |",
        "\n| Declaration not carried here |",
    );
    table_rows(body, 2)
        .into_iter()
        .map(|cells| {
            let declaration = backticked(&cells[0]);
            assert_eq!(
                declaration.len(),
                1,
                "a field row names one declaration: {:?}",
                cells[0]
            );
            (declaration[0].clone(), backticked(&cells[1]))
        })
        .collect()
}

/// The declarations the first table does not carry, with the document that owns each field set.
fn ownership_rows() -> Vec<(String, String)> {
    let body = symbiote_contract_read::region(
        CONTRACT,
        "| Declaration not carried here | Owning document |",
        "\n## Scope and storage",
    );
    table_rows(body, 2)
        .into_iter()
        .map(|cells| {
            let declaration = backticked(&cells[0]);
            assert_eq!(
                declaration.len(),
                1,
                "an ownership row names one declaration: {:?}",
                cells[0]
            );
            (declaration[0].clone(), cells[1].clone())
        })
        .collect()
}

/// The rows of a markdown table body, each with the number of cells its table states.
fn table_rows(body: &str, cells: usize) -> Vec<Vec<String>> {
    body.lines()
        .filter(|line| line.starts_with("| ") && !line.contains("---"))
        .map(|line| {
            let row: Vec<String> = line
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect();
            assert_eq!(
                row.len(),
                cells,
                "a table row states {cells} cells: {line:?}"
            );
            row
        })
        .collect()
}

/// The identifiers a cell writes in backticks, in the order it writes them.
fn backticked(cell: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = cell;
    while let Some(start) = rest.find('`') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('`') else { break };
        names.push(after[..end].to_string());
        rest = &after[end + 1..];
    }
    names
}

/// The schema `schemars` generates for a declaration the first table names. A row the case cannot
/// derive fails here by name rather than being skipped.
fn schema_of(declaration: &str) -> Value {
    match declaration {
        "ProjectManifest" => serde_json::to_value(schemars::schema_for!(ProjectManifest)).unwrap(),
        "Command" => serde_json::to_value(schemars::schema_for!(Command)).unwrap(),
        "RoleAssignment" => serde_json::to_value(schemars::schema_for!(RoleAssignment)).unwrap(),
        "Resources" => serde_json::to_value(schemars::schema_for!(Resources)).unwrap(),
        "Policies" => serde_json::to_value(schemars::schema_for!(Policies)).unwrap(),
        "Settings" => serde_json::to_value(schemars::schema_for!(Settings)).unwrap(),
        "ManagedField" => serde_json::to_value(schemars::schema_for!(ManagedField)).unwrap(),
        other => panic!("the table names {other}, and this case derives no schema for it"),
    }
}

/// The fields a schema declares, read from the declaration rather than a list kept beside it.
fn declared_fields(schema: &Value) -> Vec<String> {
    let fields = schema
        .get("properties")
        .and_then(Value::as_object)
        .unwrap_or_else(|| panic!("a declaration schema states its fields: {schema}"));
    let mut names: Vec<String> = fields.keys().cloned().collect();
    names.sort();
    names
}

/// Every `#/$defs/...` a value references, wherever the reference sits.
fn referenced(value: &Value) -> Vec<String> {
    fn walk(value: &Value, found: &mut Vec<String>) {
        match value {
            Value::Object(fields) => {
                for (key, child) in fields {
                    if key == "$ref" {
                        if let Some(name) = child
                            .as_str()
                            .and_then(|reference| reference.strip_prefix("#/$defs/"))
                        {
                            found.push(name.to_string());
                        }
                    }
                    walk(child, found);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| walk(item, found)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    walk(value, &mut found);
    found.sort();
    found.dedup();
    found
}

/// The markdown link target a cell writes, or nothing when it writes no link.
fn linked_document(cell: &str) -> Option<String> {
    let start = cell.find("](")? + 2;
    let rest = &cell[start..];
    Some(rest[..rest.find(')')?].to_string())
}

#[test]
fn the_contract_names_every_field_of_the_declarations_it_names() {
    let rows = field_rows();
    let ownership = ownership_rows();
    assert!(
        !rows.is_empty(),
        "the document states the field table: {CONTRACT}"
    );
    let named: Vec<&str> = rows
        .iter()
        .map(|(declaration, _)| declaration.as_str())
        .collect();

    for (declaration, fields) in &rows {
        let schema = schema_of(declaration);
        let declared = declared_fields(&schema);
        let mut documented = fields.clone();
        documented.sort();
        assert_eq!(
            documented, declared,
            "the document names {declaration}'s fields as {documented:?}, and the declaration \
             generates {declared:?}"
        );
    }

    // Every declaration a row's own fields reach is a row, an owned-elsewhere row, or a declaration
    // whose members are not fields: an enum, or a map of arbitrary payloads.
    let mut reached_elsewhere: Vec<String> = Vec::new();
    for (declaration, _) in &rows {
        let schema = schema_of(declaration);
        let definitions = schema.get("$defs").and_then(Value::as_object);
        let properties = schema.get("properties").cloned().unwrap_or(Value::Null);
        for reference in referenced(&properties) {
            if named.contains(&reference.as_str()) {
                continue;
            }
            if ownership.iter().any(|(name, _)| *name == reference) {
                reached_elsewhere.push(reference);
                continue;
            }
            let definition = definitions.and_then(|defined| defined.get(&reference));
            let Some(definition) = definition else {
                panic!(
                    "{declaration} references {reference}, which its schema does not define and \
                     neither table covers: {named:?}"
                );
            };
            assert!(
                definition.get("properties").is_none(),
                "{declaration} declares {reference}, which neither table covers; add its row or \
                 name the document that owns it: {named:?}"
            );
        }
    }

    // Each owned-elsewhere row is reached by a row, and the document it names is a document.
    for (declaration, owner) in &ownership {
        assert!(
            reached_elsewhere.contains(declaration),
            "the field table leaves {declaration} to another document, and no row reaches it"
        );
        let path = linked_document(owner).unwrap_or_else(|| {
            panic!("the owner of {declaration} is written as a document link: {owner:?}")
        });
        let document = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/contracts")
            .join(&path);
        assert!(
            document.is_file(),
            "the owner of {declaration} is {path}, which is not a document under docs/contracts"
        );
    }
}
