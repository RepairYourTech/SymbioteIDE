//! The `symbiote` CLI's published contract: what operation kinds exist and
//! which are dangerous, and the two JSON Schema documents the binary emits.
//!
//! This module is the ONE owner of that contract. The `symbiote` binary is the
//! interface: it parses arguments, renders help and envelopes, decides
//! authorization and talks to the daemon. Keeping the contract here means the
//! binary, the CLI's own unit tests and the integration suite all read the same
//! classification and the same documents, and a change to a published schema
//! lands in one obvious file.
//!
//! Data flows one way: nothing here imports the binary, and nothing here reads
//! the filesystem except the explicit schema helpers, which take the directory
//! they act on. `published_schemas` renders the object the bare
//! `symbiote schema` prints; `selected_schemas` narrows that set to the one
//! document a selector names; `document_text` produces the canonical bytes
//! that `write_schemas` writes, `schema_drift` compares and the tests pin to
//! the committed fixtures.
use std::path::{Path, PathBuf};

/// The machine-readable envelope's schema identity. Automation keys on this
/// string, not on the presence of individual fields.
pub const CLI_SCHEMA: &str = "symbiote.cli/v1";

/// The authorization policy file's schema identity, versioned independently
/// of the envelope so either can evolve without silently reinterpreting the
/// other.
pub const POLICY_SCHEMA: &str = "symbiote.cli-policy/v1";

/// What a command can do to the Host. The classification decides whether the
/// CLI sends it without an explicit authorization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk {
    /// Reads only: no daemon state changes.
    ReadOnly,
    /// Changes daemon state but starts and stops nothing.
    Mutation,
    /// Starts or ends execution — or can express any operation (`raw`).
    Dangerous,
}

impl Risk {
    pub fn requires_authorization(self) -> bool {
        matches!(self, Risk::Dangerous)
    }
}

/// The ONE table of operation kinds this CLI knows and what sending each can
/// do. Lookups, the dangerous list the help prints, and policy validation all
/// read this array, so they cannot drift apart. A kind absent from the table
/// cannot be proven safe and is treated as dangerous.
pub const OPERATION_RISKS: &[(&str, Risk)] = &[
    // Reads: no daemon state changes.
    ("hello", Risk::ReadOnly),
    ("health", Risk::ReadOnly),
    ("get_host_pulse", Risk::ReadOnly),
    ("get_project", Risk::ReadOnly),
    ("get_task", Risk::ReadOnly),
    ("read_journal", Risk::ReadOnly),
    ("get_dispatch_preparation", Risk::ReadOnly),
    ("get_scheduling_projection", Risk::ReadOnly),
    ("get_team", Risk::ReadOnly),
    ("get_task_origin", Risk::ReadOnly),
    ("get_binding", Risk::ReadOnly),
    ("get_binding_readiness", Risk::ReadOnly),
    ("get_task_dependencies", Risk::ReadOnly),
    ("get_route", Risk::ReadOnly),
    ("resolve_route", Risk::ReadOnly),
    ("get_work", Risk::ReadOnly),
    ("get_provider_connection", Risk::ReadOnly),
    ("get_billing_entitlement", Risk::ReadOnly),
    ("get_model_descriptor", Risk::ReadOnly),
    ("get_resource_consent", Risk::ReadOnly),
    // Mutations: they change state, but they neither start or end execution
    // nor widen authority.
    ("create_task", Risk::Mutation),
    ("prepare_dispatch", Risk::Mutation),
    ("request_task_completion", Risk::Mutation),
    ("request_elevation", Risk::Mutation),
    ("replace_binding", Risk::Mutation),
    ("record_route", Risk::Mutation),
    ("set_task_dependencies", Risk::Mutation),
    ("acquire_task_lease", Risk::Mutation),
    ("release_task_lease", Risk::Mutation),
    ("expire_stale_leases", Risk::Mutation),
    ("replace_provider_connection", Risk::Mutation),
    ("replace_billing_entitlement", Risk::Mutation),
    ("replace_model_descriptor", Risk::Mutation),
    ("replace_team", Risk::Mutation),
    ("create_work", Risk::Mutation),
    ("change_work", Risk::Mutation),
    ("assign_task_origin", Risk::Mutation),
    ("register_project", Risk::Mutation),
    ("observe_root_placement", Risk::Mutation),
    ("revoke_elevation", Risk::Mutation),
    ("revoke_resource_consent", Risk::Mutation),
    // Starting or ending execution, or granting capability.
    ("start_prepared_task", Risk::Dangerous),
    ("run_started_dispatch", Risk::Dangerous),
    ("shutdown", Risk::Dangerous),
    ("decide_elevation", Risk::Dangerous),
    ("record_resource_consent", Risk::Dangerous),
];

