//! Export and restore of the adapter's own state.
//!
//! One document, keyed by stable identity: the tables the schema declares are read from
//! `sqlite_schema` rather than listed here, so a migration that adds one is exported and restored
//! without a second owner, and a row's identity columns and foreign keys travel verbatim. Restore
//! inserts the document's rows inside one immediate transaction with foreign keys deferred to its
//! commit, then re-derives supported state from the restored journal and compares it with the
//! restored records; a document this schema, this application, or its own relationships refuse
//! leaves the target exactly as it was.
//!
//! What this is not: a backup of the filesystem. It reads through a connection, so a caller that
//! needs a point-in-time copy of a running database takes the document while no writer holds the
//! transaction, exactly as it would for any read. Encryption at rest, retention, compaction and
//! managed backup remain outside this adapter, which stores no secret values and therefore cannot
//! export any.

use super::*;
use rusqlite::types::{Value as SqlValue, ValueRef};
use serde_json::{Value, json};

/// The envelope's own version, separate from the schema version so a change to the document's shape
/// is refused by name rather than read as a schema mismatch.
const EXPORT_FORMAT: u64 = 1;

/// Every table the adapter's schema declares, in a stable order. `sqlite_%` tables are SQLite's own
/// bookkeeping and are rebuilt by the engine, not exported.
fn tables(connection: &Connection) -> Result<Vec<String>> {
    let mut query = connection.prepare(
        "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )?;
    let rows = query.query_map([], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn column_names(connection: &Connection, table: &str) -> Result<Vec<String>> {
    let query = connection.prepare(&format!("SELECT * FROM \"{table}\" LIMIT 0"))?;
    Ok(query
        .column_names()
        .iter()
        .map(|name| name.to_string())
        .collect())
}

/// A stored column as JSON: the scalar types this schema uses, and a refusal for the two shapes it
/// cannot represent. Nothing here converts a value to a different type, so identity survives.
fn json_value(value: ValueRef<'_>) -> Result<Value> {
    Ok(match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(integer) => Value::from(integer),
        ValueRef::Real(real) => Value::from(real),
        ValueRef::Text(text) => Value::from(
            std::str::from_utf8(text)
                .map_err(|_| StoreError::Integrity("stored text is not UTF-8".into()))?,
        ),
        ValueRef::Blob(_) => {
            return Err(StoreError::Integrity(
                "this schema stores no blobs, and an export would not carry one".into(),
            ));
        }
    })
}

fn sql_value(value: &Value) -> Result<SqlValue> {
    Ok(match value {
        Value::Null => SqlValue::Null,
        Value::Number(number) if number.is_i64() => SqlValue::Integer(number.as_i64().unwrap()),
        Value::Number(number) => SqlValue::Real(
            number
                .as_f64()
                .ok_or_else(|| StoreError::Integrity("a number an export cannot carry".into()))?,
        ),
        Value::String(text) => SqlValue::Text(text.clone()),
        Value::Bool(_) | Value::Array(_) | Value::Object(_) => {
            return Err(StoreError::Integrity(
                "stored columns are scalars; an export names no array, object or boolean".into(),
            ));
        }
    })
}

pub(super) fn export(connection: &Connection) -> Result<String> {
    let mut exported = serde_json::Map::new();
    for table in tables(connection)? {
        let columns = column_names(connection, &table)?;
        let mut query = connection.prepare(&format!("SELECT * FROM \"{table}\" ORDER BY rowid"))?;
        let mut rows = query.query([])?;
        let mut records = Vec::new();
        while let Some(row) = rows.next()? {
            let mut record = serde_json::Map::new();
            for (index, column) in columns.iter().enumerate() {
                record.insert(column.clone(), json_value(row.get_ref(index)?)?);
            }
            records.push(Value::Object(record));
        }
        exported.insert(table, Value::Array(records));
    }
    Ok(serde_json::to_string(&json!({
        "export_format": EXPORT_FORMAT,
        "schema_version": DATABASE_VERSION,
        "application_id": APPLICATION_ID,
        "tables": Value::Object(exported),
    }))?)
}

pub(super) fn restore(connection: &mut Connection, document: &str) -> Result<()> {
    let parsed: Value = serde_json::from_str(document)?;
    if parsed.get("export_format").and_then(Value::as_u64) != Some(EXPORT_FORMAT) {
        return Err(StoreError::Integrity(
            "document is not an export of this format".into(),
        ));
    }
    if parsed.get("schema_version").and_then(Value::as_u64) != Some(DATABASE_VERSION as u64) {
        return Err(StoreError::UnsupportedVersion);
    }
    if parsed.get("application_id").and_then(Value::as_i64) != Some(APPLICATION_ID) {
        return Err(StoreError::Integrity(
            "document belongs to another application".into(),
        ));
    }
    let documented = parsed
        .get("tables")
        .and_then(Value::as_object)
        .ok_or_else(|| StoreError::Integrity("document carries no tables".into()))?;

    // Restore replaces state; it never merges into it. A store that holds anything at all is
    // refused before the transaction opens, so a caller cannot silently lose what it holds.
    let declared = tables(connection)?;
    for table in &declared {
        let held: i64 =
            connection.query_row(&format!("SELECT count(*) FROM \"{table}\""), [], |row| {
                row.get(0)
            })?;
        if held != 0 {
            return Err(StoreError::InvalidInitialState);
        }
    }
    let documented_tables: BTreeSet<&String> = documented.keys().collect();
    let declared_tables: BTreeSet<&String> = declared.iter().collect();
    if documented_tables != declared_tables {
        return Err(StoreError::Integrity(
            "document's tables are not this schema's".into(),
        ));
    }

    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    // Rows carry their own foreign keys, so the document's table order must not matter: the
    // constraints are checked once, at the commit that makes the set whole.
    transaction.pragma_update(None, "defer_foreign_keys", true)?;
    for (table, records) in documented.iter() {
        let columns = column_names(&transaction, table)?;
        let records = records
            .as_array()
            .ok_or_else(|| StoreError::Integrity("a table's records must be a list".into()))?;
        let insert = format!(
            "INSERT INTO \"{table}\" ({}) VALUES ({})",
            columns
                .iter()
                .map(|column| format!("\"{column}\""))
                .collect::<Vec<_>>()
                .join(","),
            vec!["?"; columns.len()].join(",")
        );
        for record in records {
            let record = record
                .as_object()
                .ok_or_else(|| StoreError::Integrity("a record must be an object".into()))?;
            let mut values = Vec::with_capacity(columns.len());
            for column in &columns {
                let value = record.get(column).ok_or_else(|| {
                    StoreError::Integrity("a record omits one of its columns".into())
                })?;
                values.push(sql_value(value)?);
            }
            transaction.execute(&insert, rusqlite::params_from_iter(values.iter()))?;
        }
    }
    audit_connection(&transaction)?;
    transaction.commit()?;
    Ok(())
}
