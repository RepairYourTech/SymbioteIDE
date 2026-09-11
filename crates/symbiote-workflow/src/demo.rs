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

/// The labeled fixture transport's report: deliberately HOSTILE markup
/// (a script that would append an observable marker, an onerror image,
/// bold tags) so every rendering surface — the desktop UI log, the
/// Preview window, journal evidence — permanently proves that worker
/// content renders as inert text. If the script ever EXECUTES anywhere,
/// it appends the observable marker WORKER_SCRIPT_EXECUTED.
pub const FIXTURE_REPORT: &str = "implemented the bounded change; produced.txt written by the sandboxed tool <script>document.body.appendChild(document.createTextNode(\"WORKER_SCRIPT_EXECUTED\"))</script><img src=x onerror=\"document.title='WORKER_IMG_ONERROR'\"><b>bold markup</b>";

/// One project's full canonical identity for the demo flow. Every id is
/// project-scoped at the daemon, so two lanes may reuse the same role,
/// provider, and model NAMES while remaining entirely separate
/// configuration; what must differ (project, root, objective, task,
/// stream, binding, credential reference) is explicit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DemoLane {
    pub project: &'static str,
    pub root: &'static str,
    pub objective: &'static str,
    pub task: &'static str,
    pub stream: &'static str,
    pub chat: &'static str,
    /// The lane's lead Role id and executor Role id. Role ids (and Root
    /// ids) are GLOBALLY unique across the daemon — two projects cannot
    /// reuse a role name — so each lane names its own.
    pub lead_role: &'static str,
    pub role: &'static str,
    pub binding_id: &'static str,
    /// The lane's provider connection and model descriptor ids. These
    /// records are HOST-GLOBAL (not project-scoped), so each lane names
    /// its own.
    pub provider: &'static str,
    pub model: &'static str,
    /// The credential reference this lane's profile names. It must be
    /// registered with the operator's broker FOR THIS PROJECT — a
    /// reference registered for another project is refused by the broker
    /// (the Project-isolation demo pins exactly that refusal).
    pub credential: &'static str,
    /// Whether the project's Team carries the external-harness lane
    /// (registered only for the staffing project, whose Team the
    /// two-harness demo extends with a third member).
    pub with_external_lane: bool,
}

