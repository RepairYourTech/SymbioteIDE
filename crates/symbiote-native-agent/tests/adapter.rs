//! The native lane's adapter: a real `AgentRuntimeAdapter` that passes the
//! suite, a mock harness it drives, and every refusal on the session boundary.
//!
//! The harness here is a mock on purpose and says so — it records what it was
//! given and reports the surfaces it was handed, which is what a real process
//! does with the projection it receives. What is under test is the adapter: the
//! permit recheck, the session binding, the event binding, the lifecycle, and
//! the refusals when any of those is violated.
use std::collections::{BTreeMap, BTreeSet};

use symbiote_domain::*;
use symbiote_native_agent::adapter::{
    HarnessProcess, HarnessSession, LaunchRequest, NativeAgentAdapter,
};
use symbiote_runtime_sdk::conformance::run_conformance;
use symbiote_runtime_sdk::dossier::{CompatibilityDossier, PackRun};
use symbiote_runtime_sdk::events::{RuntimeEvent, SessionBinding};
use symbiote_runtime_sdk::projection::{
    Agreement, ContractProjection, RuntimeHandshake, SurfaceReport,
};
use symbiote_runtime_sdk::{
    ActivationJournal, ActivationReceipt, AdapterError, AdapterOperation, AgentRuntimeAdapter,
    Capability, ControlSupport, DeclaredControl, DeclaredRuntime, LaunchPermit, PreparedLaunch,
    ProbeEvidence, RuntimeContextLimits, RuntimeDescriptor, RuntimeOwner, SessionControl,
    Transport, TurnInput,
};

const MECHANISM: &str = "the native loop reads the contract and files the report";

fn profile() -> RuntimeProfile {
    RuntimeProfile {
        id: RuntimeProfileId::new("native-worker").unwrap(),
        revision: Revision(1),
        runtime: RuntimeKind::NativeSymbiote,
        adapter: AgentRuntimeAdapterId::new("native-agent").unwrap(),
        installation: None,
        provider: ProviderConnectionId::new("native-openai").unwrap(),
        credential: CredentialReferenceId::new("native-credential").unwrap(),
        billing_entitlement: BillingEntitlementId::new("native-billing").unwrap(),
        model: ModelId::new("coding-model").unwrap(),
        eligible_hosts: [HostId::new("host-a").unwrap()].into(),
    }
}

fn evidence(descriptor: &RuntimeDescriptor) -> ProbeEvidence {
    ProbeEvidence {
        artifact: EvidenceId::new("native-proof").unwrap(),
        adapter_id: descriptor.adapter_id.clone(),
        installation: descriptor.installation.clone(),
        profile_id: descriptor.profile_id.clone(),
        profile_revision: descriptor.profile_revision,
        model_id: descriptor.model_id.clone(),
        host_id: descriptor.host_id.clone(),
        adapter_version: descriptor.adapter_version.clone(),
        upstream_version: descriptor.upstream_version.clone(),
        platform: descriptor.platform.clone(),
        observed_at: Timestamp(1),
        expires_at: Timestamp(1_000_000),
    }
}

/// The native runtime a Host would declare: the identity the operator asserts
/// and the capabilities and controls it asserts, with nothing about who observed
/// them. This is the shape `hold` is later given.
fn descriptor() -> RuntimeDescriptor {
    let mut descriptor = RuntimeDescriptor {
        sdk_version: symbiote_runtime_sdk::SDK_VERSION,
        adapter_id: profile().adapter,
        installation: None,
        profile_id: profile().id,
        profile_revision: profile().revision,
        model_id: profile().model,
        adapter_version: "0.1.0".into(),
        upstream_version: "0.1.0".into(),
        runtime: RuntimeKind::NativeSymbiote,
        owner: RuntimeOwner::SymbioteNative {},
        transport: Transport::NativeLoop,
        tier: symbiote_runtime_sdk::IntegrationTier::Detected,
        host_id: HostId::new("host-a").unwrap(),
        platform: "linux".into(),
        capabilities: BTreeMap::new(),
        controls: BTreeMap::new(),
        tools: ["read_file".into()].into(),
        skills: ["review".into()].into(),
        context_limits: Some(RuntimeContextLimits {
            context_window_tokens: 128_000,
            max_output_tokens: 16_384,
        }),
    };
    let proof = evidence(&descriptor);
    descriptor.capabilities = [
        Capability::StructuredMessages,
        Capability::ContextLimits,
        Capability::Extensions,
        Capability::Hooks,
        Capability::Tools,
        Capability::Skills,
        Capability::Instructions,
        Capability::ProfileIsolation,
    ]
    .into_iter()
    .map(|capability| {
        (
            capability,
            symbiote_runtime_sdk::Support::Supported {
                evidence: Box::new(proof.clone()),
            },
        )
    })
    .collect();
    descriptor.controls = [
        Control::Filesystem,
        Control::Process,
        Control::Network,
        Control::Credentials,
        Control::Cancellation,
        Control::CompletionAuthority,
    ]
    .into_iter()
    .map(|control| {
        (
            control,
            ControlSupport {
                strength: EnforcementStrength::HostEnforced,
                mechanism: MECHANISM.into(),
                evidence: proof.clone(),
            },
        )
    })
    .collect();
    descriptor
}