/// `None` for a kind this build does not know: the caller must fail closed
/// rather than assume safety.
pub fn known_risk_of_kind(kind: &str) -> Option<Risk> {
    OPERATION_RISKS
        .iter()
        .find(|(known, _)| *known == kind)
        .map(|(_, risk)| *risk)
}

/// An unknown kind cannot be proven safe, so it is Dangerous rather than
/// silently allowed — and, for the same reason, it is not *nameable* by a
/// policy (`known_risk_of_kind` returns `None` for it).
pub fn risk_of_kind(kind: &str) -> Risk {
    known_risk_of_kind(kind).unwrap_or(Risk::Dangerous)
}

/// The dangerous operation kinds, in table order. A kind absent from the table
/// is dangerous too, but is not nameable by a policy, so it is not listed.
pub fn dangerous_kinds() -> Vec<&'static str> {
    OPERATION_RISKS
        .iter()
        .filter(|(_, risk)| risk.requires_authorization())
        .map(|(kind, _)| *kind)
        .collect()
}

/// A published schema document: the short name `symbiote schema <selector>`
/// addresses it by, its identity, the committed fixture file it is published
/// as, and the builder that produces it.
#[derive(Debug)]
pub struct PublishedSchema {
    pub selector: &'static str,
    pub identity: &'static str,
    pub file_name: &'static str,
    build: fn() -> serde_json::Value,
}

/// The published documents. One table, so the object `symbiote schema` prints,
/// the single document `symbiote schema <selector>` prints, the files
/// `symbiote schema --write DIR` writes, and the committed fixtures cannot
/// drift apart.
pub const PUBLISHED_SCHEMAS: &[PublishedSchema] = &[
    PublishedSchema {
        selector: "envelope",
        identity: CLI_SCHEMA,
        file_name: "symbiote.cli.v1.schema.json",
        build: envelope_schema,
    },
    PublishedSchema {
        selector: "policy",
        identity: POLICY_SCHEMA,
        file_name: "symbiote.cli-policy.v1.schema.json",
        build: policy_schema,
    },
];

/// A published document could not be produced or written. Kept as a value so
/// the binary maps it to its own usage error rather than matching on strings.
#[derive(Debug)]
pub enum SchemaError {
    Serialize {
        identity: &'static str,
        source: serde_json::Error,
    },
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    /// `symbiote schema <selector>` named a document that is not published.
    UnknownSelector { selector: String },
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::Serialize { identity, source } => {
                write!(f, "cannot serialize the {identity} schema: {source}")
            }
            SchemaError::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            SchemaError::Write { path, source } => {
                write!(f, "cannot write {}: {source}", path.display())
            }
            SchemaError::UnknownSelector { selector } => write!(
                f,
                "unknown schema `{selector}`; expected {}",
                selectors().join(" or ")
            ),
        }
    }
}

impl std::error::Error for SchemaError {}

