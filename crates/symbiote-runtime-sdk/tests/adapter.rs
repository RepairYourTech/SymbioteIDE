use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
use symbiote_runtime_sdk::*;

fn proof(runtime: RuntimeKind) -> ProbeEvidence {
    ProbeEvidence {
        artifact: EvidenceId::new("proof").unwrap(),
        adapter_id: AgentRuntimeAdapterId::new("adapter").unwrap(),
        installation: profile(runtime).installation,
        profile_id: RuntimeProfileId::new("profile").unwrap(),
        profile_revision: Revision(1),
        model_id: ModelId::new("model").unwrap(),
        host_id: HostId::new("host").unwrap(),
        adapter_version: "1.0".into(),
        upstream_version: "fixture-1".into(),
        platform: "linux".into(),
        observed_at: Timestamp(1),
        expires_at: Timestamp(1000),
    }
}
fn profile(runtime: RuntimeKind) -> RuntimeProfile {
    RuntimeProfile {
        id: RuntimeProfileId::new("profile").unwrap(),
        revision: Revision(1),
        runtime,
        adapter: AgentRuntimeAdapterId::new("adapter").unwrap(),
        installation: (runtime == RuntimeKind::ExternalHarness)
            .then(|| InstallationId::new("installation").unwrap()),
        provider: ProviderConnectionId::new("provider").unwrap(),
        credential: CredentialReferenceId::new("credential").unwrap(),
        billing_entitlement: BillingEntitlementId::new("billing").unwrap(),
        model: ModelId::new("model").unwrap(),
        eligible_hosts: [HostId::new("host").unwrap()].into(),
    }
}

