//! The published JSON Schema fixtures, validated against the real binary.
//!
//! `docs/contracts/cli.md` publishes two documents: `symbiote.cli/v1`, the
//! envelope `--json` prints, and `symbiote.cli-policy/v1`, an authorization
//! policy file. What matters is not that the fixtures parse but that the
//! binary's **own output** conforms to them, and that the checker cannot pass
//! by ignoring a constraint. So the checker below is closed: it implements a
//! named subset of JSON Schema and refuses any keyword outside it.
mod support;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command as Process, Output, Stdio};

use support::*;

const ENVELOPE_SCHEMA: &str = "symbiote.cli.v1.schema.json";
const POLICY_SCHEMA: &str = "symbiote.cli-policy.v1.schema.json";

/// Loads a published fixture from `docs/contracts/schemas/`.
fn fixture(name: &str) -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/contracts/schemas")
        .join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is not JSON: {error}", path.display()))
}

/// The one envelope a `--json` invocation prints, with its exit code pinned.
fn envelope(output: &Output, expected_exit: i32) -> serde_json::Value {
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 1, "one envelope per invocation: {stdout}");
    serde_json::from_str(lines[0]).expect("the envelope is JSON")
}

/// Runs the CLI with no daemon listening, so the transport fails.
fn run_without_daemon(arguments: &[&str]) -> Output {
    let directory = unique_directory();
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = Process::new(CLI)
        .arg("--state-dir")
        .arg(&directory)
        .args(arguments)
        .stdin(Stdio::null())
        .env_remove("SYMBIOTE_CLI_POLICY")
        .output()
        .expect("the CLI binary runs");
    let _ = std::fs::remove_dir_all(&directory);
    output
}

/// A deliberately small, **closed** JSON Schema checker.
///
/// Closed is the point: a keyword this does not implement is an error rather
/// than a silent pass, so a fixture can never assert something the test
/// ignores. The subset is exactly what the two published fixtures use, and it
/// is documented in `docs/contracts/cli.md`. It is not a general validator and
/// does not claim to be one; a conforming draft 2020-12 validator can consume
/// the same fixtures independently.
mod bounded {
    use serde_json::Value;