/// The published envelope schema, built here so it has one source. The `const`
/// reads the same `CLI_SCHEMA` the envelope constructors write, so the two
/// cannot drift.
fn envelope_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://symbiote.dev/schemas/symbiote.cli.v1.schema.json",
        "title": "symbiote CLI JSON envelope (symbiote.cli/v1)",
        "description": "One envelope printed to stdout by `symbiote --json`, one per invocation. A success carries `result` (the daemon's own response body, unmodified); a failure carries `error` (the daemon's typed error, or one of the CLI's own codes). Exactly one of the two is present, and `ok` agrees with which one it is. The default (non-`--json`) output is the daemon's body alone and is not described here.",
        "type": "object",
        "required": ["schema", "command", "command_id", "ok"],
        "additionalProperties": false,
        "properties": {
            "schema": {
                "description": "Schema identity. Automation keys on this string rather than on the presence of individual fields.",
                "const": CLI_SCHEMA
            },
            "command": {
                "description": "The command name as invoked, e.g. `health`, or `raw`.",
                "type": "string",
                "minLength": 1
            },
            "command_id": {
                "description": "The idempotency key this invocation used: the minted id, or a caller-supplied `--command-id`.",
                "type": "string",
                "minLength": 1
            },
            "ok": {
                "description": "True exactly when the command succeeded (exit 0) and `result` is present.",
                "type": "boolean"
            },
            "result": {
                "description": "The daemon's own response body, unmodified. An internally tagged object whose `kind` names the answer.",
                "$ref": "#/$defs/result"
            },
            "error": {
                "description": "A failure, whether the daemon refused the command or the CLI could not send it.",
                "$ref": "#/$defs/error"
            }
        },
        "oneOf": [
            {
                "description": "Success: a result and no error, with ok true.",
                "required": ["result"],
                "not": { "required": ["error"] },
                "properties": { "ok": { "const": true } }
            },
            {
                "description": "Failure: an error and no result, with ok false.",
                "required": ["error"],
                "not": { "required": ["result"] },
                "properties": { "ok": { "const": false } }
            }
        ],
        "$defs": {
            "result": {
                "description": "symbiote_protocol::ResponseBody, which is internally tagged on `kind`.",
                "type": "object",
                "required": ["kind"],
                "properties": {
                    "kind": {
                        "description": "The response body variant, snake_case, e.g. `health`, `shutdown`.",
                        "type": "string",
                        "minLength": 1
                    }
                }
            },
            "error": {
                "description": "The daemon's own ProtocolError, or the CLI's own code when nothing was sent.",
                "type": "object",
                "required": ["code", "message"],
                "additionalProperties": false,
                "properties": {
                    "code": {
                        "description": "The daemon's error code, or one of the CLI's own: `authorization_required` (nothing was sent), `policy_invalid` (the configured policy could not be honored; nothing was sent), `unreachable`.",
                        "type": "string",
                        "minLength": 1
                    },
                    "message": {
                        "description": "Human-readable detail. Not stable for automation; branch on `code`.",
                        "type": "string"
                    }
                }
            }
        }
    })
}

/// The published policy schema. The `authorize` enum is computed from the
/// operation-risk table rather than transcribed, so promoting a kind to
/// dangerous — or adding one — makes it nameable by a policy and published in
/// the same change.
fn policy_schema() -> serde_json::Value {
    let mut dangerous = dangerous_kinds();
    dangerous.sort_unstable();
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://symbiote.dev/schemas/symbiote.cli-policy.v1.schema.json",
        "title": "symbiote CLI authorization policy (symbiote.cli-policy/v1)",
        "description": "A document named by `--policy FILE`, or by $SYMBIOTE_CLI_POLICY when the flag is absent, that pre-authorizes dangerous operation kinds for noninteractive runs. It may name only the operation kinds this CLI classifies as dangerous: a read or a mutation is already ungated, so listing one would be a no-op an operator could mistake for coverage; an unknown kind cannot be classified; and a wildcard is deliberately not a syntax. A policy is a pre-authorization, never a widening — it cannot grant a permission the daemon would refuse.",
        "type": "object",
        "required": ["schema", "authorize"],
        "additionalProperties": false,
        "properties": {
            "schema": {
                "description": "Schema identity. A different version is refused rather than reinterpreted.",
                "const": POLICY_SCHEMA
            },
            "authorize": {
                "description": "The dangerous operation kinds this policy pre-authorizes. At least one. These are kinds, not command names: `shutdown` covers both the typed command and `raw` naming that operation. Repeating a kind is accepted and collapses: the grants are a set.",
                "type": "array",
                "minItems": 1,
                "items": {
                    "description": "A kind this CLI classifies as dangerous.",
                    "enum": dangerous
                }
            },
            "expires_at": {
                "description": "Optional unix-millisecond bound. The expiry instant itself is still authorized; a lapsed policy grants nothing and the refusal names the file and the lapse.",
                "type": "integer",
                "minimum": 0,
                "maximum": u64::MAX
            }
        }
    })
}

