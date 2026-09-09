//! Tool execution for the native loop (#465): the host-gated boundary
//! between the model's proposed tool call and a real side effect. The
//! native loop validates tool proposals against the dispatch contract's
//! required tools; this module decides, per proposal, what the Host may
//! actually run and records the decision as tracker-ingestible events.
//!
//! Boundary: execution goes through the Host sandbox (bubblewrap via the
//! trusted launcher), never a raw process spawn. The first supported tool
//! class is `shell` — a bounded command run inside the reserved worktree
//! with the sandbox's WorktreeWrite profile, no network, and a private
//! HOME. Arguments come from the model's JSON arguments object and are
//! never shell-expanded: the sandbox launcher rejects any program outside
//! /usr/bin and treats every argument as a literal.
//!
//! The executor is injected. The production implementation composes the
//! sandbox launch; tests use deterministic scripted executors — a model
//! cannot cause a real side effect in any test.
use serde::{Deserialize, Serialize};
use symbiote_runtime_sdk::events::{EventText, RuntimeEventKind};

/// The only tool classes the Host will execute today, matched against the
/// dispatch contract's required-tool names. A required tool outside this
/// set stays propose-only: recorded, never executed, never refused — the
/// contract declared it, so a refusal would be a contract violation, but
/// the Host has no safe execution path for it yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolClass {
    Shell,
}

/// Classifies a required-tool name. `shell` (and its explicit alias
/// `shell-command`) maps to the sandboxed command class; everything else
/// is `None` — propose-only.
pub fn classify(name: &str) -> Option<ToolClass> {
    match name {
        "shell" | "shell-command" => Some(ToolClass::Shell),
        _ => None,
    }
}

/// The model's tool arguments, strictly validated before any execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellInvocation {
    /// The program to run, as a bare name (`cargo`, `ls`) or an absolute
    /// path. Bare names are resolved by the sandbox launcher against
    /// /usr/bin only.
    pub program: String,
    /// Literal arguments, never shell-expanded.
    pub arguments: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellArgsError {
    /// The arguments JSON was not an object with the expected fields.
    InvalidShape,
    /// The program field was missing, empty, over-bound, or contained a
    /// NUL (the sandbox launcher's argument bound).
    InvalidProgram,
    /// Too many arguments, an argument over-bound, or a NUL byte.
    InvalidArguments,
}

/// Parses and bounds the model's `arguments` object for the shell class.
/// Nothing is executed here; the output is data for the injected executor.
pub fn parse_shell_arguments(
    arguments: &serde_json::Value,
) -> Result<ShellInvocation, ShellArgsError> {
    let object = arguments.as_object().ok_or(ShellArgsError::InvalidShape)?;
    let program = object
        .get("program")
        .and_then(serde_json::Value::as_str)
        .ok_or(ShellArgsError::InvalidProgram)?;
    if program.is_empty() || program.len() > 256 || program.contains('\0') {
        return Err(ShellArgsError::InvalidProgram);
    }
    let arguments_value = object
        .get("arguments")
        .ok_or(ShellArgsError::InvalidArguments)?;
    let list = arguments_value
        .as_array()
        .ok_or(ShellArgsError::InvalidArguments)?;
    if list.len() > 64 {
        return Err(ShellArgsError::InvalidArguments);
    }
    let mut parsed = Vec::with_capacity(list.len());
    for value in list {
        let argument = value.as_str().ok_or(ShellArgsError::InvalidArguments)?;
        if argument.len() > 65_536 || argument.contains('\0') {
            return Err(ShellArgsError::InvalidArguments);
        }
        parsed.push(argument.to_owned());
    }
    Ok(ShellInvocation {
        program: program.to_owned(),
        arguments: parsed,
    })
}

/// The recorded result of one executed tool: bounded output and the exit
/// code, tracker-ingestible through the loop's event stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolExecution {
    pub call_id: String,
    pub name: String,
    pub exit_code: Option<i32>,
    /// The tool's stdout, truncated to the SDK's event-text bound with a
    /// visible marker.
    pub output: EventText,
}