    pub struct Checker<'a> {
        root: &'a Value,
    }

    const KEYWORDS: &[&str] = &[
        "$schema",
        "$id",
        "title",
        "description",
        "$defs",
        "$ref",
        "type",
        "const",
        "enum",
        "required",
        "properties",
        "additionalProperties",
        "items",
        "minItems",
        "minLength",
        "minimum",
        "maximum",
        "oneOf",
        "not",
    ];

    impl<'a> Checker<'a> {
        pub fn new(root: &'a Value) -> Self {
            Self { root }
        }

        /// Fails unless the fixture uses only supported keywords.
        pub fn assert_supported(&self) {
            self.walk(self.root, "$");
        }

        pub fn validate(&self, instance: &Value) -> Result<(), String> {
            let mut problems = Vec::new();
            self.check(self.root, instance, "$", &mut problems);
            if problems.is_empty() {
                Ok(())
            } else {
                Err(problems.join("; "))
            }
        }

        fn walk(&self, schema: &Value, path: &str) {
            for (keyword, value) in expect_object(schema, path, "a schema") {
                assert!(
                    KEYWORDS.contains(&keyword.as_str()),
                    "{path}: unsupported keyword `{keyword}` — add it to the checker and to the \
                     subset documented in docs/contracts/cli.md, or the fixture would assert \
                     something this test silently ignores"
                );
                let child = format!("{path}.{keyword}");
                match keyword.as_str() {
                    "$defs" | "properties" => {
                        for (name, sub) in expect_object(value, &child, "a map of schemas") {
                            self.walk(sub, &format!("{child}.{name}"));
                        }
                    }
                    "items" | "not" => self.walk(value, &child),
                    "oneOf" => {
                        for (index, sub) in expect_array(value, &child, "a list of schemas")
                            .iter()
                            .enumerate()
                        {
                            self.walk(sub, &format!("{child}[{index}]"));
                        }
                    }
                    "additionalProperties" => {
                        assert!(value.is_boolean(), "{child}: only a boolean is supported");
                    }
                    _ => {}
                }
            }
        }

        fn check(&self, schema: &Value, instance: &Value, path: &str, problems: &mut Vec<String>) {
            for (keyword, value) in expect_object(schema, path, "a schema") {
                match keyword.as_str() {
                    "$schema" | "$id" | "title" | "description" | "$defs" => {}
                    "$ref" => {
                        if let Some(target) = self.resolve(value, path, problems) {
                            self.check(&target, instance, path, problems);
                        }
                    }
                    "type" => {
                        if !self.type_accepts(value, instance) {
                            problems.push(format!(
                                "{path}: expected type {value}, found {}",
                                kind(instance)
                            ));
                        }
                    }
                    "const" => {
                        if instance != value {
                            problems.push(format!("{path}: expected {value}, found {instance}"));
                        }
                    }
                    "enum" => {
                        if !expect_array(value, path, "an enum")
                            .iter()
                            .any(|option| option == instance)
                        {
                            problems.push(format!("{path}: {instance} is not one of {value}"));
                        }
                    }
                    "required" => {
                        if let Some(object) = instance.as_object() {
                            for name in expect_array(value, path, "a required list") {
                                let name = name.as_str().expect("required names are strings");
                                if !object.contains_key(name) {
                                    problems.push(format!(
                                        "{path}: missing required property `{name}`"
                                    ));
                                }
                            }
                        }
                    }
                    "properties" => {
                        if let Some(object) = instance.as_object() {
                            for (name, sub) in expect_object(value, path, "a properties map") {
                                if let Some(member) = object.get(name) {
                                    self.check(sub, member, &format!("{path}.{name}"), problems);
                                }
                            }
                        }
                    }
                    "additionalProperties" => {
                        if value.as_bool() == Some(false) {
                            let declared = self.properties_of(schema);
                            if let Some(object) = instance.as_object() {
                                for name in object.keys() {
                                    if !declared.iter().any(|known| known == name) {
                                        problems
                                            .push(format!("{path}: unexpected property `{name}`"));
                                    }
                                }
                            }
                        }
                    }
                    "items" => {
                        if let Some(array) = instance.as_array() {
                            for (index, member) in array.iter().enumerate() {
                                self.check(value, member, &format!("{path}[{index}]"), problems);
                            }
                        }
                    }
                    "minItems" => {
                        if let Some(array) = instance.as_array() {
                            let least = value.as_u64().expect("minItems is a number");
                            if (array.len() as u64) < least {
                                problems.push(format!(
                                    "{path}: {} items is fewer than {least}",
                                    array.len()
                                ));
                            }
                        }
                    }
                    "minLength" => {
                        if let Some(text) = instance.as_str() {
                            let least = value.as_u64().expect("minLength is a number");
                            let length = text.chars().count() as u64;
                            if length < least {
                                problems.push(format!(
                                    "{path}: {length} characters is fewer than {least}"
                                ));
                            }
                        }
                    }
                    "minimum" => {
                        if compare(instance, value) == Some(std::cmp::Ordering::Less) {
                            problems
                                .push(format!("{path}: {instance} is below the minimum {value}"));
                        }
                    }
                    "maximum" => {
                        if compare(instance, value) == Some(std::cmp::Ordering::Greater) {
                            problems
                                .push(format!("{path}: {instance} is above the maximum {value}"));
                        }
                    }
                    "oneOf" => {
                        let branches = expect_array(value, path, "oneOf");
                        let matched = branches
                            .iter()
                            .filter(|branch| self.matches(branch, instance))
                            .count();
                        if matched != 1 {
                            problems.push(format!(
                                "{path}: exactly one of {} oneOf branches must match, {matched} did",
                                branches.len()
                            ));
                        }
                    }
                    "not" => {
                        if self.matches(value, instance) {
                            problems.push(format!("{path}: must not match {value}"));
                        }
                    }
                    other => problems.push(format!("{path}: unsupported keyword `{other}`")),
                }
            }
        }

        /// Validates into a fresh problem list, for `oneOf` and `not`.
        fn matches(&self, schema: &Value, instance: &Value) -> bool {
            let mut problems = Vec::new();
            self.check(schema, instance, "$", &mut problems);
            problems.is_empty()
        }

        fn resolve(
            &self,
            reference: &Value,
            path: &str,
            problems: &mut Vec<String>,
        ) -> Option<Value> {
            let reference = reference.as_str().expect("$ref is a string");
            let Some(name) = reference.strip_prefix("#/$defs/") else {
                problems.push(format!(
                    "{path}: only local `#/$defs/…` refs are supported, found {reference}"
                ));
                return None;
            };
            match self.root.get("$defs").and_then(|defs| defs.get(name)) {
                Some(target) => Some(target.clone()),
                None => {
                    problems.push(format!("{path}: unresolved ref {reference}"));
                    None
                }
            }
        }

        /// The property names a schema declares, used by `additionalProperties`.
        fn properties_of(&self, schema: &Value) -> Vec<String> {
            schema
                .get("properties")
                .and_then(Value::as_object)
                .map(|properties| properties.keys().cloned().collect())
                .unwrap_or_default()
        }

        fn type_accepts(&self, expected: &Value, instance: &Value) -> bool {
            match expected {
                Value::String(name) => matches_type(name, instance),
                Value::Array(names) => names.iter().any(|name| {
                    matches_type(name.as_str().expect("type names are strings"), instance)
                }),
                _ => panic!("type must be a string or an array of strings"),
            }
        }
    }

    fn expect_object<'a>(
        value: &'a Value,
        path: &str,
        what: &str,
    ) -> &'a serde_json::Map<String, Value> {
        value
            .as_object()
            .unwrap_or_else(|| panic!("{path}: expected {what}, found {}", kind(value)))
    }

    fn expect_array<'a>(value: &'a Value, path: &str, what: &str) -> &'a [Value] {
        value
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_else(|| panic!("{path}: expected {what}, found {}", kind(value)))
    }

    fn kind(instance: &Value) -> &'static str {
        match instance {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    fn matches_type(name: &str, instance: &Value) -> bool {
        match name {
            "object" => instance.is_object(),
            "array" => instance.is_array(),
            "string" => instance.is_string(),
            "boolean" => instance.is_boolean(),
            "null" => instance.is_null(),
            "number" => instance.is_number(),
            // Draft 2020-12 `integer` is a number with a zero fractional part.
            "integer" => {
                instance.as_i64().is_some()
                    || instance.as_u64().is_some()
                    || instance
                        .as_f64()
                        .is_some_and(|number| number.fract() == 0.0)
            }
            _ => false,
        }
    }

    /// Exact for representable integers, falling back to floating point; `None`
    /// for a non-number, where `minimum`/`maximum` do not apply.
    fn compare(instance: &Value, limit: &Value) -> Option<std::cmp::Ordering> {
        if let (Some(left), Some(right)) = (instance.as_u64(), limit.as_u64()) {
            return Some(left.cmp(&right));
        }
        if let (Some(left), Some(right)) = (instance.as_i64(), limit.as_i64()) {
            return Some(left.cmp(&right));
        }
        instance.as_f64()?.partial_cmp(&limit.as_f64()?)
    }
}