/// The short names `symbiote schema <selector>` accepts, in table order.
pub fn selectors() -> Vec<&'static str> {
    PUBLISHED_SCHEMAS
        .iter()
        .map(|schema| schema.selector)
        .collect()
}

/// The documents an optional selector names: `None` is all of them, so the
/// bare `schema` command, `schema --write DIR` and `schema --check DIR` cover
/// both, while `schema envelope` narrows every action to one document.
pub fn selected_schemas(
    selector: Option<&str>,
) -> Result<Vec<&'static PublishedSchema>, SchemaError> {
    match selector {
        None => Ok(PUBLISHED_SCHEMAS.iter().collect()),
        Some(name) => PUBLISHED_SCHEMAS
            .iter()
            .find(|schema| schema.selector == name)
            .map(|schema| vec![schema])
            .ok_or_else(|| SchemaError::UnknownSelector {
                selector: name.to_owned(),
            }),
    }
}

/// Both published documents, keyed by the schema identity each declares. This
/// is exactly what `symbiote schema` prints.
pub fn published_schemas() -> serde_json::Value {
    let mut documents = serde_json::Map::new();
    for schema in PUBLISHED_SCHEMAS {
        documents.insert(schema.identity.to_owned(), (schema.build)());
    }
    serde_json::Value::Object(documents)
}

/// The canonical bytes of one published document: pretty JSON plus a trailing
/// newline. `write_schemas` writes exactly this and the contract tests compare
/// it to the committed fixture, so "what the binary emits" has one producer.
pub fn document_text(schema: &PublishedSchema) -> Result<String, SchemaError> {
    let mut text = serde_json::to_string_pretty(&(schema.build)()).map_err(|source| {
        SchemaError::Serialize {
            identity: schema.identity,
            source,
        }
    })?;
    text.push('\n');
    Ok(text)
}

/// Writes each selected document to `directory/<fixture file name>` in the
/// canonical form regeneration produces. Committing exactly what this writes
/// is what keeps the fixtures generated artifacts: running it again on a clean
/// tree leaves the files unchanged.
pub fn write_schemas(directory: &Path, schemas: &[&PublishedSchema]) -> Result<(), SchemaError> {
    for schema in schemas {
        let path = directory.join(schema.file_name);
        std::fs::write(&path, document_text(schema)?)
            .map_err(|source| SchemaError::Write { path, source })?;
    }
    Ok(())
}

/// One way a directory fails to be exactly what the binary publishes: a
/// selected document that is absent or different, or an entry that is not a
/// published document at all.
#[derive(Debug, PartialEq, Eq)]
pub struct Drift {
    pub file_name: String,
    /// `is missing`, `differs at line N`, or `is not a published document`.
    pub detail: String,
}