#[test]
fn explicit_control_minimums_preserve_partial_order_and_fresh_evidence() {
    use EnforcementStrength::*;
    let runtime = RuntimeKind::NativeSymbiote;
    let profile = profile(runtime);
    let mut d = descriptor(runtime);
    let strengths = [
        Native,
        HostEnforced,
        ExternallyObserved,
        Emulated,
        Unsupported,
    ];
    let expected = [
        [true, true, true, true, false],
        [false, true, true, true, false],
        [false, false, true, false, false],
        [false, false, false, true, false],
        [false, false, false, false, false],
    ];
    for (i, actual) in strengths.iter().enumerate() {
        d.controls.get_mut(&Control::Filesystem).unwrap().strength = *actual;
        for (j, minimum) in strengths.iter().enumerate() {
            assert_eq!(
                qualify_profile_with_minimums(
                    &profile,
                    &d,
                    &BTreeSet::new(),
                    &BTreeMap::from([(Control::Filesystem, *minimum)]),
                    Timestamp(10)
                )
                .is_ok(),
                expected[i][j],
                "actual {actual:?}, minimum {minimum:?}"
            );
        }
    }
    d.controls.get_mut(&Control::Filesystem).unwrap().strength = ExternallyObserved;
    let requirements = BTreeMap::from([(Control::Filesystem, ExternallyObserved)]);
    assert!(
        qualify_profile(
            &profile,
            &d,
            &BTreeSet::new(),
            &[Control::Filesystem].into(),
            Timestamp(10)
        )
        .is_err()
    );
    assert_eq!(
        qualify_profile_with_minimums(
            &profile,
            &d,
            &BTreeSet::new(),
            &requirements,
            Timestamp(1000)
        ),
        Err(QualificationError::StaleEvidence)
    );
    d.controls
        .get_mut(&Control::Filesystem)
        .unwrap()
        .evidence
        .profile_revision = Revision(9);
    assert_eq!(
        qualify_profile_with_minimums(&profile, &d, &BTreeSet::new(), &requirements, Timestamp(10)),
        Err(QualificationError::StaleEvidence)
    );
}
fn descriptor(runtime: RuntimeKind) -> RuntimeDescriptor {
    RuntimeDescriptor {
        sdk_version: SDK_VERSION,
        adapter_id: AgentRuntimeAdapterId::new("adapter").unwrap(),
        installation: profile(runtime).installation,
        profile_id: RuntimeProfileId::new("profile").unwrap(),
        profile_revision: Revision(1),
        model_id: ModelId::new("model").unwrap(),
        adapter_version: "1.0".into(),
        upstream_version: "fixture-1".into(),
        runtime,
        owner: if runtime == RuntimeKind::NativeSymbiote {
            RuntimeOwner::SymbioteNative {}
        } else {
            RuntimeOwner::External {
                driver: HarnessDriverId::new("fixture-driver").unwrap(),
            }
        },
        transport: if runtime == RuntimeKind::NativeSymbiote {
            Transport::NativeLoop
        } else {
            Transport::StructuredRpc
        },
        tier: IntegrationTier::Structured,
        host_id: HostId::new("host").unwrap(),
        platform: "linux".into(),
        capabilities: [
            Capability::StructuredMessages,
            Capability::ContextLimits,
            Capability::Tools,
        ]
        .into_iter()
        .map(|capability| {
            (
                capability,
                Support::Supported {
                    evidence: Box::new(proof(runtime)),
                },
            )
        })
        .collect(),
        controls: [
            Control::Filesystem,
            Control::Cancellation,
            Control::CompletionAuthority,
        ]
        .into_iter()
        .map(|control| {
            (
                control,
                ControlSupport {
                    strength: EnforcementStrength::HostEnforced,
                    mechanism: "fixture only".into(),
                    evidence: proof(runtime),
                },
            )
        })
        .collect(),
        tools: ["read_file".into()].into(),
        skills: BTreeSet::new(),
        context_limits: Some(RuntimeContextLimits {
            context_window_tokens: 8192,
            max_output_tokens: 2048,
        }),
    }
}
fn dispatch(runtime: RuntimeKind) -> Dispatch {
    let project = ProjectId::new("project").unwrap();
    let role_id = RoleId::new("engineer").unwrap();
    let root = RootId::new("root").unwrap();
    let task = Task::new(
        TaskId::new("task").unwrap(),
        project.clone(),
        root.clone(),
        role_id.clone(),
        ChangeStreamId::new("stream").unwrap(),
        VersionedTaskContract {
            id: TaskContractId::new("task-contract").unwrap(),
            revision: Revision(1),
        },
    );
    let role = Role {
        id: role_id.clone(),
        project_id: project.clone(),
        revision: Revision(1),
        name: "Engineer".into(),
        operating_contract: VersionedRoleContract {
            id: RoleContractId::new("role-contract").unwrap(),
            revision: Revision(1),
        },
    };
    let profile = profile(runtime);
    let binding = WorkforceBinding {
        id: BindingId::new("binding").unwrap(),
        revision: Revision(1),
        project_id: project.clone(),
        role_id,
        profile_id: profile.id.clone(),
        profile_revision: profile.revision,
        protocol: VersionedProtocol {
            id: ProtocolId::new("protocol").unwrap(),
            revision: Revision(1),
        },
        access: AccessSnapshot {
            project_id: project,
            roots: [root].into(),
            grants: [Permission::ReadRoot, Permission::MutateStream].into(),
            policy_revision: Revision(1),
        },
        required_controls: BTreeSet::new(),
        context: ContextPolicy {
            bundle: ContextBundleId::new("context").unwrap(),
            revision: Revision(1),
            max_input_tokens: 4096,
            reserved_output_tokens: 1024,
        },
        required_tools: ["read_file".into()].into(),
        required_skills: BTreeSet::new(),
        escalation: EscalationPolicy::ReturnToLead,
    };
    let host = Host {
        id: HostId::new("host").unwrap(),
        revision: Revision(1),
        device: DeviceId::new("device").unwrap(),
        fabric: None,
        supported_runtimes: vec![runtime],
        controls: [
            Control::Filesystem,
            Control::Cancellation,
            Control::CompletionAuthority,
        ]
        .into_iter()
        .map(|control| {
            (
                control,
                EnforcementClaim {
                    strength: EnforcementStrength::HostEnforced,
                    evidence: EvidenceId::new("proof").unwrap(),
                    verified_at: Timestamp(1),
                    expires_at: Timestamp(1000),
                },
            )
        })
        .collect::<BTreeMap<_, _>>(),
    };
    Dispatch::compile(
        DispatchId::new("dispatch").unwrap(),
        RuntimeContractId::new("runtime-contract").unwrap(),
        DispatchInputs {
            task: &task,
            role: &role,
            binding: &binding,
            profile: &profile,
            host: &host,
            minimum_enforcement: &BTreeMap::new(),
            now: Timestamp(2),
        },
    )
    .unwrap()
}