/// The operator's declaration of the same runtime, which is what a reader holds
/// a published record against.
fn declared() -> DeclaredRuntime {
    let descriptor = descriptor();
    DeclaredRuntime {
        adapter_id: descriptor.adapter_id,
        installation: descriptor.installation,
        profile_id: descriptor.profile_id,
        profile_revision: descriptor.profile_revision,
        model_id: descriptor.model_id,
        adapter_version: descriptor.adapter_version,
        upstream_version: descriptor.upstream_version,
        runtime: descriptor.runtime,
        owner: descriptor.owner,
        transport: descriptor.transport,
        tier: descriptor.tier,
        platform: descriptor.platform,
        capabilities: BTreeSet::new(),
        controls: BTreeMap::from([(
            Control::CompletionAuthority,
            DeclaredControl {
                strength: EnforcementStrength::HostEnforced,
                mechanism: MECHANISM.into(),
            },
        )]),
        tools: descriptor.tools,
        skills: descriptor.skills,
        context_limits: descriptor.context_limits,
    }
}

/// A real dispatch: the canonical Task, Role, binding, profile and Host a
/// native run would be staffed from. The suite is run over this, not over
/// something a fixture invented, so a dossier it publishes is about work.
fn dispatch(descriptor: &RuntimeDescriptor) -> Dispatch {
    let project = ProjectId::new("native-project").unwrap();
    let root = RootId::new("native-root").unwrap();
    let task = Task::new(
        TaskId::new("native-task").unwrap(),
        project.clone(),
        root.clone(),
        RoleId::new("native-engineer").unwrap(),
        ChangeStreamId::new("native-stream").unwrap(),
        VersionedTaskContract {
            id: TaskContractId::new("native-task-contract").unwrap(),
            revision: Revision(7),
        },
    );
    let role = Role {
        id: RoleId::new("native-engineer").unwrap(),
        project_id: project.clone(),
        revision: Revision(1),
        name: "Native engineer".into(),
        operating_contract: VersionedRoleContract {
            id: RoleContractId::new("native-role-contract").unwrap(),
            revision: Revision(3),
        },
    };
    let profile = profile();
    let binding = WorkforceBinding {
        id: BindingId::new("native-binding").unwrap(),
        revision: Revision(2),
        project_id: project.clone(),
        role_id: role.id.clone(),
        profile_id: profile.id.clone(),
        profile_revision: profile.revision,
        protocol: VersionedProtocol {
            id: ProtocolId::new("native-protocol").unwrap(),
            revision: Revision(5),
        },
        access: AccessSnapshot {
            project_id: project,
            roots: [root].into(),
            grants: [
                Permission::ReadRoot,
                Permission::MutateStream,
                Permission::ExecuteProcess,
                Permission::Network,
                Permission::UseCredential,
            ]
            .into(),
            policy_revision: Revision(1),
        },
        required_controls: BTreeSet::new(),
        context: ContextPolicy {
            bundle: ContextBundleId::new("native-context").unwrap(),
            revision: Revision(1),
            max_input_tokens: 16_384,
            reserved_output_tokens: 4_096,
        },
        required_tools: ["read_file".into()].into(),
        required_skills: ["review".into()].into(),
        escalation: EscalationPolicy::ReturnToLead,
    };
    let host = Host {
        id: descriptor.host_id.clone(),
        revision: Revision(1),
        device: DeviceId::new("device-a").unwrap(),
        fabric: None,
        supported_runtimes: vec![RuntimeKind::NativeSymbiote],
        controls: [
            Control::Filesystem,
            Control::Process,
            Control::Network,
            Control::Credentials,
            Control::Cancellation,
            Control::CompletionAuthority,
        ]
        .into_iter()
        .map(|control| {
            (
                control,
                EnforcementClaim {
                    strength: EnforcementStrength::HostEnforced,
                    evidence: EvidenceId::new("native-proof").unwrap(),
                    verified_at: Timestamp(1),
                    expires_at: Timestamp(1_000_000),
                },
            )
        })
        .collect(),
    };
    Dispatch::compile(
        DispatchId::new("native-dispatch").unwrap(),
        RuntimeContractId::new("native-runtime-contract").unwrap(),
        DispatchInputs {
            task: &task,
            role: &role,
            binding: &binding,
            profile: &profile,
            host: &host,
            limits: &ResourceLimits {
                max_total_tokens: 100_000,
                max_wall_time_ms: 60_000,
                max_concurrency: 1,
                max_memory_bytes: 1 << 30,
                max_cpu_millicores: None,
            },
            minimum_enforcement: &BTreeMap::new(),
            now: Timestamp(2),
        },
    )
    .expect("the native dispatch compiles")
}

/// A journal that persists what it is given and refuses nothing else — the
/// trusted Host's side of authorization, and the only thing that can mint a
/// permit.
struct Journal;