/// Compares each selected document's canonical bytes to `directory/<file
/// name>` **and** reports every entry in `directory` that is not a published
/// document, without writing anything.
///
/// Both directions are needed for this to be the same verdict the CI drift
/// step's `diff -ru` reaches for the same tree: with no selector `directory`
/// must hold exactly the published documents, so a missing, edited, extra or
/// renamed entry is drift. A selector narrows which documents' bytes are
/// compared; the other published documents may be present or absent, but an
/// entry that is not a published document is reported either way.
pub fn schema_drift(
    directory: &Path,
    schemas: &[&PublishedSchema],
) -> Result<Vec<Drift>, SchemaError> {
    let mut drift = Vec::new();
    for schema in schemas {
        let path = directory.join(schema.file_name);
        let expected = document_text(schema)?;
        match std::fs::read_to_string(&path) {
            Ok(actual) if actual == expected => {}
            Ok(actual) => drift.push(Drift {
                file_name: schema.file_name.to_owned(),
                detail: first_difference(&actual, &expected),
            }),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => drift.push(Drift {
                file_name: schema.file_name.to_owned(),
                detail: "is missing".to_owned(),
            }),
            Err(source) => return Err(SchemaError::Read { path, source }),
        }
    }
    // The other direction: `diff -ru` reports a file the binary does not write
    // as "Only in", so an added or renamed fixture is drift here too. A
    // directory that does not exist is already reported per document above, so
    // its `NotFound` is nothing further to say.
    let mut unexpected = Vec::new();
    match std::fs::read_dir(directory) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.map_err(|source| SchemaError::Read {
                    path: directory.to_path_buf(),
                    source,
                })?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if !PUBLISHED_SCHEMAS
                    .iter()
                    .any(|known| known.file_name == name.as_str())
                {
                    unexpected.push(name);
                }
            }
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(SchemaError::Read {
                path: directory.to_path_buf(),
                source,
            });
        }
    }
    unexpected.sort();
    for file_name in unexpected {
        drift.push(Drift {
            file_name,
            detail: "is not a published document".to_owned(),
        });
    }
    Ok(drift)
}

/// Where the first line differs, so a drift gate says *where* a regeneration
/// would change the file and not merely that it would. A difference past the
/// shorter document's last line (an extra or missing line) is reported at the
/// first line the two do not share.
fn first_difference(actual: &str, expected: &str) -> String {
    for (index, (left, right)) in actual.lines().zip(expected.lines()).enumerate() {
        if left != right {
            return format!("differs at line {}", index + 1);
        }
    }
    format!(
        "differs at line {}",
        actual.lines().count().min(expected.lines().count()) + 1
    )
}