#[test]
fn the_fixtures_stay_within_the_checkers_subset() {
    // If a fixture used a keyword the checker did not implement, "the output
    // validates" would become a claim this test silently ignored. Fail first.
    for name in [ENVELOPE_SCHEMA, POLICY_SCHEMA] {
        bounded::Checker::new(&fixture(name)).assert_supported();
    }
}

#[test]
fn the_checker_rejects_every_way_a_real_envelope_could_be_wrong() {
    let schema = fixture(ENVELOPE_SCHEMA);
    let checker = bounded::Checker::new(&schema);
    let (output, _) = run_cli(&["health", "--json"]);
    let real = envelope(&output, 0);
    checker
        .validate(&real)
        .unwrap_or_else(|problems| panic!("a real envelope must conform: {problems}\n{real}"));

    // Each mutation is a contract violation; if any were accepted, the
    // positive assertions above would prove nothing about it. `ok` is flipped
    // alongside the error cases so only the *shape* is under test, not the
    // oneOf branch.
    let mut mutations: Vec<(&str, serde_json::Value)> = Vec::new();
    let mut missing = real.clone();
    missing.as_object_mut().unwrap().remove("schema");
    mutations.push(("a missing schema", missing));
    let mut wrong_schema = real.clone();
    wrong_schema["schema"] = serde_json::json!("symbiote.cli/v2");
    mutations.push(("a different schema string", wrong_schema));
    let mut empty_id = real.clone();
    empty_id["command_id"] = serde_json::json!("");
    mutations.push(("an empty command_id", empty_id));
    let mut wrong_ok = real.clone();
    wrong_ok["ok"] = serde_json::json!("true");
    mutations.push(("a non-boolean ok", wrong_ok));
    let mut extra = real.clone();
    extra["extra"] = serde_json::json!(1);
    mutations.push(("an undeclared property", extra));
    let mut both = real.clone();
    both["error"] = serde_json::json!({ "code": "x", "message": "y" });
    mutations.push(("both a result and an error", both));
    let mut neither = real.clone();
    neither.as_object_mut().unwrap().remove("result");
    mutations.push(("neither a result nor an error", neither));
    let mut no_message = real.clone();
    no_message.as_object_mut().unwrap().remove("result");
    no_message["ok"] = serde_json::json!(false);
    no_message["error"] = serde_json::json!({ "code": "x" });
    mutations.push(("an error without a message", no_message));
    let mut empty_code = real.clone();
    empty_code.as_object_mut().unwrap().remove("result");
    empty_code["ok"] = serde_json::json!(false);
    empty_code["error"] = serde_json::json!({ "code": "", "message": "y" });
    mutations.push(("an empty error code", empty_code));
    let mut no_kind = real.clone();
    no_kind["result"] = serde_json::json!({});
    mutations.push(("a result without a kind", no_kind));

    for (what, mutated) in mutations {
        assert!(
            checker.validate(&mutated).is_err(),
            "the checker accepted {what}: {mutated}"
        );
    }
}