/// The tool execution boundary. Production composes the sandbox launch
/// (WorktreeWrite profile, reserved worktree, no network, private HOME);
/// tests script outputs and failures — no real side effect is possible in
/// a test.
pub trait ShellToolExecutor {
    /// Runs one shell invocation inside the reserved worktree and returns
    /// its bounded stdout and exit code. `Err` means the invocation could
    /// not be completed (transport/consent/launch failure) — distinct from
    /// a tool that ran and exited nonzero.
    fn run_shell(
        &mut self,
        invocation: &ShellInvocation,
        worktree: &std::path::Path,
    ) -> Result<(Vec<u8>, Option<i32>), ToolExecError>;
}

/// A minimal error type so the tool module does not depend on the git
/// module's error identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolExecError {
    Execution,
}

impl std::fmt::Display for ToolExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tool execution failed: {self:?}")
    }
}
impl std::error::Error for ToolExecError {}

/// The total output recorded per executed tool.
pub const MAX_TOOL_OUTPUT_BYTES: usize = 16_384;

/// Truncates tool output to the event-text bound on a char boundary with a
/// visible marker (the same discipline as the loop's other event text).
fn tool_output_event(bytes: &[u8]) -> EventText {
    let text = String::from_utf8_lossy(bytes);
    const MARKER: &str = "…[truncated]";
    if text.len() <= crate::MAX_EVENT_TEXT_BYTES {
        return EventText::new(text.into_owned()).expect("within the checked bound");
    }
    let budget = crate::MAX_EVENT_TEXT_BYTES - MARKER.len();
    let mut end = budget;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = String::from(&text[..end]);
    truncated.push_str(MARKER);
    EventText::new(truncated).expect("bounded by construction")
}