/// The committed fixtures' directory, resolved from this crate's manifest so
/// the binary, the unit tests, the integration tests and any tooling agree on
/// one path instead of each spelling `../../docs/contracts/schemas`.
pub fn fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/schemas")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table is the single source of truth, so these pin the properties the
    /// rest of the CLI (and the help text) read out of it.
    #[test]
    fn the_operation_table_is_consistent_and_fails_closed() {
        let mut kinds: Vec<&str> = OPERATION_RISKS.iter().map(|(kind, _)| *kind).collect();
        let mut sorted = kinds.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(kinds.len(), sorted.len(), "the table has duplicate kinds");
        kinds.sort_unstable();
        // Every kind is found by lookup, and the declared entry is the risk.
        for (kind, risk) in OPERATION_RISKS {
            assert_eq!(known_risk_of_kind(kind), Some(*risk), "{kind}");
            assert_eq!(risk_of_kind(kind), *risk, "{kind}");
        }
        // A kind this build does not know is Dangerous (fail closed) but is
        // NOT nameable by a policy, because only `Some(Dangerous)` is.
        assert_eq!(known_risk_of_kind("delete_everything"), None);
        assert_eq!(risk_of_kind("delete_everything"), Risk::Dangerous);
        assert!(!dangerous_kinds().is_empty());
        for kind in dangerous_kinds() {
            assert!(risk_of_kind(kind).requires_authorization(), "{kind}");
        }
    }

    /// The committed fixtures are generated artifacts. This pins the builders
    /// to them in-process, so a change to a document fails without needing the
    /// binary or a daemon — the gap the subprocess-only tests left open.
    #[test]
    fn the_committed_fixtures_are_exactly_what_the_builders_emit() {
        for schema in PUBLISHED_SCHEMAS {
            let path = fixture_directory().join(schema.file_name);
            let committed = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(
                committed,
                document_text(schema).expect("a published document serializes"),
                "{} is not what the builder emits; regenerate it with `symbiote schema --write`",
                schema.file_name
            );
        }
        // The printed object is the same documents the writer produces.
        let printed = published_schemas();
        assert_eq!(printed.as_object().map(serde_json::Map::len), Some(2));
    }

    /// The selector is the whole naming rule, so a published document is
    /// addressable exactly when the table says so and no other way.
    #[test]
    fn selecting_narrows_the_published_set_and_rejects_an_unknown_name() {
        assert_eq!(selectors(), vec!["envelope", "policy"]);
        // No selector is every document; a selector is exactly one.
        assert_eq!(
            selected_schemas(None).unwrap().len(),
            PUBLISHED_SCHEMAS.len()
        );
        for name in selectors() {
            let selected = selected_schemas(Some(name)).unwrap();
            assert_eq!(selected.len(), 1, "{name}");
            assert_eq!(selected[0].selector, name);
        }
        // An identity is NOT a selector: the table exposes one naming scheme.
        let error = selected_schemas(Some(CLI_SCHEMA)).unwrap_err();
        let message = error.to_string();
        assert!(message.contains(CLI_SCHEMA), "{message}");
        assert!(
            message.contains("envelope") && message.contains("policy"),
            "{message}"
        );
    }

    /// `schema_drift` is the local counterpart of the CI step, so it must name
    /// the document that diverges and must read without writing.
    #[test]
    fn drift_is_named_per_document_and_reads_without_writing() {
        let directory =
            std::env::temp_dir().join(format!("symbiote-cli-schema-drift-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let selected = selected_schemas(None).unwrap();
        write_schemas(&directory, &selected).unwrap();
        assert_eq!(schema_drift(&directory, &selected).unwrap(), Vec::new());

        // A constraint change — the exact drift the CI step catches — is
        // reported by file name and line.
        let envelope = directory.join("symbiote.cli.v1.schema.json");
        let text = std::fs::read_to_string(&envelope).unwrap();
        std::fs::write(
            &envelope,
            text.replacen("\"minLength\": 1", "\"minLength\": 2", 1),
        )
        .unwrap();
        let drift = schema_drift(&directory, &selected).unwrap();
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].file_name, "symbiote.cli.v1.schema.json");
        assert!(drift[0].detail.starts_with("differs at line "), "{drift:?}");

        // The other direction: an entry the binary does not publish is drift,
        // however it is named. `diff -ru` reports the same tree as "Only in".
        std::fs::write(&envelope, &text).unwrap();
        let renamed = directory.join("symbiote.cli.v1.schema.json.bak");
        std::fs::write(&renamed, "{}").unwrap();
        let drift = schema_drift(&directory, &selected).unwrap();
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].file_name, "symbiote.cli.v1.schema.json.bak");
        assert_eq!(drift[0].detail, "is not a published document");
        // A selector narrows the byte comparison but not the entry check, and
        // the *other* published document is never "unexpected".
        let policy_only = selected_schemas(Some("policy")).unwrap();
        assert_eq!(schema_drift(&directory, &policy_only).unwrap().len(), 1);
        std::fs::remove_file(&renamed).unwrap();
        assert_eq!(schema_drift(&directory, &policy_only).unwrap(), Vec::new());

        // A missing document is reported as such, and checking does NOT
        // recreate it: the gate reads, it never writes.
        std::fs::remove_file(directory.join("symbiote.cli-policy.v1.schema.json")).unwrap();
        std::fs::remove_file(&envelope).unwrap();
        let drift = schema_drift(&directory, &selected).unwrap();
        assert_eq!(drift.len(), 2);
        assert!(
            drift.iter().all(|entry| entry.detail == "is missing"),
            "{drift:?}"
        );
        assert!(
            std::fs::read_dir(&directory).unwrap().next().is_none(),
            "a drift check must not write"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }
}