/// The default lane: the checked-in fixtures' identities.
pub const STAFFING: DemoLane = DemoLane {
    project: PROJECT,
    root: ROOT,
    objective: "staffing-objective",
    task: TASK,
    stream: STREAM,
    chat: "chat",
    lead_role: "lead",
    role: "engineer",
    binding_id: "engineer-binding",
    provider: "native-openai",
    model: "coding-model",
    credential: "native-vault-ref",
    with_external_lane: true,
};

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
    /// The open repository's path (registered as the Root placement) for
    /// the default lane.
    repository: PathBuf,
    /// Additional projects' open repositories, registered with
    /// [`Self::with_repository`] — one daemon can host several projects,
    /// each rooted at its own repository.
    repositories: std::collections::BTreeMap<String, PathBuf>,
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
            repositories: std::collections::BTreeMap::new(),
        })
    }

    /// Registers an additional project's open repository — the
    /// Project-isolation demo opens two repositories on one daemon.
    pub fn with_repository(mut self, project: &str, repository: &Path) -> Self {
        self.repositories
            .insert(project.to_owned(), repository.to_path_buf());
        self
    }

    fn repository_for(&self, lane: &DemoLane) -> PathBuf {
        self.repositories
            .get(lane.project)
            .cloned()
            .unwrap_or_else(|| self.repository.clone())
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

    /// The first half of the sequence for the default lane: open the
    /// repository, describe the task, route, prepare, and START the
    /// dispatch.
    pub fn start_demo(&mut self) -> Result<String, WorkflowError> {
        self.start_demo_lane(&STAFFING)
    }

    /// The same first half, driven for an explicit project lane: every
    /// canonical identity comes from the lane, so ONE daemon can host
    /// several fully isolated projects. Everything here is journaled
    /// canonical state — it survives a daemon SIGKILL, which is exactly
    /// what the restart/resume proof does between the halves.
    pub fn start_demo_lane(&mut self, lane: &DemoLane) -> Result<String, WorkflowError> {
        let project = symbiote_domain::ProjectId::new(lane.project).expect("lane project");
        let repository = self.repository_for(lane);
        let mut git = symbiote_repo::SystemGit::new();
        let head = symbiote_repo::observe_head(&mut git, &repository)
            .map_err(|_| WorkflowError::LocalObservation)?;
        let base = head.commit.clone();
        let target = "b".repeat(40);
        let host_id = self.host_pulse()?;
        // 1. Open the repository: the real HEAD becomes the stream base.
        // The register fixture carries an empty host_paths map — client
        // declared placements are refused — and the placement is observed
        // by the Host right after registration. The fixture's project
        // identities are rewritten for the lane.
        let mut register = Self::register_operation();
        register["project"]["id"] = serde_json::json!(lane.project);
        register["project"]["name"] = serde_json::json!(format!("Demo {}", lane.project));
        register["project"]["lead"] = serde_json::json!(lane.lead_role);
        register["project"]["roots"][0]["id"] = serde_json::json!(lane.root);
        register["project"]["roots"][0]["project_id"] = serde_json::json!(lane.project);
        let roles = register["project"]["roles"]
            .as_array_mut()
            .ok_or(WorkflowError::UnexpectedBody)?;
        roles[0]["id"] = serde_json::json!(lane.lead_role);
        roles[0]["name"] = serde_json::json!(lane.lead_role);
        roles[1]["id"] = serde_json::json!(lane.role);
        roles[1]["name"] = serde_json::json!(lane.role);
        for role in roles.iter_mut() {
            role["project_id"] = serde_json::json!(lane.project);
        }
        if lane.with_external_lane {
            register["project"]["roles"]
                .as_array_mut()
                .ok_or(WorkflowError::UnexpectedBody)?
                .push(serde_json::json!({
                    "id": EXTERNAL_ROLE,
                    "project_id": lane.project,
                    "revision": 0,
                    "name": EXTERNAL_ROLE,
                    "operating_contract": {"id": "engineer-external-contract", "revision": 1}
                }));
        }
        self.call(
            &format!("wf-register-{}", lane.project),
            register,
            Some(&project),
        )?;
        self.call(
            &format!("wf-place-{}", lane.project),
            serde_json::json!({"kind":"observe_root_placement","project_id":lane.project,
            "root_id":lane.root,"host_id":host_id,
            "path":repository.display().to_string(),"expected_revision":0}),
            Some(&project),
        )?;
        // 2. Team: rewrite the fixture's project identities for the lane;
        // the engineer may mutate streams (the demo's worker writes in the
        // reserved worktree) and use its OWN project's credentials.
        let mut team = Self::team_operation();
        team["team"]["project_id"] = serde_json::json!(lane.project);
        team["team"]["lead_role_id"] = serde_json::json!(lane.lead_role);
        team["team"]["access_ceiling"]["project_id"] = serde_json::json!(lane.project);
        team["team"]["access_ceiling"]["roots"][0] = serde_json::json!(lane.root);
        let members = team["team"]["members"]
            .as_array_mut()
            .ok_or(WorkflowError::UnexpectedBody)?;
        members[0]["role_id"] = serde_json::json!(lane.lead_role);
        members[0]["independent_reviewers"][0] = serde_json::json!(lane.role);
        members[1]["role_id"] = serde_json::json!(lane.role);
        members[1]["independent_reviewers"][0] = serde_json::json!(lane.lead_role);
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
        for member in team["team"]["members"]
            .as_array_mut()
            .ok_or(WorkflowError::UnexpectedBody)?
            .iter_mut()
        {
            member["access"]["project_id"] = serde_json::json!(lane.project);
            member["access"]["roots"][0] = serde_json::json!(lane.root);
        }
        if lane.with_external_lane {
            team["team"]["members"]
                .as_array_mut()
                .ok_or(WorkflowError::UnexpectedBody)?
                .push(serde_json::json!({
                    "role_id": EXTERNAL_ROLE,
                    "function": "general_execution",
                    "responsibilities": ["Implement bounded coding tasks"],
                    "task_domains": ["coding"],
                    "access": {"project_id": lane.project, "roots": [lane.root],
                        "grants": ["read_root", "execute_process", "mutate_stream"],
                        "policy_revision": 1},
                    "context_policy_ref": "default-context",
                    "tool_policy_ref": "default-tools",
                    "skill_policy_ref": "default-skills",
                    "execution_policy_ref": "bounded-execution",
                    "independent_reviewers": [lane.lead_role],
                    "fallbacks": []
                }));
        }
        self.call(&format!("wf-team-{}", lane.project), team, Some(&project))?;
        // 3. Binding: this Host eligible, stream mutation + credential use
        // granted, the `shell` tool declared (primary.tools must equal
        // binding.required_tools — the workforce validator's rule).
        let binding = self.binding_operation_for(lane, &host_id)?;
        self.call(
            &format!("wf-binding-{}", lane.binding_id),
            binding,
            Some(&project),
        )?;
        // 4. Provider connection and model descriptor — HOST-GLOBAL
        // records (keyed by id alone, not per project), so each lane names
        // its own; reusing another lane's id would overwrite its record.
        // The fixture transport ignores the endpoint — nothing contacts
        // it.
        self.call(
            &format!("wf-provider-{}", lane.project),
            serde_json::json!({"kind":"replace_provider_connection","attribution":lane.project,
            "connection":{"id":lane.provider,"adapter":"openai-responses",
            "endpoint_reference":"https://api.openai.example/v1","authentication":"api_credential"}}),
            Some(&project),
        )?;
        self.call(
            &format!("wf-model-{}", lane.project),
            serde_json::json!({"kind":"replace_model_descriptor","attribution":lane.project,
            "descriptor":{"schema_version":1,"id":lane.model,"provider_id":lane.provider,
            "context_window_tokens":16384,"max_output_tokens":4096,
            "capabilities":{"reasoning_efforts":[],"tools":true,"images":false,"streaming":false}}}),
            Some(&project),
        )?;
        self.start_lane_tail(lane, &host_id, &base, &target)
    }

    /// The per-lane tail — binding (optional), objective, task, route,
    /// prepare, and START — for a lane whose project is already
    /// registered (its project-level composition exists). A lane carries
    /// its OWN binding and credential reference; pass `with_binding =
    /// false` to reuse the role's CURRENT binding (e.g. after the
    /// operator replaced it to fix a credential reference).
    pub fn start_followon_lane(
        &mut self,
        lane: &DemoLane,
        with_binding: bool,
    ) -> Result<String, WorkflowError> {
        let host_id = self.host_pulse()?;
        if with_binding {
            let binding = self.binding_operation_for(lane, &host_id)?;
            let project = symbiote_domain::ProjectId::new(lane.project).expect("lane project");
            self.call(
                &format!("wf-binding-{}", lane.binding_id),
                binding,
                Some(&project),
            )?;
        }
        let repository = self.repository_for(lane);
        let mut git = symbiote_repo::SystemGit::new();
        let head = symbiote_repo::observe_head(&mut git, &repository)
            .map_err(|_| WorkflowError::LocalObservation)?;
        let base = head.commit.clone();
        let target = "b".repeat(40);
        self.start_lane_tail(lane, &host_id, &base, &target)
    }

    /// The operator's correction flow: read the lane's binding, then
    /// replace it under an exact revision CAS with a corrected credential
    /// reference. The dispatch contracts compiled BEFORE the replacement
    /// keep the old snapshot — a running dispatch is not silently
    /// rewired; the next start compiles the corrected binding.
    pub fn replace_lane_binding(
        &mut self,
        lane: &DemoLane,
        credential: &str,
    ) -> Result<(), WorkflowError> {
        let project = symbiote_domain::ProjectId::new(lane.project).expect("lane project");
        let host_id = self.host_pulse()?;
        let read = self.call(
            &format!("wf-binding-read-{}", lane.binding_id),
            serde_json::json!({"kind":"get_binding","project_id":lane.project,
            "binding_id":lane.binding_id}),
            Some(&project),
        )?;
        let current_revision = match &read {
            ResponseBody::Binding(configuration) => configuration.binding.revision,
            _ => return Err(WorkflowError::UnexpectedBody),
        };
        let mut binding = self.binding_operation_for(lane, &host_id)?;
        binding["expected_revision"] = serde_json::json!(current_revision.0);
        binding["configuration"]["binding"]["revision"] = serde_json::json!(current_revision.0 + 1);
        binding["configuration"]["primary"]["profile"]["credential"] =
            serde_json::json!(credential);
        self.call(
            &format!(
                "wf-binding-replace-{}-{}",
                lane.binding_id,
                current_revision.0 + 1
            ),
            binding,
            Some(&project),
        )?;
        Ok(())
    }

    fn start_lane_tail(
        &mut self,
        lane: &DemoLane,
        host_id: &str,
        base: &str,
        target: &str,
    ) -> Result<String, WorkflowError> {
        let project = symbiote_domain::ProjectId::new(lane.project).expect("lane project");
        // 5. Describe the coding task: a classified objective the task
        // hangs off, with the requirements the run's context will carry.
        // The stream's worktree identity and branch are the seed-derived
        // ones provisioning verifies — the driver derives them exactly
        // like the Host does, from the stream's recorded identity.
        self.call(
            &format!("wf-objective-{}", lane.objective),
            serde_json::json!({"kind":"create_work","work":{"id":{"kind":"objective","id":lane.objective},
            "project_id":lane.project,"role_id":lane.role,"title":"Implement a bounded change",
            "description":"Implement the bounded change described by this objective.",
            "utterance":null,"objective_class":"maintenance","parent":null,"dependencies":[],
            "requirements":["the produced file exists"],"constraints":[],"risks":[],
            "acceptance":["produced file content is the worker's"],"priority":2,
            "budget":null,"external_references":[]}}),
            Some(&project),
        )?;
        let stream_id = symbiote_domain::ChangeStreamId::new(lane.stream).expect("lane stream");
        let root_id = symbiote_domain::RootId::new(lane.root).expect("lane root");
        let derived = derive_stream_worktree(&project, &root_id, &stream_id)?;
        self.call(
            &format!("wf-task-{}", lane.task),
            serde_json::json!({"kind":"create_task","task":{"id":lane.task,"project_id":lane.project,
            "root_id":lane.root,"role_id":lane.role,
            "origin":{"kind":"objective","work":{"project_id":lane.project,
                "id":{"kind":"objective","id":lane.objective}}},
            "task_contract":{"id":"coding-contract","revision":1},
            "stream":{"id":lane.stream,"originating_chat":lane.chat,
            "worktree":derived.worktree_id.as_str(),
            "branch":derived.branch,"base":base,
            "target":target}}}),
            Some(&project),
        )?;
        // 6. Route, prepare, start.
        self.call(
            &format!("wf-route-{}", lane.objective),
            serde_json::json!({"kind":"record_route",
            "request":{"project_id":lane.project,"work_id":{"kind":"objective","id":lane.objective},
            "requested":lane.role,"domains":[]}}),
            Some(&project),
        )?;
        self.call(
            &format!("wf-prepare-{}", lane.task),
            serde_json::json!({"kind":"prepare_dispatch","task_id":lane.task}),
            Some(&project),
        )?;
        let started = self.call(
            &format!("wf-start-{}", lane.task),
            serde_json::json!({"kind":"start_prepared_task","task_id":lane.task,"host_id":host_id}),
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
        let project = symbiote_domain::ProjectId::new(PROJECT).expect("fixture project");
        self.journal_to_head(&project).map(|(cursor, _)| cursor)
    }

    /// Follows the evidence trail for an explicit lane's project — each
    /// project's journal is read and tracked independently.
    pub fn read_journal_lane(&mut self, lane: &DemoLane) -> Result<u64, WorkflowError> {
        let project = symbiote_domain::ProjectId::new(lane.project).expect("lane project");
        self.journal_to_head(&project).map(|(cursor, _)| cursor)
    }

    /// The second half for the default lane: execute the started dispatch
    /// (real provisioning, real sandboxed tool execution, fixture model
    /// turn), then read the completion evidence, the journal trail, and
    /// the worktree diff.
    pub fn finish_demo(&mut self, dispatch_id: &str) -> Result<DemoOutcome, WorkflowError> {
        self.finish_demo_lane(&STAFFING, dispatch_id)
    }

    /// The second half for an explicit project lane. `dispatch_id` comes
    /// from [`Self::start_demo_lane`] — possibly across a daemon restart.
    pub fn finish_demo_lane(
        &mut self,
        lane: &DemoLane,
        dispatch_id: &str,
    ) -> Result<DemoOutcome, WorkflowError> {
        let project = symbiote_domain::ProjectId::new(lane.project).expect("lane project");
        let run = self.call(
            &format!("wf-run-{}", lane.task),
            serde_json::json!({"kind":"run_started_dispatch","task_id":lane.task,
            "dispatch_id":dispatch_id}),
            Some(&project),
        )?;
        let _completed = match &run {
            ResponseBody::WorkerRun(run) => {
                // The lane is native: the runtime comes from the dispatch
                // contract, never from the caller.
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
            &format!("wf-task-read-{}", lane.task),
            serde_json::json!({"kind":"get_task","project_id":lane.project,"task_id":lane.task}),
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
        // The filter is per task: another project's (or lane's) completion
        // must never surface here.
        let mut report = None;
        let (_cursor, events) = self.journal_to_head(&project)?;
        for event in &events {
            if let symbiote_protocol::EventPayload::TaskChanged {
                task_id, command, ..
            } = &event.payload
            {
                if task_id.as_str() != lane.task {
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
            &symbiote_domain::ProjectId::new(lane.project).expect("lane project"),
            &symbiote_domain::RootId::new(lane.root).expect("lane root"),
            &symbiote_domain::ChangeStreamId::new(lane.stream).expect("lane stream"),
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
        let (_cursor, events) = self.journal_to_head(&project)?;
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
        project: &symbiote_domain::ProjectId,
    ) -> Result<(u64, Vec<symbiote_protocol::JournalEvent>), WorkflowError> {
        let mut cursor = self.driver.journal_position(project).0;
        let mut events = Vec::new();
        loop {
            let page = self.call(
                &format!("wf-journal-{}-{cursor}", project.as_str()),
                serde_json::json!({"kind":"read_journal","project_id":project.as_str(),
                "after":cursor,"limit":100}),
                Some(project),
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

    /// The checked-in binding fixture, patched for a lane: the lane's
    /// binding id and credential reference, this Host eligible, stream
    /// mutation + credential use granted, and the `shell` tool declared
    /// (both sides of the validator's equality rule).
    fn binding_operation_for(
        &self,
        lane: &DemoLane,
        host_id: &str,
    ) -> Result<serde_json::Value, WorkflowError> {
        let mut binding = Self::binding_operation_fixture();
        binding["configuration"]["binding"]["id"] = serde_json::json!(lane.binding_id);
        binding["configuration"]["binding"]["role_id"] = serde_json::json!(lane.role);
        binding["configuration"]["binding"]["project_id"] = serde_json::json!(lane.project);
        for pointer in [
            "/configuration/binding/access",
            "/configuration/primary/access",
        ] {
            *binding
                .pointer_mut(&format!("{pointer}/project_id"))
                .ok_or(WorkflowError::UnexpectedBody)? = serde_json::json!(lane.project);
            *binding
                .pointer_mut(&format!("{pointer}/roots/0"))
                .ok_or(WorkflowError::UnexpectedBody)? = serde_json::json!(lane.root);
        }
        binding["configuration"]["primary"]["profile"]["provider"] =
            serde_json::json!(lane.provider);
        binding["configuration"]["primary"]["profile"]["model"] = serde_json::json!(lane.model);
        binding["configuration"]["primary"]["profile"]["credential"] =
            serde_json::json!(lane.credential);
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