#[test]
fn native_external_and_mixed_staffing_preserve_role_task_and_stream_identity() {
    let native = dispatch(RuntimeKind::NativeSymbiote);
    let external = dispatch(RuntimeKind::ExternalHarness);
    for dispatch in [&native, &external] {
        qualify_dispatch(
            dispatch,
            &descriptor(dispatch.contract().profile().runtime),
            &BTreeSet::new(),
            Timestamp(3),
        )
        .unwrap();
    }
    assert_eq!(native.contract().role().id, external.contract().role().id);
    assert_eq!(native.contract().task_id(), external.contract().task_id());
    assert_eq!(
        native.contract().stream_id(),
        external.contract().stream_id()
    );
    assert!(native.contract().profile().installation.is_none());
}

#[test]
fn external_full_harness_cannot_claim_native_ownership_or_transport() {
    let mut descriptor = descriptor(RuntimeKind::ExternalHarness);
    descriptor.owner = RuntimeOwner::SymbioteNative {};
    assert_eq!(
        descriptor.validate(),
        Err(QualificationError::RuntimeOwnerMismatch)
    );
    assert!(
        serde_json::from_str::<RuntimeOwner>(r#"{"kind":"symbiote_native","driver":"codex"}"#)
            .is_err()
    );
    assert!(
        serde_json::from_str::<SessionControl>(r#"{"kind":"cancel","approve_all":true}"#).is_err()
    );
}

#[test]
fn evidence_expiry_version_drift_and_observed_controls_reject_qualification() {
    let dispatch = dispatch(RuntimeKind::ExternalHarness);
    let mut report = descriptor(RuntimeKind::ExternalHarness);
    report
        .controls
        .get_mut(&Control::Filesystem)
        .unwrap()
        .strength = EnforcementStrength::ExternallyObserved;
    assert_eq!(
        qualify_dispatch(&dispatch, &report, &BTreeSet::new(), Timestamp(3)),
        Err(QualificationError::InsufficientControl(Control::Filesystem))
    );
    let mut report = descriptor(RuntimeKind::ExternalHarness);
    report.upstream_version = "changed".into();
    assert_eq!(
        qualify_dispatch(&dispatch, &report, &BTreeSet::new(), Timestamp(3)),
        Err(QualificationError::StaleEvidence)
    );
    assert!(
        qualify_dispatch(
            &dispatch,
            &descriptor(RuntimeKind::ExternalHarness),
            &BTreeSet::new(),
            Timestamp(1000)
        )
        .is_err()
    );
}

#[test]
fn proof_cannot_be_substituted_across_adapter_installation_profile_or_model() {
    for change in 0..4 {
        let mut profile = profile(RuntimeKind::ExternalHarness);
        let mut report = descriptor(RuntimeKind::ExternalHarness);
        match change {
            0 => {
                profile.adapter = AgentRuntimeAdapterId::new("other-adapter").unwrap();
                report.adapter_id = profile.adapter.clone();
            }
            1 => {
                profile.installation = Some(InstallationId::new("other-installation").unwrap());
                report.installation = profile.installation.clone();
            }
            2 => {
                profile.revision = Revision(2);
                report.profile_revision = Revision(2);
            }
            _ => {
                profile.model = ModelId::new("other-model").unwrap();
                report.model_id = profile.model.clone();
            }
        }
        assert_eq!(
            qualify_profile(
                &profile,
                &report,
                &[Capability::Tools].into(),
                &[Control::Filesystem].into(),
                Timestamp(3)
            ),
            Err(QualificationError::StaleEvidence)
        );
    }
}

#[test]
fn tier_never_substitutes_for_capability_and_context_or_tool_requirements() {
    let dispatch = dispatch(RuntimeKind::ExternalHarness);
    let mut report = descriptor(RuntimeKind::ExternalHarness);
    report.tier = IntegrationTier::Certified;
    report
        .capabilities
        .insert(Capability::Tools, Support::Unknown);
    assert_eq!(
        qualify_dispatch(&dispatch, &report, &BTreeSet::new(), Timestamp(3)),
        Err(QualificationError::UnknownCapability(Capability::Tools))
    );
    let mut report = descriptor(RuntimeKind::ExternalHarness);
    report.tools.clear();
    assert_eq!(
        qualify_dispatch(&dispatch, &report, &BTreeSet::new(), Timestamp(3)),
        Err(QualificationError::MissingResource)
    );
    let mut report = descriptor(RuntimeKind::ExternalHarness);
    report
        .context_limits
        .as_mut()
        .unwrap()
        .context_window_tokens = 2048;
    assert_eq!(
        qualify_dispatch(&dispatch, &report, &BTreeSet::new(), Timestamp(3)),
        Err(QualificationError::ContextLimitExceeded)
    );
}

struct Journal {
    fail: bool,
    mismatch: bool,
    writes: usize,
}
impl ActivationJournal for Journal {
    fn persist_authorization(
        &mut self,
        dispatch: &Dispatch,
        descriptor: &RuntimeDescriptor,
        at: Timestamp,
    ) -> Result<ActivationReceipt, AdapterError> {
        self.writes += 1;
        if self.fail {
            return Err(AdapterError::AuthorizationNotPersisted);
        }
        Ok(ActivationReceipt {
            command_id: CommandId::new("authorization").unwrap(),
            revision: Revision(if self.mismatch { 0 } else { 1 }),
            dispatch: dispatch.clone(),
            descriptor: descriptor.clone(),
            authorized_at: at,
        })
    }
}
struct FixtureAdapter {
    descriptor: RuntimeDescriptor,
    launched: bool,
}
impl AgentRuntimeAdapter for FixtureAdapter {
    fn descriptor(&self) -> &RuntimeDescriptor {
        &self.descriptor
    }
    fn launch(&mut self, permit: LaunchPermit, at: Timestamp) -> Result<SessionId, AdapterError> {
        permit.validate_at(&self.descriptor, at)?;
        self.launched = true;
        Ok(SessionId::new("fixture-session").unwrap())
    }
}

#[test]
fn preparation_is_inert_and_authorization_precedes_activation_with_fresh_recheck() {
    let mut journal = Journal {
        fail: true,
        mismatch: false,
        writes: 0,
    };
    let prepared = PreparedLaunch::new(
        dispatch(RuntimeKind::NativeSymbiote),
        descriptor(RuntimeKind::NativeSymbiote),
        BTreeSet::new(),
        Timestamp(3),
    )
    .unwrap();
    assert_eq!(journal.writes, 0);
    assert!(matches!(
        prepared.authorize(&mut journal, Timestamp(4)),
        Err(AdapterError::AuthorizationNotPersisted)
    ));
    assert_eq!(journal.writes, 1);
    journal.fail = false;
    journal.mismatch = true;
    assert!(matches!(
        PreparedLaunch::new(
            dispatch(RuntimeKind::NativeSymbiote),
            descriptor(RuntimeKind::NativeSymbiote),
            BTreeSet::new(),
            Timestamp(3)
        )
        .unwrap()
        .authorize(&mut journal, Timestamp(4)),
        Err(AdapterError::ContractMismatch)
    ));
    journal.mismatch = false;
    let permit = PreparedLaunch::new(
        dispatch(RuntimeKind::NativeSymbiote),
        descriptor(RuntimeKind::NativeSymbiote),
        BTreeSet::new(),
        Timestamp(3),
    )
    .unwrap()
    .authorize(&mut journal, Timestamp(4))
    .unwrap();
    let mut adapter = FixtureAdapter {
        descriptor: descriptor(RuntimeKind::NativeSymbiote),
        launched: false,
    };
    assert!(adapter.launch(permit, Timestamp(1000)).is_err());
    assert!(!adapter.launched);
}

#[test]
fn structured_and_generic_pty_expose_different_guarantees_for_same_dispatch() {
    let dispatch = dispatch(RuntimeKind::ExternalHarness);
    let structured = descriptor(RuntimeKind::ExternalHarness);
    let mut pty = structured.clone();
    pty.transport = Transport::Pty;
    pty.tier = IntegrationTier::TerminalCompatible;
    pty.capabilities.clear();
    pty.context_limits = None;
    pty.controls.clear();
    pty.tools.clear();
    assert!(PreparedLaunch::new(dispatch.clone(), pty, BTreeSet::new(), Timestamp(3)).is_err());
    let mut journal = Journal {
        fail: false,
        mismatch: false,
        writes: 0,
    };
    let permit = PreparedLaunch::new(dispatch, structured.clone(), BTreeSet::new(), Timestamp(3))
        .unwrap()
        .authorize(&mut journal, Timestamp(4))
        .unwrap();
    let mut adapter = FixtureAdapter {
        descriptor: structured,
        launched: false,
    };
    let session = adapter.launch(permit, Timestamp(5)).unwrap();
    assert!(adapter.launched);
    assert_eq!(
        adapter.poll_events(&session, 0, 10),
        Err(AdapterError::Unsupported {
            operation: AdapterOperation::PollEvents
        })
    );
    assert_eq!(
        adapter.control(&session, SessionControl::Resume {}),
        Err(AdapterError::Unsupported {
            operation: AdapterOperation::Resume
        })
    );
}
