//! The first-release demonstration sequence (#54): open a repository,
//! describe a coding task, execute it through a started dispatch, and
//! inspect the completion evidence and worktree diff — driven entirely
//! through `symbiote-client-sdk` against the operator-provisioned daemon.
//!
//! The composition reuses the checked-in fixture payloads
//! (`fixtures/project-team`, `fixtures/workforce-bindings`) with exactly
//! the substitutions the demo needs (this Host's id, the repository's real
//! HEAD as the stream base, the `shell` required tool, the repository
//! placement). Everything the daemon does around the fixture model turn —
//! worktree provisioning, sandboxed tool execution, completion-evidence
//! filing — is real; the model turn itself is the operator's explicitly
//! labeled fixture transport, and a live one stays gated on user
//! authorization for credentials and billing.
use crate::{Driver, WorkflowError, WorktreeEvidence};
use std::path::{Path, PathBuf};
use symbiote_protocol::ResponseBody;

/// The demo's fixed canonical identities (matching the checked-in
/// fixtures): one project, one root, one objective-owned task.
pub const PROJECT: &str = "staffing-demo";
pub const ROOT: &str = "staffing-root";
pub const TASK: &str = "staffing-task";
pub const STREAM: &str = "staffing-stream";

/// What the first-release sequence observed, for the caller (desktop UI,
/// test, operator) to inspect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DemoOutcome {
    pub task_state: String,
    /// The worker's completion report (evidence — never verified
    /// completion).
    pub report: Option<String>,
    /// The journal position the driver reached while reading the
    /// completion evidence.
    pub journal_cursor: u64,
    /// The real worktree status: what the run produced.
    pub worktree: WorktreeEvidence,
    pub dispatch_id: String,
}

/// One driver for the whole sequence; connect once per daemon lifetime.
pub struct DemoWorkflow {
    driver: Driver,
    /// The operator's reservation base (for worktree evidence).
    reservation_base: PathBuf,
    /// The open repository's path (registered as the Root placement).
    repository: PathBuf,
}

impl DemoWorkflow {
    pub fn connect(
        state_directory: &Path,
        reservation_base: &Path,
        repository: &Path,
    ) -> Result<Self, WorkflowError> {
        Ok(Self {
            driver: Driver::connect(state_directory)?,
            reservation_base: reservation_base.to_path_buf(),
            repository: repository.to_path_buf(),
        })
    }

    /// Restores journal positions after a driver restart.
    pub fn with_positions(mut self, positions: Vec<symbiote_client_sdk::JournalPosition>) -> Self {
        self.driver = self.driver.with_positions(positions);
        self
    }

    pub fn journal_position(
        &self,
        project: &symbiote_domain::ProjectId,
    ) -> symbiote_protocol::JournalCursor {
        self.driver.journal_position(project)
    }