impl ActivationJournal for Journal {
    fn persist_authorization(
        &mut self,
        dispatch: &Dispatch,
        descriptor: &RuntimeDescriptor,
        at: Timestamp,
    ) -> Result<ActivationReceipt, AdapterError> {
        Ok(ActivationReceipt {
            command_id: CommandId::new("native-activation").unwrap(),
            revision: Revision(1),
            dispatch: dispatch.clone(),
            descriptor: descriptor.clone(),
            authorized_at: at,
        })
    }
}

fn permit(dispatch: &Dispatch, descriptor: &RuntimeDescriptor) -> LaunchPermit {
    PreparedLaunch::new(
        dispatch.clone(),
        descriptor.clone(),
        [Capability::Tools, Capability::Skills].into(),
        Timestamp(10),
    )
    .expect("the runtime qualifies for this dispatch")
    .authorize(&mut Journal, Timestamp(10))
    .expect("the journal persists the authorization")
}

/// What the mock harness observed and what it was told to do, shared with the
/// case so the adapter's refusals are read from the harness rather than inferred.
#[derive(Default)]
struct MockLog {
    started: Vec<SessionId>,
    /// A complete alternate handle for `start` to answer with, so identity and
    /// generation cleanup can be exercised without another harness implementation.
    start_as: Option<HarnessSession>,
    turns: Vec<String>,
    disposed: Vec<SessionId>,
    controls: usize,
    handshakes: usize,
    /// A report the mock answers with instead of its own, so a case can make it
    /// answer for another session or another dispatch.
    report: Option<RuntimeHandshake>,
    refuse_dispose: bool,
    /// Return more than the caller asked for, so the adapter's own page bound is
    /// measured rather than the mock's cooperation.
    ignore_limit: bool,
    events: Vec<RuntimeEvent>,
}

/// A mock harness: it records what it was handed, reports nothing by default
/// (silence is a fact the SDK records as unreported), and can be told to
/// misbehave so the adapter's refusals are exercised rather than assumed.
struct MockHarness {
    log: std::rc::Rc<std::cell::RefCell<MockLog>>,
}

impl MockHarness {
    fn new() -> (Self, std::rc::Rc<std::cell::RefCell<MockLog>>) {
        let log = std::rc::Rc::new(std::cell::RefCell::new(MockLog::default()));
        (Self { log: log.clone() }, log)
    }
}

impl HarnessProcess for MockHarness {
    fn start(&mut self, request: &LaunchRequest<'_>) -> Result<HarnessSession, AdapterError> {
        // The harness is handed the session, the dispatch, the descriptor and
        // the projection, and nothing else: no canonical content, no permit.
        assert_eq!(request.descriptor.runtime, RuntimeKind::NativeSymbiote);
        assert_eq!(request.projection.adapter, request.descriptor.adapter_id);
        let started = self
            .log
            .borrow()
            .start_as
            .clone()
            .unwrap_or_else(|| HarnessSession {
                session: request.session.clone(),
                generation: self.log.borrow().started.len() as u64 + 1,
            });
        if !self.log.borrow().started.contains(&started.session) {
            self.log.borrow_mut().started.push(started.session.clone());
        }
        Ok(started)
    }

    fn send(&mut self, session: &HarnessSession, input: &TurnInput) -> Result<(), AdapterError> {
        assert!(self.log.borrow().started.contains(&session.session));
        self.log.borrow_mut().turns.push(input.text.clone());
        Ok(())
    }

