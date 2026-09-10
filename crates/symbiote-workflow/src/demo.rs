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

/// The second staffing lane (#269 two-harness demo): an external-harness
/// engineer on the SAME project — its own role, objective, task, stream,
/// and binding. Nothing is shared with the native lane except the
/// project, the root, the provider facts, and the daemon itself; the
/// external profile's credential reference is its own and is never
/// registered with the operator's broker.
pub const EXTERNAL_TASK: &str = "external-task";
pub const EXTERNAL_STREAM: &str = "external-stream";
pub const EXTERNAL_ROLE: &str = "engineer-external";
pub const EXTERNAL_OBJECTIVE: &str = "external-objective";

/// What the first-release sequence observed, for the caller (desktop UI,
/// test, operator) to inspect.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// The seed-derived worktree identity for a stream — the exact derivation
/// provisioning verifies (the Host computes the same value from the
/// stream's recorded identity). Shared by both demo lanes and kept in one
/// place so the derivation cannot diverge from the evidence reader.
fn derive_stream_worktree(
    project: &symbiote_domain::ProjectId,
    root: &symbiote_domain::RootId,
    stream_id: &symbiote_domain::ChangeStreamId,
) -> Result<symbiote_worktrees::Derived, WorkflowError> {
    let digest = symbiote_trust::Fingerprint::of(stream_id.as_str().as_bytes());
    let seed = symbiote_worktrees::policy_seed(digest.as_str())
        .map_err(|_| WorkflowError::LocalObservation)?;
    symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
        project_id: project,
        root_id: root,
        stream_id,
        seed,
    })
    .map_err(|_| WorkflowError::LocalObservation)
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

    /// The positions to persist across a desktop restart; restore with
    /// [`Self::with_positions`].
    pub fn positions(&self) -> Vec<symbiote_client_sdk::JournalPosition> {
        self.driver.positions()
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
            .map_err(|_| WorkflowError::LocalObservation)?;
        let base = head.commit.clone();
        let target = "b".repeat(40);
        let host_id = self.host_pulse()?;
        // The team gains the second staffing lane: a third role in the
        // registration and an external-harness engineer member whose
        // grants carry stream mutation but NOT UseCredential — harness
        // credentials stay harness-owned, and the operator's broker has
        // no registration for the external profile's reference.
        let mut register = Self::register_operation();
        let roles = register["project"]["roles"]
            .as_array_mut()
            .ok_or(WorkflowError::UnexpectedBody)?;
        roles.push(serde_json::json!({
            "id": EXTERNAL_ROLE,
            "project_id": PROJECT,
            "revision": 0,
            "name": EXTERNAL_ROLE,
            "operating_contract": {"id": "engineer-external-contract", "revision": 1}
        }));
        self.call("wf-register", register, Some(&project))?;
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
        team["team"]["members"]
            .as_array_mut()
            .ok_or(WorkflowError::UnexpectedBody)?
            .push(serde_json::json!({
                "role_id": EXTERNAL_ROLE,
                "function": "general_execution",
                "responsibilities": ["Implement bounded coding tasks"],
                "task_domains": ["coding"],
                "access": {"project_id": PROJECT, "roots": [ROOT],
                    "grants": ["read_root", "execute_process", "mutate_stream"],
                    "policy_revision": 1},
                "context_policy_ref": "default-context",
                "tool_policy_ref": "default-tools",
                "skill_policy_ref": "default-skills",
                "execution_policy_ref": "bounded-execution",
                "independent_reviewers": ["lead"],
                "fallbacks": []
            }));
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
        let derived = derive_stream_worktree(&project, &root_id, &stream_id)?;
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
            ResponseBody::WorkerRun(run) => {
                // The lane this method drives is the native one: the
                // runtime comes from the dispatch contract, never from
                // the caller.
                if run.runtime != symbiote_domain::RuntimeKind::NativeSymbiote {
                    return Err(WorkflowError::UnexpectedBody);
                }
                run.completed
            }
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
            if let symbiote_protocol::EventPayload::TaskChanged {
                task_id, command, ..
            } = &event.payload
            {
                if task_id.as_str() != TASK {
                    continue;
                }
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

    /// The external lane of the two-harness demo (#269): on the SAME
    /// project, describe a second task for the external-harness engineer —
    /// its own objective, its own seed-derived stream, its own binding
    /// (profile runtime EXTERNAL_HARNESS with the installation pinned, and
    /// a credential reference that is the harness's own) — route, prepare,
    /// and START it. Call after [`Self::start_demo`], possibly across a
    /// daemon restart. Nothing is shared with the native lane but the
    /// project, root, provider facts, and the daemon.
    pub fn start_demo_external(&mut self) -> Result<String, WorkflowError> {
        let project = symbiote_domain::ProjectId::new(PROJECT).expect("fixture project");
        let host_id = self.host_pulse()?;
        // Binding: the external profile pins its installation (the
        // validator's rule for harness runtimes) and carries stream
        // mutation but NOT UseCredential — the fixture credential
        // reference it names is the harness's own and is never registered
        // with the operator's broker. No `shell` tool: the external
        // harness has no Host-consented tools.
        let mut binding = Self::binding_operation_fixture();
        binding["configuration"]["binding"]["id"] = serde_json::json!("engineer-external-binding");
        binding["configuration"]["binding"]["role_id"] = serde_json::json!(EXTERNAL_ROLE);
        binding["configuration"]["binding"]["profile_id"] = serde_json::json!("external-codex");
        let profile = &mut binding["configuration"]["primary"]["profile"];
        profile["id"] = serde_json::json!("external-codex");
        profile["runtime"] = serde_json::json!("EXTERNAL_HARNESS");
        profile["installation"] = serde_json::json!("codex-0-118-0");
        profile["adapter"] = serde_json::json!("codex-app-server");
        profile["credential"] = serde_json::json!("external-harness-ref");
        profile["billing_entitlement"] = serde_json::json!("external-codex-entitlement");
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
            *binding.pointer_mut(pointer).unwrap() = serde_json::json!(grants);
        }
        self.call("wf-binding-external", binding, Some(&project))?;
        // The second objective and task: same open repository, its own
        // stream identity and seed-derived worktree.
        self.call(
            "wf-objective-external",
            serde_json::json!({"kind":"create_work","work":{"id":{"kind":"objective","id":EXTERNAL_OBJECTIVE},
            "project_id":PROJECT,"role_id":EXTERNAL_ROLE,
            "title":"Implement a bounded change with the harness",
            "description":"Implement the bounded change described by this objective.",
            "utterance":null,"objective_class":"maintenance","parent":null,"dependencies":[],
            "requirements":["the produced file exists"],"constraints":[],"risks":[],
            "acceptance":["produced file content is the worker's"],"priority":2,
            "budget":null,"external_references":[]}}),
            Some(&project),
        )?;
        let mut git = symbiote_repo::SystemGit::new();
        let head = symbiote_repo::observe_head(&mut git, &self.repository)
            .map_err(|_| WorkflowError::LocalObservation)?;
        let base = head.commit.clone();
        let target = "b".repeat(40);
        let stream_id = symbiote_domain::ChangeStreamId::new(EXTERNAL_STREAM).expect("stream");
        let root_id = symbiote_domain::RootId::new(ROOT).expect("fixture root");
        let derived = derive_stream_worktree(&project, &root_id, &stream_id)?;
        self.call(
            "wf-task-external",
            serde_json::json!({"kind":"create_task","task":{"id":EXTERNAL_TASK,"project_id":PROJECT,
            "root_id":ROOT,"role_id":EXTERNAL_ROLE,
            "origin":{"kind":"objective","work":{"project_id":PROJECT,
                "id":{"kind":"objective","id":EXTERNAL_OBJECTIVE}}},
            "task_contract":{"id":"coding-contract","revision":1},
            "stream":{"id":EXTERNAL_STREAM,"originating_chat":"chat-external",
            "worktree":derived.worktree_id.as_str(),
            "branch":derived.branch,"base":base,
            "target":target}}}),
            Some(&project),
        )?;
        self.call(
            "wf-route-external",
            serde_json::json!({"kind":"record_route",
            "request":{"project_id":PROJECT,"work_id":{"kind":"objective","id":EXTERNAL_OBJECTIVE},
            "requested":EXTERNAL_ROLE,"domains":[]}}),
            Some(&project),
        )?;
        let prepared = self.call(
            "wf-prepare-external",
            serde_json::json!({"kind":"prepare_dispatch","task_id":EXTERNAL_TASK}),
            Some(&project),
        )?;
        let ResponseBody::DispatchPreparation(preparation) = &prepared else {
            return Err(WorkflowError::UnexpectedBody);
        };
        if preparation.outcome != symbiote_domain::PreparationOutcome::Ready {
            // The recorded refusal is durable evidence; the caller reads it
            // through `get_dispatch_preparation`.
            return Err(WorkflowError::UnexpectedBody);
        }
        let started = self.call(
            "wf-start-external",
            serde_json::json!({"kind":"start_prepared_task","task_id":EXTERNAL_TASK,"host_id":host_id}),
            Some(&project),
        )?;
        match started {
            ResponseBody::StartedDispatch(started) => Ok(started.dispatch_id.to_string()),
            _ => Err(WorkflowError::UnexpectedBody),
        }
    }

    /// Runs the EXTERNAL lane's started dispatch (real provisioning, the
    /// operator's explicitly labeled fixture harness conversation — no
    /// binary, no model turn, no spending), then reads the completion
    /// evidence and the worktree state. The harness runtime comes from the
    /// dispatch contract; a `native_symbiote` runtime here would mean the
    /// lanes crossed.
    pub fn finish_demo_external(
        &mut self,
        dispatch_id: &str,
    ) -> Result<DemoOutcome, WorkflowError> {
        let project = symbiote_domain::ProjectId::new(PROJECT).expect("fixture project");
        let run = self.call(
            "wf-run-external",
            serde_json::json!({"kind":"run_started_dispatch","task_id":EXTERNAL_TASK,
            "dispatch_id":dispatch_id}),
            Some(&project),
        )?;
        let _completed = match &run {
            ResponseBody::WorkerRun(run) => {
                if run.runtime != symbiote_domain::RuntimeKind::ExternalHarness {
                    return Err(WorkflowError::UnexpectedBody);
                }
                run.completed
            }
            _ => return Err(WorkflowError::UnexpectedBody),
        };
        let task = self.call(
            "wf-task-read-external",
            serde_json::json!({"kind":"get_task","project_id":PROJECT,"task_id":EXTERNAL_TASK}),
            Some(&project),
        )?;
        let task_state = match &task {
            ResponseBody::Task(task) => serde_json::to_value(task.state())
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .ok_or(WorkflowError::UnexpectedBody)?,
            _ => return Err(WorkflowError::UnexpectedBody),
        };
        let mut report = None;
        let (_cursor, events) = self.journal_to_head()?;
        for event in &events {
            if let symbiote_protocol::EventPayload::TaskChanged {
                task_id, command, ..
            } = &event.payload
            {
                if task_id.as_str() != EXTERNAL_TASK {
                    continue;
                }
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
        let worktree = crate::observe_worktree_evidence(
            &self.reservation_base,
            &symbiote_domain::ProjectId::new(PROJECT).expect("fixture project"),
            &symbiote_domain::RootId::new(ROOT).expect("fixture root"),
            &symbiote_domain::ChangeStreamId::new(EXTERNAL_STREAM).expect("fixture stream"),
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

    /// Graceful shutdown: the daemon exits cleanly (the journaled state
    /// is the same state a crash would have preserved).
    pub fn shutdown(&mut self) -> Result<(), WorkflowError> {
        let project = symbiote_domain::ProjectId::new(PROJECT).expect("fixture project");
        self.call(
            "wf-shutdown",
            serde_json::json!({"kind":"shutdown"}),
            Some(&project),
        )?;
        Ok(())
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