    fn register_operation() -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../fixtures/project-team/register.json"
        ))
        .expect("fixture envelope")
        .get("operation")
        .cloned()
        .expect("fixture operation")
    }

    fn team_operation() -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../fixtures/project-team/configure.json"
        ))
        .expect("fixture envelope")
        .get("operation")
        .cloned()
        .expect("fixture operation")
    }

    fn binding_operation_fixture() -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../fixtures/workforce-bindings/configure.json"
        ))
        .expect("fixture envelope")
        .get("operation")
        .cloned()
        .expect("fixture operation")
    }

    /// The first half of the sequence: open the repository, describe the
    /// task, route, prepare, and START the dispatch. Everything here is
    /// journaled canonical state — it survives a daemon SIGKILL, which is
    /// exactly what the restart/resume proof does between the halves.
    pub fn start_demo(&mut self) -> Result<String, WorkflowError> {
        let project = symbiote_domain::ProjectId::new(PROJECT).expect("fixture project");
        // 1. Open the repository: the real HEAD becomes the stream base.
        // The register fixture carries an empty host_paths map — client
        // declared placements are refused — and the placement is observed
        // by the Host right after registration.
        let mut git = symbiote_repo::SystemGit::new();
        let head = symbiote_repo::observe_head(&mut git, &self.repository)
            .map_err(|_| WorkflowError::Socket)?;
        let base = head.commit.clone();
        let target = "b".repeat(40);
        let host_id = self.host_pulse()?;
        self.call("wf-register", Self::register_operation(), Some(&project))?;
        self.call(
            "wf-observe-placement",
            serde_json::json!({"kind":"observe_root_placement","project_id":PROJECT,
            "root_id":ROOT,"host_id":host_id,
            "path":self.repository.display().to_string(),"expected_revision":0}),
            Some(&project),
        )?;
        // 2. Team: the engineer may mutate streams (the demo's worker
        // writes in the reserved worktree).
        let mut team = Self::team_operation();
        for pointer in [
            "/team/access_ceiling/grants",
            "/team/members/1/access/grants",
        ] {
            let grants = team
                .pointer_mut(pointer)
                .and_then(|g| g.as_array().cloned())
                .ok_or(WorkflowError::UnexpectedBody)?;
            let mut grants = grants;
            grants.push(serde_json::json!("mutate_stream"));
            grants.push(serde_json::json!("use_credential"));
            *team.pointer_mut(pointer).unwrap() = serde_json::json!(grants);
        }
        self.call("wf-team", team, Some(&project))?;
        // 3. Binding: this Host eligible, stream mutation + credential use
        // granted, the `shell` tool declared (primary.tools must equal
        // binding.required_tools — the workforce validator's rule).
        let binding = self.binding_operation(&host_id)?;
        self.call("wf-binding", binding, Some(&project))?;
        // 4. Provider connection and model descriptor (canonical facts the
        // contract's profile references; the fixture transport ignores the
        // endpoint — nothing contacts it).
        self.call(
            "wf-provider",
            serde_json::json!({"kind":"replace_provider_connection","attribution":"staffing-demo",
            "connection":{"id":"native-openai","adapter":"openai-responses",
            "endpoint_reference":"https://api.openai.example/v1","authentication":"api_credential"}}),
            Some(&project),
        )?;
        self.call(
            "wf-model",
            serde_json::json!({"kind":"replace_model_descriptor","attribution":"staffing-demo",
            "descriptor":{"schema_version":1,"id":"coding-model","provider_id":"native-openai",
            "context_window_tokens":16384,"max_output_tokens":4096,
            "capabilities":{"reasoning_efforts":[],"tools":true,"images":false,"streaming":false}}}),
            Some(&project),
        )?;
        // 5. Describe the coding task: a classified objective the task
        // hangs off, with the requirements the run's context will carry.
        // The stream's worktree identity and branch are the seed-derived
        // ones provisioning verifies — the driver derives them exactly like
        // the Host does, from the stream's recorded identity.
        self.call(
            "wf-objective",
            serde_json::json!({"kind":"create_work","work":{"id":{"kind":"objective","id":"staffing-objective"},
            "project_id":"staffing-demo","role_id":"engineer","title":"Implement a bounded change",
            "description":"Implement the bounded change described by this objective.",
            "utterance":null,"objective_class":"maintenance","parent":null,"dependencies":[],
            "requirements":["the produced file exists"],"constraints":[],"risks":[],
            "acceptance":["produced file content is the worker's"],"priority":2,
            "budget":null,"external_references":[]}}),
            Some(&project),
        )?;
        let stream_id = symbiote_domain::ChangeStreamId::new(STREAM).expect("fixture stream");
        let root_id = symbiote_domain::RootId::new(ROOT).expect("fixture root");
        let digest = symbiote_trust::Fingerprint::of(stream_id.as_str().as_bytes());
        let seed =
            symbiote_worktrees::policy_seed(digest.as_str()).map_err(|_| WorkflowError::Socket)?;
        let derived = symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
            project_id: &project,
            root_id: &root_id,
            stream_id: &stream_id,
            seed,
        })
        .map_err(|_| WorkflowError::Socket)?;
        self.call(
            "wf-task",
            serde_json::json!({"kind":"create_task","task":{"id":TASK,"project_id":PROJECT,
            "root_id":ROOT,"role_id":"engineer",
            "origin":{"kind":"objective","work":{"project_id":PROJECT,
                "id":{"kind":"objective","id":"staffing-objective"}}},
            "task_contract":{"id":"coding-contract","revision":1},
            "stream":{"id":STREAM,"originating_chat":"chat",
            "worktree":derived.worktree_id.as_str(),
            "branch":derived.branch,"base":base,
            "target":target}}}),
            Some(&project),
        )?;
        // 6. Route, prepare, start.
        self.call(
            "wf-route",
            serde_json::json!({"kind":"record_route",
            "request":{"project_id":PROJECT,"work_id":{"kind":"objective","id":"staffing-objective"},
            "requested":"engineer","domains":[]}}),
            Some(&project),
        )?;
        self.call(
            "wf-prepare",
            serde_json::json!({"kind":"prepare_dispatch","task_id":TASK}),
            Some(&project),
        )?;
        let started = self.call(
            "wf-start",
            serde_json::json!({"kind":"start_prepared_task","task_id":TASK,"host_id":host_id}),
            Some(&project),
        )?;
        let dispatch_id = match started {
            ResponseBody::StartedDispatch(started) => started.dispatch_id,
            _ => return Err(WorkflowError::UnexpectedBody),
        };
        Ok(dispatch_id.to_string())
    }

    /// Reads the durable journal trail from the driver's current position to
    /// the head, advancing the tracked position. Returns the reached cursor.
    /// This is how a desktop shell follows the evidence trail — and what a
    /// restarted driver restores via [`Self::with_positions`].
    pub fn read_journal(&mut self) -> Result<u64, WorkflowError> {
        self.journal_to_head().map(|(cursor, _)| cursor)
    }

    /// The second half: execute the started dispatch (real provisioning,
    /// real sandboxed tool execution, fixture model turn), then read the
    /// completion evidence, the journal trail, and the worktree diff.
    /// `dispatch_id` comes from [`Self::start_demo`] — possibly across a
    /// daemon restart.
    pub fn finish_demo(&mut self, dispatch_id: &str) -> Result<DemoOutcome, WorkflowError> {
        let project = symbiote_domain::ProjectId::new(PROJECT).expect("fixture project");
        let run = self.call(
            "wf-run",
            serde_json::json!({"kind":"run_started_dispatch","task_id":TASK,
            "dispatch_id":dispatch_id}),
            Some(&project),
        )?;
        let _completed = match &run {
            ResponseBody::WorkerRun(run) => run.completed,
            _ => return Err(WorkflowError::UnexpectedBody),
        };
        // 8. Completion is evidence: the task is CompletionRequested, and
        // Host verification + independent review remain the gates.
        let task = self.call(
            "wf-task-read",
            serde_json::json!({"kind":"get_task","project_id":PROJECT,"task_id":TASK}),
            Some(&project),
        )?;
        let task_state = match &task {
            ResponseBody::Task(task) => serde_json::to_value(task.state())
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .ok_or(WorkflowError::UnexpectedBody)?,
            _ => return Err(WorkflowError::UnexpectedBody),
        };
        // 9. The journal is the durable evidence trail; read our position.
        let mut report = None;
        let (_cursor, events) = self.journal_to_head()?;
        for event in &events {
            if let symbiote_protocol::EventPayload::TaskChanged { command, .. } = &event.payload {
                if let symbiote_domain::TaskAction::RequestCompletion {
                    report: completion_report,
                    ..
                } = &command.action
                {
                    report = Some(completion_report.clone());
                }
            }
        }
        let journal_cursor = self.driver.journal_position(&project).0;
        // 10. The diff: what the run really produced in the reserved
        // worktree (observed from the filesystem, not from the protocol).
        let worktree = crate::observe_worktree_evidence(
            &self.reservation_base,
            &symbiote_domain::ProjectId::new(PROJECT).expect("fixture project"),
            &symbiote_domain::RootId::new(ROOT).expect("fixture root"),
            &symbiote_domain::ChangeStreamId::new(STREAM).expect("fixture stream"),
        )?;
        Ok(DemoOutcome {
            task_state,
            report,
            journal_cursor,
            worktree,
            dispatch_id: dispatch_id.to_owned(),
        })
    }

    /// Pages the project journal from the driver's tracked position to the
    /// head, returning the events and the reached cursor. The session
    /// advances its per-project position as each validated page arrives,
    /// which is what survives a restart via the recorded positions.
    fn journal_to_head(
        &mut self,
    ) -> Result<(u64, Vec<symbiote_protocol::JournalEvent>), WorkflowError> {
        let project = symbiote_domain::ProjectId::new(PROJECT).expect("fixture project");
        let mut cursor = self.driver.journal_position(&project).0;
        let mut events = Vec::new();
        loop {
            let page = self.call(
                &format!("wf-journal-{cursor}"),
                serde_json::json!({"kind":"read_journal","project_id":PROJECT,
                "after":cursor,"limit":100}),
                Some(&project),
            )?;
            let ResponseBody::Journal(page) = page else {
                return Err(WorkflowError::UnexpectedBody);
            };
            cursor = page.next_cursor.0;
            let has_more = page.has_more;
            events.extend(page.events);
            if !has_more {
                break;
            }
        }
        Ok((cursor, events))
    }

    fn host_pulse(&mut self) -> Result<String, WorkflowError> {
        let pulse = self.call(
            "wf-pulse",
            serde_json::json!({"kind":"get_host_pulse"}),
            None,
        )?;
        match pulse {
            ResponseBody::HostPulse(pulse) => Ok(pulse.host_id.to_string()),
            _ => Err(WorkflowError::UnexpectedBody),
        }
    }

    /// The checked-in binding fixture, patched for the demo: this Host
    /// eligible, stream mutation + credential use granted, and the `shell`
    /// tool declared (both sides of the validator's equality rule).
    fn binding_operation(&self, host_id: &str) -> Result<serde_json::Value, WorkflowError> {
        let mut binding = Self::binding_operation_fixture();
        binding["configuration"]["primary"]["profile"]["eligible_hosts"] =
            serde_json::json!([host_id]);
        for pointer in [
            "/configuration/primary/access/grants",
            "/configuration/binding/access/grants",
        ] {
            let grants = binding
                .pointer_mut(pointer)
                .and_then(|g| g.as_array().cloned())
                .ok_or(WorkflowError::UnexpectedBody)?;
            let mut grants = grants;
            grants.push(serde_json::json!("mutate_stream"));
            grants.push(serde_json::json!("use_credential"));
            *binding.pointer_mut(pointer).unwrap() = serde_json::json!(grants);
        }
        for pointer in [
            "/configuration/binding/required_tools",
            "/configuration/primary/tools",
        ] {
            let tools = binding
                .pointer_mut(pointer)
                .and_then(|g| g.as_array().cloned())
                .ok_or(WorkflowError::UnexpectedBody)?;
            let mut tools = tools;
            tools.push(serde_json::json!("shell"));
            *binding.pointer_mut(pointer).unwrap() = serde_json::json!(tools);
        }
        Ok(binding)
    }

    fn call(
        &mut self,
        command_id: &str,
        operation: serde_json::Value,
        project: Option<&symbiote_domain::ProjectId>,
    ) -> Result<ResponseBody, WorkflowError> {
        self.driver.call(command_id, operation, project)
    }
}