    fn events(
        &mut self,
        session: &HarnessSession,
        after: u64,
        limit: u16,
    ) -> Result<Vec<RuntimeEvent>, AdapterError> {
        assert!(self.log.borrow().started.contains(&session.session));
        let log = self.log.borrow();
        let limit = if log.ignore_limit { u16::MAX } else { limit };
        Ok(log
            .events
            .iter()
            .filter(|event| event.sequence() > after)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    fn control(
        &mut self,
        session: &HarnessSession,
        _command: &SessionControl,
    ) -> Result<(), AdapterError> {
        assert!(self.log.borrow().started.contains(&session.session));
        self.log.borrow_mut().controls += 1;
        Ok(())
    }

    fn dispose(&mut self, session: &HarnessSession) -> Result<(), AdapterError> {
        if self.log.borrow().refuse_dispose {
            return Err(AdapterError::Unavailable);
        }
        assert!(self.log.borrow().started.contains(&session.session));
        self.log.borrow_mut().disposed.push(session.session.clone());
        Ok(())
    }

    fn handshake(&self, session: &HarnessSession) -> Result<RuntimeHandshake, AdapterError> {
        assert!(self.log.borrow().started.contains(&session.session));
        self.log.borrow_mut().handshakes += 1;
        if let Some(report) = &self.log.borrow().report {
            return Ok(report.clone());
        }
        // Silence by default: the mock has no more to say than it was given, and
        // a report that claimed agreement would be indistinguishable from one the
        // adapter made up.
        Ok(RuntimeHandshake {
            dispatch: DispatchId::new("native-dispatch").unwrap(),
            session: session.session.clone(),
            surfaces: Vec::new(),
        })
    }
}

/// The mock's report of a session, taken from the projection the harness was
/// handed: what a process that loaded the contract actually took.
fn honest_handshake(session: &SessionId, projection: &ContractProjection) -> RuntimeHandshake {
    RuntimeHandshake {
        dispatch: projection.dispatch.clone(),
        session: session.clone(),
        surfaces: projection
            .surfaces
            .iter()
            .filter(|entry| !entry.preventive.is_empty() || !entry.carried.is_empty())
            .map(|entry| SurfaceReport {
                surface: entry.surface,
                preventive: entry
                    .preventive
                    .iter()
                    .filter_map(|(control, delivery)| {
                        delivery
                            .realized()
                            .map(|strength| (control.clone(), strength))
                    })
                    .collect(),
                carried: entry.carried.keys().cloned().collect(),
                mechanism: MECHANISM.into(),
            })
            .collect(),
    }
}

/// The whole point of this slice: a production adapter, over a real dispatch,
/// passes the suite and publishes a record a reader can hold. The run is over
/// actual work — the canonical Task, Role, binding and Host a native lane is
/// staffed from — because a suite run over something a fixture invented would
/// prove nothing about an adapter anyone will launch.
#[test]
fn the_native_adapter_passes_the_suite_and_publishes_a_readable_record() {
    let descriptor = descriptor();
    let dispatch = dispatch(&descriptor);
    let (harness, log) = MockHarness::new();
    let mut adapter = NativeAgentAdapter::with_harness(descriptor.clone(), Box::new(harness));

    // A session really launches: the permit was journaled, the harness was
    // started, and the identity is the adapter's own.
    let session = adapter
        .launch(permit(&dispatch, &descriptor), Timestamp(11))
        .expect("a native session launches");
    assert_eq!(log.borrow().started, vec![session.clone()]);
    assert_eq!(adapter.live_sessions(), vec![session.clone()]);

    // The suite runs over the real dispatch, through the adapter's own
    // projection, and every rule holds.
    let report = run_conformance(&adapter, &[&dispatch]);
    assert!(
        report.passed(),
        "every rule holds, and the failures name it: {:?}",
        report.failures()
    );
    assert_eq!(
        report.rules.len(),
        symbiote_runtime_sdk::conformance::RULES.len()
    );
    assert!(
        report.withheld().is_empty(),
        "this runtime carries every demand"
    );

    // And the record it publishes is one a reader can hold: the SDK builds it
    // from this run, and the same holding check the Host uses accepts it.
    let dossier = CompatibilityDossier::publish(vec![PackRun {
        adapter: &adapter,
        dispatches: vec![&dispatch],
    }])
    .expect("a passing run publishes");
    let run = dossier
        .run("0.1.0", "linux")
        .expect("the record publishes the version it was run against");
    assert_eq!(run.adapter.as_str(), "native-agent");
    assert_eq!(
        run.driver, None,
        "a native runtime drives no external harness"
    );
    assert_eq!(run.tier, symbiote_runtime_sdk::IntegrationTier::Detected);
    assert_eq!(run.capabilities, report.capabilities);
    assert!(run.withheld.is_empty());
    let declaration = declared();
    let held = dossier
        .hold(&declaration)
        .expect("this pack certified this runtime");
    assert_eq!(held.run.upstream_version, "0.1.0");
    assert_eq!(held.run.platform, "linux");
    assert_eq!(held.run.adapter_version, "0.1.0");
}

/// The adapter's default is no harness, and no harness means every session method
/// refuses. An adapter with nothing to drive must not simulate a turn, invent a
/// session, or return a handshake nobody sent.
#[test]
fn an_adapter_with_no_harness_refuses_every_session_method() {
    let descriptor = descriptor();
    let dispatch = dispatch(&descriptor);
    let mut adapter = NativeAgentAdapter::new(descriptor.clone());
    assert_eq!(
        adapter
            .launch(permit(&dispatch, &descriptor), Timestamp(11))
            .expect_err("an adapter with no harness launches nothing"),
        AdapterError::Unavailable
    );
    assert!(adapter.live_sessions().is_empty());
    let foreign = SessionId::new("native-dispatch-1").unwrap();
    let record = Session {
        id: foreign.clone(),
        dispatch_id: DispatchId::new("native-dispatch").unwrap(),
        kind: RuntimeKind::NativeSymbiote,
        foreign_reference: None,
    };
    let turn = TurnInput {
        text: "do the work".into(),
        artifacts: Vec::new(),
    };
    for refused in [
        adapter.send(&foreign, turn.clone()).err(),
        adapter.poll_events(&foreign, 0, 8).err(),
        adapter.control(&foreign, SessionControl::Cancel {}).err(),
        adapter.dispose(&foreign).err(),
        adapter.handshake(&record).err(),
    ] {
        assert_eq!(refused, Some(AdapterError::Unavailable));
    }
    // The projection is still readable, which is the point: an operator can see
    // what a native runtime could carry before anything is launched.
    assert_eq!(adapter.projection(&dispatch).surfaces.len(), 12);
}

/// The permit is rechecked at activation, against the descriptor the adapter
/// actually holds: a descriptor that moved, and a clock before the permit was
/// authorized, both refuse before a harness is started.
#[test]
fn launch_refuses_a_permit_the_adapter_cannot_still_honour() {
    let descriptor = descriptor();
    let dispatch = dispatch(&descriptor);
    let (harness, log) = MockHarness::new();
    let mut adapter = NativeAgentAdapter::with_harness(descriptor.clone(), Box::new(harness));

    // A descriptor the adapter no longer holds: the runtime moved under the
    // permit, and preparation latency cannot make the old proof current.
    let mut moved = descriptor.clone();
    moved.adapter_version = "0.2.0".into();
    let moved_proof = evidence(&moved);
    for support in moved.capabilities.values_mut() {
        if let symbiote_runtime_sdk::Support::Supported { evidence } = support {
            **evidence = moved_proof.clone();
        }
    }
    for support in moved.controls.values_mut() {
        support.evidence = moved_proof.clone();
    }
    assert_eq!(
        adapter
            .launch(permit(&dispatch, &moved), Timestamp(11))
            .expect_err("a moved descriptor"),
        AdapterError::ContractMismatch
    );
    // A clock before the permit was authorized: an authorization cannot be used
    // backwards.
    assert_eq!(
        adapter
            .launch(permit(&dispatch, &descriptor), Timestamp(9))
            .expect_err("a permit from the future"),
        AdapterError::ContractMismatch
    );
    assert!(
        log.borrow().started.is_empty(),
        "no harness is started for a refused launch"
    );
    // A harness that answers for another identity has not started this session.
    // The handle it returned is still a resource the adapter now owns, so it is
    // released before the mismatch is reported.
    let wrong = HarnessSession {
        session: SessionId::new("another-harness-session").unwrap(),
        generation: 1,
    };
    log.borrow_mut().start_as = Some(wrong.clone());
    assert_eq!(
        adapter
            .launch(permit(&dispatch, &descriptor), Timestamp(11))
            .expect_err("another harness identity"),
        AdapterError::SessionMismatch
    );
    assert!(adapter.live_sessions().is_empty());
    assert_eq!(log.borrow().started, vec![wrong.session.clone()]);
    assert_eq!(log.borrow().disposed, vec![wrong.session]);
    log.borrow_mut().start_as = None;
    log.borrow_mut().started.clear();
    log.borrow_mut().disposed.clear();
    // The refusals changed nothing: the adapter still launches the permit it can
    // honour, and only then.
    let session = adapter
        .launch(permit(&dispatch, &descriptor), Timestamp(11))
        .expect("the honoured permit launches");
    assert_eq!(log.borrow().started, vec![session.clone()]);

    // If a broken harness returns the exact complete handle already in the
    // table, cleanup must not end that live resource.
    log.borrow_mut().start_as = Some(HarnessSession {
        session: session.clone(),
        generation: 1,
    });
    assert_eq!(
        adapter
            .launch(permit(&dispatch, &descriptor), Timestamp(11))
            .expect_err("an already-owned handle"),
        AdapterError::SessionMismatch
    );
    assert_eq!(adapter.live_sessions(), vec![session.clone()]);
    assert!(log.borrow().disposed.is_empty());

    // The same session id at a different generation is not the owned handle, so
    // it is a newly returned resource and is released.
    log.borrow_mut().start_as = Some(HarnessSession {
        session: session.clone(),
        generation: 2,
    });
    assert_eq!(
        adapter
            .launch(permit(&dispatch, &descriptor), Timestamp(11))
            .expect_err("a new generation under the live id"),
        AdapterError::SessionMismatch
    );
    assert_eq!(adapter.live_sessions(), vec![session.clone()]);
    assert_eq!(log.borrow().disposed, vec![session]);
}

/// Every later call is bound to a session this adapter started. A harness cannot
/// answer for a session nobody launched, and a disposed session stays disposed.
#[test]
fn every_session_call_is_bound_to_a_session_the_adapter_started() {
    let descriptor = descriptor();
    let dispatch = dispatch(&descriptor);
    let (harness, log) = MockHarness::new();
    let mut adapter = NativeAgentAdapter::with_harness(descriptor.clone(), Box::new(harness));
    let session = adapter
        .launch(permit(&dispatch, &descriptor), Timestamp(11))
        .expect("a native session launches");
    let turn = TurnInput {
        text: "read the contract".into(),
        artifacts: Vec::new(),
    };

    // A session nobody started: every method refuses with the same name.
    let foreign = SessionId::new("native-dispatch-9").unwrap();
    for refused in [
        adapter.send(&foreign, turn.clone()).err(),
        adapter.poll_events(&foreign, 0, 8).err(),
        adapter
            .control(&foreign, SessionControl::Interrupt {})
            .err(),
        adapter.dispose(&foreign).err(),
    ] {
        assert_eq!(refused, Some(AdapterError::SessionMismatch));
    }
    assert!(log.borrow().disposed.is_empty(), "nothing was disposed");

    // A turn goes to the harness, bounded by the input's own validation: an
    // empty turn never reaches a process.
    adapter
        .send(&session, turn)
        .expect("a valid turn is handed on");
    assert_eq!(log.borrow().turns, vec!["read the contract".to_string()]);
    assert_eq!(
        adapter
            .send(
                &session,
                TurnInput {
                    text: "   ".into(),
                    artifacts: Vec::new()
                }
            )
            .expect_err("an empty turn"),
        AdapterError::InvalidInput
    );
    assert_eq!(log.borrow().turns.len(), 1);

    // Lifecycle control forwards what this reference adapter actually owns.
    // Resume and fork stay typed refusals: without continuation state or a
    // successor handle, forwarding them would claim a lifecycle it cannot bind.
    adapter
        .control(&session, SessionControl::Interrupt {})
        .expect("interrupt is forwarded");
    adapter
        .control(&session, SessionControl::Cancel {})
        .expect("cancel is forwarded");
    assert_eq!(log.borrow().controls, 2);
    assert_eq!(
        adapter
            .control(&session, SessionControl::Resume {})
            .expect_err("resume has no continuation store"),
        AdapterError::Unsupported {
            operation: AdapterOperation::Resume
        }
    );
    assert_eq!(
        adapter
            .control(
                &session,
                SessionControl::Fork {
                    successor: session.clone()
                }
            )
            .expect_err("fork has no successor handle"),
        AdapterError::Unsupported {
            operation: AdapterOperation::Fork
        }
    );
    assert_eq!(
        log.borrow().controls,
        2,
        "an unsupported command is not forwarded"
    );

    // Disposal ends the session, and the session stays ended: a second disposal
    // is a caller's error, not a second end.
    adapter.dispose(&session).expect("disposal is forwarded");
    assert_eq!(log.borrow().disposed, vec![session.clone()]);
    assert!(adapter.live_sessions().is_empty());
    assert_eq!(
        adapter.dispose(&session).expect_err("a disposed session"),
        AdapterError::SessionMismatch
    );
    assert_eq!(log.borrow().disposed.len(), 1);
}

/// A disposal the harness refuses leaves the session live, because an adapter
/// that forgot a session whose resources are still running would answer for a
/// process it no longer controls.
#[test]
fn a_refused_disposal_keeps_the_session_live() {
    let descriptor = descriptor();
    let dispatch = dispatch(&descriptor);
    let (harness, log) = MockHarness::new();
    log.borrow_mut().refuse_dispose = true;
    let mut adapter = NativeAgentAdapter::with_harness(descriptor.clone(), Box::new(harness));
    let session = adapter
        .launch(permit(&dispatch, &descriptor), Timestamp(11))
        .expect("a native session launches");
    assert_eq!(
        adapter.dispose(&session).expect_err("a refused disposal"),
        AdapterError::Unavailable
    );
    assert_eq!(adapter.live_sessions(), vec![session.clone()]);
    log.borrow_mut().refuse_dispose = false;
    adapter.dispose(&session).expect("the retry disposes");
    assert!(adapter.live_sessions().is_empty());
    // A later process for the same dispatch receives a new identity even though
    // the table is empty: a stale report cannot be credited to its successor.
    let successor = adapter
        .launch(permit(&dispatch, &descriptor), Timestamp(11))
        .expect("the dispatch launches again");
    assert_ne!(successor, session);
    assert_eq!(log.borrow().started, vec![session, successor.clone()]);
    adapter.dispose(&successor).expect("the successor disposes");
}

/// An event is bound to the session it was polled for. A harness that answers
/// for another lane fails at the adapter boundary rather than at whichever
/// consumer happened to read the event.
#[test]
fn an_event_for_another_session_is_refused_at_the_adapter() {
    let descriptor = descriptor();
    let dispatch = dispatch(&descriptor);
    let (harness, log) = MockHarness::new();
    let mut adapter = NativeAgentAdapter::with_harness(descriptor.clone(), Box::new(harness));
    let session = adapter
        .launch(permit(&dispatch, &descriptor), Timestamp(11))
        .expect("a native session launches");
    let binding = |session: &SessionId| SessionBinding {
        session_id: session.clone(),
        dispatch_id: dispatch.id().clone(),
        host_id: descriptor.host_id.clone(),
        runtime: RuntimeKind::NativeSymbiote,
    };
    let text = symbiote_runtime_sdk::events::EventText::new("working").unwrap();
    let event = |sequence: u64, session: &SessionId| {
        RuntimeEvent::new(
            CommandId::new(format!("native-event-{sequence}")).unwrap(),
            sequence,
            binding(session),
            symbiote_runtime_sdk::events::RuntimeEventKind::Message { text: text.clone() },
        )
        .expect("a well-formed event")
    };

    // The harness's own events, bound to this session, come through in order and
    // respect the caller's own bounds.
    log.borrow_mut().events = vec![event(1, &session), event(2, &session)];
    let first = adapter
        .poll_events(&session, 0, 8)
        .expect("the session's own events");
    assert_eq!(
        first
            .iter()
            .map(|event| event.sequence())
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    let second = adapter
        .poll_events(&session, 1, 8)
        .expect("events after the first");
    assert_eq!(
        second
            .iter()
            .map(|event| event.sequence())
            .collect::<Vec<_>>(),
        vec![2]
    );
    let bounded = adapter
        .poll_events(&session, 0, 1)
        .expect("the caller's limit bounds the page");
    assert_eq!(bounded.len(), 1);
    log.borrow_mut().ignore_limit = true;
    assert_eq!(
        adapter
            .poll_events(&session, 0, 1)
            .expect_err("a harness that ignores the page bound"),
        AdapterError::ContractMismatch
    );
    log.borrow_mut().ignore_limit = false;

    // An event bound to another session is refused here, before a consumer sees it.
    let other = SessionId::new("native-dispatch-2").unwrap();
    log.borrow_mut().events = vec![event(3, &other)];
    assert_eq!(
        adapter
            .poll_events(&session, 0, 8)
            .expect_err("an event for another session"),
        AdapterError::SessionMismatch
    );
    // And so is one bound to another dispatch under this session's identity.
    let mut foreign_dispatch = binding(&session);
    foreign_dispatch.dispatch_id = DispatchId::new("native-dispatch-7").unwrap();
    log.borrow_mut().events = vec![
        RuntimeEvent::new(
            CommandId::new("native-event-4").unwrap(),
            4,
            foreign_dispatch,
            symbiote_runtime_sdk::events::RuntimeEventKind::Message { text: text.clone() },
        )
        .expect("a well-formed event"),
    ];
    assert_eq!(
        adapter
            .poll_events(&session, 0, 8)
            .expect_err("an event for another dispatch"),
        AdapterError::SessionMismatch
    );
    // The rest of the immutable binding is checked too: a native Host event is
    // not made native by carrying the right session and dispatch identities.
    let mut foreign_host = binding(&session);
    foreign_host.host_id = HostId::new("host-b").unwrap();
    log.borrow_mut().events = vec![
        RuntimeEvent::new(
            CommandId::new("native-event-5").unwrap(),
            5,
            foreign_host,
            symbiote_runtime_sdk::events::RuntimeEventKind::Message { text: text.clone() },
        )
        .expect("a well-formed event"),
    ];
    assert_eq!(
        adapter
            .poll_events(&session, 0, 8)
            .expect_err("an event for another Host"),
        AdapterError::SessionMismatch
    );
    let mut foreign_runtime = binding(&session);
    foreign_runtime.runtime = RuntimeKind::ExternalHarness;
    log.borrow_mut().events = vec![
        RuntimeEvent::new(
            CommandId::new("native-event-6").unwrap(),
            6,
            foreign_runtime,
            symbiote_runtime_sdk::events::RuntimeEventKind::Message { text: text.clone() },
        )
        .expect("a well-formed event"),
    ];
    assert_eq!(
        adapter
            .poll_events(&session, 0, 8)
            .expect_err("an event for another runtime kind"),
        AdapterError::SessionMismatch
    );
    // Events after a cursor must actually advance it. Replayed or reordered
    // output is refused here rather than left for a consumer to guess about.
    log.borrow_mut().events = vec![event(8, &session), event(7, &session)];
    assert_eq!(
        adapter
            .poll_events(&session, 0, 8)
            .expect_err("events that move backwards"),
        AdapterError::ContractMismatch
    );
}

/// The handshake is the process's own account, and the adapter checks whose it
/// is before anyone reconciles it. A report about another session or another
/// dispatch is refused here; a report that widens what the projection delivered
/// is refused by the SDK's own reconciliation.
#[test]
fn the_handshake_is_the_process_report_and_it_may_only_narrow() {
    let descriptor = descriptor();
    let dispatch = dispatch(&descriptor);
    let (harness, log) = MockHarness::new();
    let mut adapter = NativeAgentAdapter::with_harness(descriptor.clone(), Box::new(harness));
    let session = adapter
        .launch(permit(&dispatch, &descriptor), Timestamp(11))
        .expect("a native session launches");
    let projection = adapter.projection(&dispatch);
    let record = Session {
        id: session.clone(),
        dispatch_id: dispatch.id().clone(),
        kind: RuntimeKind::NativeSymbiote,
        foreign_reference: None,
    };

    // The adapter's own report is the harness's, and silence reconciles to
    // "unreported" rather than to agreement.
    let silent = adapter.handshake(&record).expect("the harness reports");
    assert!(
        silent.surfaces.is_empty(),
        "a mock harness has nothing to add"
    );
    assert_eq!(log.borrow().handshakes, 1);
    let outcome = projection
        .reconcile(&record, &silent)
        .expect("silence is not a disagreement");
    assert!(
        !outcome.is_exact(),
        "an unreported surface is not agreement"
    );
    assert_eq!(
        outcome
            .surface(symbiote_runtime_sdk::projection::ContractSurface::PermissionsAndSandbox)
            .expect("the surface is read")
            .preventive
            .values()
            .next()
            .map(|state| state.agreement),
        Some(Agreement::Unreported)
    );

    // An honest process report — what the harness was handed, reported back —
    // reconciles exactly, and every demanded carrier is named rather than
    // inferred from silence. A caller record for another dispatch is refused
    // before the harness is asked to speak for it.
    log.borrow_mut().report = Some(honest_handshake(&session, &projection));
    let mut caller_dispatch = record.clone();
    caller_dispatch.dispatch_id = DispatchId::new("native-dispatch-5").unwrap();
    let handshakes = log.borrow().handshakes;
    assert_eq!(
        adapter
            .handshake(&caller_dispatch)
            .expect_err("a caller record for another dispatch"),
        AdapterError::SessionMismatch
    );
    assert_eq!(
        log.borrow().handshakes,
        handshakes,
        "the harness was not asked for a foreign caller record"
    );
    let reported = adapter.handshake(&record).expect("the process reports");
    let exact = projection
        .reconcile(&record, &reported)
        .expect("an honest report reconciles");
    assert!(
        exact.is_exact(),
        "what the process took is what was projected"
    );

    // A report that claims a control stronger than the projection delivered is
    // refused by the SDK's own comparison, and this adapter never edits it into
    // agreement first.
    let mut widened = reported.clone();
    for entry in &mut widened.surfaces {
        for strength in entry.preventive.values_mut() {
            *strength = EnforcementStrength::Native;
        }
    }
    assert_eq!(
        projection
            .reconcile(&record, &widened)
            .expect_err("a widening report"),
        AdapterError::ContractMismatch
    );

    // A report about another session, or another dispatch, is refused at the
    // adapter's own boundary — before a caller can hold it against this one.
    let other = SessionId::new("native-dispatch-8").unwrap();
    log.borrow_mut().report = Some(honest_handshake(&other, &projection));
    assert_eq!(
        adapter
            .handshake(&record)
            .expect_err("a report for another session"),
        AdapterError::SessionMismatch
    );
    let mut foreign = honest_handshake(&session, &projection);
    foreign.dispatch = DispatchId::new("native-dispatch-6").unwrap();
    log.borrow_mut().report = Some(foreign);
    assert_eq!(
        adapter
            .handshake(&record)
            .expect_err("a report for another dispatch"),
        AdapterError::ContractMismatch
    );
    // A record whose kind is not this runtime is refused as well: a handshake
    // reconciled against a native projection is not an external one's.
    let mut external = record.clone();
    external.kind = RuntimeKind::ExternalHarness;
    log.borrow_mut().report = Some(honest_handshake(&session, &projection));
    let handshakes = log.borrow().handshakes;
    assert_eq!(
        adapter
            .handshake(&external)
            .expect_err("a foreign runtime kind"),
        AdapterError::SessionMismatch
    );
    assert_eq!(
        log.borrow().handshakes,
        handshakes,
        "the harness was not asked for a foreign runtime kind"
    );
}

/// An unsupported method is an explicit error, never a successful no-op: the
/// adapter's own `Unsupported` refusals are the SDK's, and this crate states
/// that rather than re-deciding them.
#[test]
fn an_unsupported_operation_is_refused_by_name() {
    let error = AdapterError::Unsupported {
        operation: AdapterOperation::Fork,
    };
    assert!(error.to_string().contains("Fork"), "{error}");
    assert_ne!(
        error,
        AdapterError::Unavailable,
        "an unsupported operation is not the same fact as an unavailable one"
    );
}

/// The contract names the production subject and the exact boundary its cases
/// prove. A stale document cannot keep claiming that no production adapter
/// exists, and a renamed refusal or harness type cannot leave the prose calling
/// a different boundary implemented.
#[test]
fn the_contract_documents_name_the_adapter_boundary() {
    let runtime_sdk = include_str!("../../../docs/contracts/runtime-sdk.md");
    for name in [
        "NativeAgentAdapter",
        "HarnessProcess",
        "LaunchPermit::validate_at",
        "SessionMismatch",
        "Unavailable",
        "run_conformance",
        "CompatibilityDossier::publish",
        "CompatibilityDossier::hold",
    ] {
        assert!(
            runtime_sdk.contains(name),
            "runtime SDK contract omits {name}"
        );
    }
    assert!(
        !runtime_sdk.contains("No production adapter implements `AgentRuntimeAdapter` yet"),
        "the contract still says the production subject does not exist"
    );

    let native_loop = include_str!("../../../docs/contracts/native-agent-loop.md");
    for name in [
        "NativeAgentAdapter",
        "HarnessProcess",
        "LaunchRequest",
        "SessionBinding",
        "MockHarness",
        "SessionMismatch",
        "ContractMismatch",
        "Unsupported",
    ] {
        assert!(
            native_loop.contains(name),
            "native loop contract omits {name}"
        );
    }

    let events = include_str!("../../../docs/contracts/runtime-events.md");
    assert!(
        events.contains("complete `SessionBinding`")
            && events.contains("`MockHarness` is not process evidence"),
        "the event contract must distinguish adapter binding from process evidence"
    );
}