/// Executes one validated tool proposal through the injected executor and
/// returns the tracker-ingestible event sequence: ToolStarted, then
/// ToolCompleted or ToolFailed. The proposal's call_id and name are
/// carried verbatim; the output is bounded; a nonzero exit is a ToolFailed
/// with the exit code recorded (a tool that ran and failed is a fact, not
/// a transport error).
pub fn execute_shell_tool(
    executor: &mut impl ShellToolExecutor,
    call_id: &symbiote_domain::RequestId,
    worktree: &std::path::Path,
    invocation: &ShellInvocation,
) -> Result<Vec<RuntimeEventKind>, ToolExecError> {
    let (output_bytes, exit_code) = executor.run_shell(invocation, worktree)?;
    let output = tool_output_event(&output_bytes);
    let mut events = vec![RuntimeEventKind::ToolStarted {
        tool_call_id: call_id.clone(),
    }];
    match exit_code {
        Some(0) => events.push(RuntimeEventKind::ToolCompleted {
            tool_call_id: call_id.clone(),
            output,
        }),
        _ => events.push(RuntimeEventKind::ToolFailed {
            tool_call_id: call_id.clone(),
            error: EventText::new(format!(
                "shell tool exited {}: {}",
                exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".into()),
                output.as_str()
            ))
            .expect("static prefix plus bounded output fits"),
        }),
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scripted executor: outputs and exit codes consumed in order. No real
    /// side effect is possible in a test.
    struct ScriptedShell {
        results: VecDeque<ScriptedResult>,
        invocations: Vec<ShellInvocation>,
    }
    impl ScriptedShell {
        fn new(results: Vec<ScriptedResult>) -> Self {
            Self {
                results: results.into(),
                invocations: Vec::new(),
            }
        }
    }
    impl ShellToolExecutor for ScriptedShell {
        fn run_shell(
            &mut self,
            invocation: &ShellInvocation,
            _worktree: &std::path::Path,
        ) -> Result<(Vec<u8>, Option<i32>), ToolExecError> {
            self.invocations.push(invocation.clone());
            self.results
                .pop_front()
                .unwrap_or(Err(ToolExecError::Execution))
        }
    }

    use std::collections::VecDeque;

    type ScriptedResult = Result<(Vec<u8>, Option<i32>), ToolExecError>;

    fn invocation() -> ShellInvocation {
        ShellInvocation {
            program: "cargo".into(),
            arguments: vec!["test".into()],
        }
    }

    #[test]
    fn classification_matches_the_shell_class_only() {
        assert_eq!(classify("shell"), Some(ToolClass::Shell));
        assert_eq!(classify("shell-command"), Some(ToolClass::Shell));
        assert_eq!(classify("edit_file"), None);
        assert_eq!(classify(""), None);
    }

    #[test]
    fn shell_arguments_parse_strictly() {
        let good = serde_json::json!({
            "program": "cargo",
            "arguments": ["test", "--locked"],
        });
        let parsed = parse_shell_arguments(&good).unwrap();
        assert_eq!(parsed.program, "cargo");
        assert_eq!(
            parsed.arguments,
            vec!["test".to_string(), "--locked".to_string()]
        );
        // Non-object shape.
        assert_eq!(
            parse_shell_arguments(&serde_json::json!(["cargo"])),
            Err(ShellArgsError::InvalidShape)
        );
        // Missing program.
        assert_eq!(
            parse_shell_arguments(&serde_json::json!({"arguments": []})),
            Err(ShellArgsError::InvalidProgram)
        );
        // NUL in program.
        assert_eq!(
            parse_shell_arguments(&serde_json::json!({"program": "a\0b", "arguments": []})),
            Err(ShellArgsError::InvalidProgram)
        );
        // Arguments not a list.
        assert_eq!(
            parse_shell_arguments(&serde_json::json!({"program": "ls", "arguments": "x"})),
            Err(ShellArgsError::InvalidArguments)
        );
        // Over 64 arguments.
        let many = serde_json::json!({"program": "ls",
            "arguments": (0..65).map(|i| i.to_string()).collect::<Vec<_>>()});
        assert_eq!(
            parse_shell_arguments(&many),
            Err(ShellArgsError::InvalidArguments)
        );
    }

    #[test]
    fn execution_emits_started_then_completed_with_bounded_output() {
        let mut executor = ScriptedShell::new(vec![Ok((b"test result: ok".to_vec(), Some(0)))]);
        let events = execute_shell_tool(
            &mut executor,
            &symbiote_domain::RequestId::new("call-1").unwrap(),
            std::path::Path::new("/w"),
            &invocation(),
        )
        .unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            RuntimeEventKind::ToolStarted { tool_call_id } if tool_call_id.as_str() == "call-1"
        ));
        match &events[1] {
            RuntimeEventKind::ToolCompleted {
                tool_call_id,
                output,
            } => {
                assert_eq!(tool_call_id.as_str(), "call-1");
                assert_eq!(output.as_str(), "test result: ok");
            }
            other => panic!("expected ToolCompleted, got {other:?}"),
        }
        assert_eq!(executor.invocations.len(), 1);
        assert_eq!(executor.invocations[0].program, "cargo");
    }

    #[test]
    fn nonzero_exit_is_tool_failed_with_the_code_recorded() {
        let mut executor = ScriptedShell::new(vec![Ok((b"error: boom".to_vec(), Some(101)))]);
        let events = execute_shell_tool(
            &mut executor,
            &symbiote_domain::RequestId::new("call-2").unwrap(),
            std::path::Path::new("/w"),
            &invocation(),
        )
        .unwrap();
        match &events[1] {
            RuntimeEventKind::ToolFailed { error, .. } => {
                assert!(error.as_str().contains("101"));
                assert!(error.as_str().contains("boom"));
            }
            other => panic!("expected ToolFailed, got {other:?}"),
        }
    }

    #[test]
    fn oversized_output_truncates_on_char_boundaries() {
        let big = "🔍".repeat(MAX_TOOL_OUTPUT_BYTES); // 4-byte chars
        let mut executor = ScriptedShell::new(vec![Ok((big.into_bytes(), Some(0)))]);
        let events = execute_shell_tool(
            &mut executor,
            &symbiote_domain::RequestId::new("call-3").unwrap(),
            std::path::Path::new("/w"),
            &invocation(),
        )
        .unwrap();
        match &events[1] {
            RuntimeEventKind::ToolCompleted { output, .. } => {
                assert!(output.as_str().len() <= MAX_TOOL_OUTPUT_BYTES);
                assert!(output.as_str().ends_with("…[truncated]"));
                assert!(!output.as_str().contains(char::REPLACEMENT_CHARACTER));
            }
            other => panic!("expected ToolCompleted, got {other:?}"),
        }
    }

    #[test]
    fn executor_failure_is_a_distinct_execution_error() {
        let mut executor = ScriptedShell::new(vec![Err(ToolExecError::Execution)]);
        assert_eq!(
            execute_shell_tool(
                &mut executor,
                &symbiote_domain::RequestId::new("call-4").unwrap(),
                std::path::Path::new("/w"),
                &invocation(),
            ),
            Err(ToolExecError::Execution)
        );
    }
}