#[test]
fn a_real_success_envelope_conforms() {
    let schema = fixture(ENVELOPE_SCHEMA);
    let checker = bounded::Checker::new(&schema);
    let (output, frame) = run_cli(&["health", "--json"]);
    assert!(frame.is_some(), "health is sent without any authorization");
    let real = envelope(&output, 0);
    assert_eq!(real["ok"], true);
    checker
        .validate(&real)
        .unwrap_or_else(|problems| panic!("{problems}\n{real}"));
}

#[test]
fn a_real_authorization_refusal_envelope_conforms() {
    let schema = fixture(ENVELOPE_SCHEMA);
    let checker = bounded::Checker::new(&schema);
    // No tty, no `--yes`, no policy: refused before connecting.
    let (output, frame) = run_cli(&["shutdown", "--json"]);
    assert!(frame.is_none(), "an unauthorized command must not connect");
    let real = envelope(&output, 3);
    assert_eq!(real["error"]["code"], "authorization_required");
    checker
        .validate(&real)
        .unwrap_or_else(|problems| panic!("{problems}\n{real}"));
}

#[test]
fn a_real_daemon_refusal_envelope_conforms() {
    let schema = fixture(ENVELOPE_SCHEMA);
    let checker = bounded::Checker::new(&schema);
    let policy = write_policy(r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"]}"#);
    let path = policy.to_str().unwrap();
    let (output, frame) = run_cli_full(
        &["--policy", path, "shutdown", "--json"],
        None,
        Answer::Refuse,
    );
    assert!(frame.is_some(), "the policy did authorize sending");
    let real = envelope(&output, 2);
    assert_eq!(real["ok"], false);
    assert_eq!(real["error"]["code"], "permission_denied");
    checker
        .validate(&real)
        .unwrap_or_else(|problems| panic!("{problems}\n{real}"));
}

#[test]
fn a_real_policy_invalid_envelope_conforms() {
    let schema = fixture(ENVELOPE_SCHEMA);
    let checker = bounded::Checker::new(&schema);
    let bogus = write_policy(r#"{"schema":"symbiote.cli-policy/v9","authorize":["shutdown"]}"#);
    let path = bogus.to_str().unwrap();
    let (output, frame) = run_cli(&["--policy", path, "shutdown", "--json"]);
    assert!(frame.is_none(), "an unusable policy sends nothing");
    let real = envelope(&output, 1);
    assert_eq!(real["error"]["code"], "policy_invalid");
    checker
        .validate(&real)
        .unwrap_or_else(|problems| panic!("{problems}\n{real}"));
}

#[test]
fn a_real_unreachable_envelope_conforms() {
    let schema = fixture(ENVELOPE_SCHEMA);
    let checker = bounded::Checker::new(&schema);
    let output = run_without_daemon(&["health", "--json"]);
    let real = envelope(&output, 1);
    assert_eq!(real["error"]["code"], "unreachable");
    checker
        .validate(&real)
        .unwrap_or_else(|problems| panic!("{problems}\n{real}"));
}

#[test]
fn the_policy_fixture_and_the_cli_classify_documents_identically() {
    let schema = fixture(POLICY_SCHEMA);
    let checker = bounded::Checker::new(&schema);
    // `true` means the fixture accepts the document, and the CLI must therefore
    // not refuse it as an unusable policy. The CLI may still decline to
    // *authorize* a command (exit 3) — that is a different question.
    let cases: &[(bool, &str)] = &[
        (
            true,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"]}"#,
        ),
        (
            true,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["start_prepared_task"]}"#,
        ),
        (
            true,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"],"expires_at":4102444800000}"#,
        ),
        (
            true,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"],"expires_at":18446744073709551615}"#,
        ),
        // Duplicates are accepted and collapse, so the fixture must not claim
        // `uniqueItems` while the CLI deduplicates them into a set.
        (
            true,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown","shutdown"]}"#,
        ),
        (
            false,
            r#"{"schema":"symbiote.cli-policy/v9","authorize":["shutdown"]}"#,
        ),
        (
            false,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":[]}"#,
        ),
        (false, r#"{"schema":"symbiote.cli-policy/v1"}"#),
        (
            false,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["health"]}"#,
        ),
        (
            false,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["create_task"]}"#,
        ),
        (
            false,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["delete_everything"]}"#,
        ),
        (
            false,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["*"]}"#,
        ),
        (
            false,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":"shutdown"}"#,
        ),
        (
            false,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"],"authorise":1}"#,
        ),
        (
            false,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"],"expires_at":-1}"#,
        ),
        (
            false,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"],"expires_at":1.5}"#,
        ),
        (
            false,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"],"expires_at":true}"#,
        ),
        (
            false,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"],"expires_at":"soon"}"#,
        ),
    ];
    for (accepted, document) in cases {
        let value: serde_json::Value =
            serde_json::from_str(document).expect("every case is a JSON document");
        let outcome = checker.validate(&value);
        assert_eq!(
            outcome.is_ok(),
            *accepted,
            "the fixture disagrees with itself on {document}: {outcome:?}"
        );
        let policy = write_policy(document);
        let path = policy.to_str().unwrap();
        let (output, frame) = run_cli(&["--policy", path, "shutdown"]);
        let refused = output.status.code() == Some(1);
        assert_eq!(
            !refused,
            *accepted,
            "the CLI disagrees with the fixture on {document}: exit {:?}, {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        if refused {
            assert!(
                frame.is_none(),
                "a refused policy must send nothing: {document}"
            );
        }
    }
}
