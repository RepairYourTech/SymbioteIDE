use std::time::{SystemTime, UNIX_EPOCH};
use symbiote_domain::{
    Actor, DispatchId, DomainError, HostId, RuntimeContractId, TaskId, Timestamp, WorkCommand,
    WorkError, WorkItem,
};
use symbiote_protocol::*;
use symbiote_store::{Store, StoreError};

fn storage_error(error: StoreError) -> ProtocolError {
    let code = match &error {
        StoreError::NotFound => ErrorCode::NotFound,
        StoreError::AlreadyExists => ErrorCode::Conflict,
        StoreError::IdempotencyConflict => ErrorCode::IdempotencyConflict,
        StoreError::TeamRevisionConflict => ErrorCode::StaleRevision,
        StoreError::BindingRevisionConflict => ErrorCode::StaleRevision,
        StoreError::InvalidBinding
        | StoreError::InvalidRoute
        | StoreError::InvalidDependency
        | StoreError::InvalidLease => ErrorCode::InvalidRequest,
        StoreError::DependenciesUnresolved => ErrorCode::Conflict,
        StoreError::InvalidProvider | StoreError::InvalidPreparation => ErrorCode::InvalidRequest,
        StoreError::DispatchRefused(_) | StoreError::ElevationCeiling => {
            ErrorCode::FailedPrecondition
        }
        StoreError::InvalidElevation => ErrorCode::InvalidRequest,
        StoreError::LeaseConflict(_) => ErrorCode::Conflict,
        StoreError::ResourceExhausted => ErrorCode::ResourceExhausted,
        StoreError::InvalidTeam => ErrorCode::InvalidRequest,
        StoreError::InvalidPage => ErrorCode::InvalidCursor,
        StoreError::InvalidInitialState
        | StoreError::RelationshipMismatch
        | StoreError::InvalidConsent => ErrorCode::InvalidRequest,
        StoreError::Domain(DomainError::RevisionConflict) => ErrorCode::StaleRevision,
        StoreError::Domain(DomainError::Cycle)
        | StoreError::Domain(DomainError::MissingReference) => ErrorCode::Conflict,
        StoreError::Work(WorkError::RevisionConflict) => ErrorCode::StaleRevision,
        StoreError::Work(WorkError::IdempotencyConflict) => ErrorCode::IdempotencyConflict,
        StoreError::Work(WorkError::ResourceLimit) => ErrorCode::ResourceExhausted,
        StoreError::Work(_) | StoreError::UnclassifiedTask => ErrorCode::InvalidRequest,
        StoreError::Domain(_) => ErrorCode::InvalidRequest,
        StoreError::Sqlite(_) => ErrorCode::Unavailable,
        _ => ErrorCode::Internal,
    };
    let mut protocol_error = ProtocolError::new(code);
    // The one place a dispatch refusal becomes a protocol error: the reason is
    // a member of a closed vocabulary (never an input value or a path), so
    // naming it is safe, and the same reason reads the same wherever a
    // dispatch is refused — the code alone would collapse it into the generic
    // precondition message and lose the answer the operator asked for.
    if let StoreError::DispatchRefused(refusal) = &error {
        protocol_error.message = format!("dispatch refused by recorded state ({})", refusal.name());
    }
    protocol_error
}

fn receipt(receipt: symbiote_store::Receipt) -> ResponseBody {
    ResponseBody::Receipt(Receipt {
        sequence: receipt.sequence,
        revision: receipt.revision,
        replayed: receipt.replayed,
    })
}

fn consent_timestamp(store: &Store, request: &Request) -> Result<Timestamp, ProtocolError> {
    if let Some(at) = store
        .consent_command_timestamp(&request.command_id)
        .map_err(storage_error)?
    {
        return Ok(at);
    }
    now_timestamp()
}

fn placement_timestamp(store: &Store, request: &Request) -> Result<Timestamp, ProtocolError> {
    if let Some(at) = store
        .placement_command_timestamp(&request.command_id)
        .map_err(storage_error)?
    {
        return Ok(at);
    }
    now_timestamp()
}

fn route_timestamp(store: &Store, request: &Request) -> Result<Timestamp, ProtocolError> {
    if let Some(at) = store
        .route_command_timestamp(&request.command_id)
        .map_err(storage_error)?
    {
        return Ok(at);
    }
    now_timestamp()
}

fn start_timestamp(store: &Store, request: &Request) -> Result<Timestamp, ProtocolError> {
    if let Some(at) = store
        .preparation_command_timestamp(&request.command_id)
        .map_err(storage_error)?
    {
        return Ok(at);
    }
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
        .as_millis();
    Ok(Timestamp(
        millis
            .try_into()
            .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
    ))
}

/// The local Host's enforcement-claim record. Claims are operator-provisioned
/// (see docs/contracts/dispatch-preparation.md); the Host asserts them for its
/// own identity only, with a fixed one-hour verification window until
/// sandbox-observed evidence lands (#269). `credentials_enforced` is true
/// only when the operator provisioned a credential broker (#217): a Host
/// with no broker never claims `Credentials`, so a dispatch granting
/// `UseCredential` refuses at start instead of running unisolated.
fn host_record(
    host_id: &HostId,
    inventory: &mut crate::inventory::InventoryService,
    credentials_enforced: bool,
    external_enforced: bool,
) -> Result<symbiote_domain::Host, ProtocolError> {
    if inventory.host_id() != host_id {
        return Err(ProtocolError::new(ErrorCode::PermissionDenied));
    }
    let now = now_timestamp()?;
    let evidence = symbiote_domain::EvidenceId::new(format!("host-claims-{}", host_id.as_str()))
        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?;
    let mut controls = vec![
        symbiote_domain::Control::Filesystem,
        symbiote_domain::Control::Process,
        symbiote_domain::Control::Cancellation,
        symbiote_domain::Control::CompletionAuthority,
    ];
    if credentials_enforced {
        controls.push(symbiote_domain::Control::Credentials);
    }
    let claims = controls
        .into_iter()
        .map(|control| {
            (
                control,
                symbiote_domain::EnforcementClaim {
                    strength: symbiote_domain::EnforcementStrength::HostEnforced,
                    evidence: evidence.clone(),
                    verified_at: now,
                    expires_at: Timestamp(now.0 + 3_600_000),
                },
            )
        })
        .collect();
    // The Host advertises `ExternalHarness` only when the operator
    // provisioned the external execution path (the explicitly labeled
    // fixture harness; a live pinned-binary path stays gated on explicit
    // user authorization). Without it, a binding with an external profile
    // refuses at start — never a silent fallback to the native runtime.
    let supported_runtimes = if external_enforced {
        vec![
            symbiote_domain::RuntimeKind::NativeSymbiote,
            symbiote_domain::RuntimeKind::ExternalHarness,
        ]
    } else {
        vec![symbiote_domain::RuntimeKind::NativeSymbiote]
    };
    Ok(symbiote_domain::Host {
        id: host_id.clone(),
        revision: symbiote_domain::Revision(0),
        device: symbiote_domain::DeviceId::new(format!("device-{}", host_id.as_str()))
            .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
        fabric: None,
        supported_runtimes,
        controls: claims,
    })
}

fn provider_timestamp(store: &Store, request: &Request) -> Result<Timestamp, ProtocolError> {
    if let Some(at) = store
        .provider_command_timestamp(&request.command_id)
        .map_err(storage_error)?
    {
        return Ok(at);
    }
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
        .as_millis();
    Ok(Timestamp(
        millis
            .try_into()
            .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
    ))
}

fn lease_timestamp(store: &Store, request: &Request) -> Result<Timestamp, ProtocolError> {
    if let Some(at) = store
        .lease_command_timestamp(&request.command_id)
        .map_err(storage_error)?
    {
        return Ok(at);
    }
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
        .as_millis();
    Ok(Timestamp(
        millis
            .try_into()
            .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
    ))
}

fn dependency_timestamp(store: &Store, request: &Request) -> Result<Timestamp, ProtocolError> {
    if let Some(at) = store
        .dependency_command_timestamp(&request.command_id)
        .map_err(storage_error)?
    {
        return Ok(at);
    }
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
        .as_millis();
    Ok(Timestamp(
        millis
            .try_into()
            .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
    ))
}

fn work_timestamp(store: &Store, request: &Request) -> Result<Timestamp, ProtocolError> {
    if let Some(at) = store
        .work_command_timestamp(&request.command_id)
        .map_err(storage_error)?
    {
        return Ok(at);
    }
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
        .as_millis();
    Ok(Timestamp(
        millis
            .try_into()
            .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
    ))
}

fn execute(
    store: &mut Store,
    principal: &Principal,
    inventory: &mut crate::inventory::InventoryService,
    workers: &mut crate::runner::WorkerTransports,
    request: &Request,
) -> Result<ResponseBody, ProtocolError> {
    authorize(principal, request)?;
    match &request.operation {
        Operation::ReplaceBinding {
            expected_revision,
            configuration,
        } => {
            let at = match store
                .binding_command_timestamp(&request.command_id)
                .map_err(storage_error)?
            {
                Some(at) => at,
                None => Timestamp(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
                        .as_millis()
                        .try_into()
                        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
                ),
            };
            store
                .replace_binding(
                    request.command_id.clone(),
                    *expected_revision,
                    *configuration.clone(),
                    principal.user_id().clone(),
                    at,
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::GetBinding {
            project_id,
            binding_id,
        } => store
            .get_binding(project_id, binding_id)
            .map(|binding| ResponseBody::Binding(Box::new(binding)))
            .map_err(storage_error),
        Operation::ResolveRoute { request } => resolve_route(store, request)
            .map(|decision| ResponseBody::RouteDecision(Box::new(decision))),
        Operation::RecordRoute { request: route } => {
            let at = route_timestamp(store, request)?;
            let decision = resolve_route(store, route)?;
            store
                .record_route(
                    request.command_id.clone(),
                    decision,
                    principal.user_id().clone(),
                    at,
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::GetRoute {
            project_id,
            work_id,
        } => store
            .get_route(project_id, work_id)
            .map(|decision| ResponseBody::RouteDecision(Box::new(decision)))
            .map_err(storage_error),
        Operation::SetTaskDependencies {
            project_id,
            task_id,
            dependencies,
        } => {
            let at = dependency_timestamp(store, request)?;
            store
                .set_task_dependencies(
                    request.command_id.clone(),
                    project_id.clone(),
                    task_id.clone(),
                    dependencies.iter().cloned().collect(),
                    principal.user_id().clone(),
                    at,
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::GetTaskDependencies {
            project_id,
            task_id,
        } => {
            let edges = store
                .task_dependencies(project_id, task_id)
                .map_err(storage_error)?;
            for edge in &edges {
                if !principal.permits(&edge.target.project_id, ProjectPermission::Read) {
                    return Err(ProtocolError::new(ErrorCode::PermissionDenied));
                }
            }
            Ok(ResponseBody::TaskDependencies(edges.into_iter().collect()))
        }
        Operation::AcquireTaskLease {
            task_id,
            dispatch_id,
            host_id,
            duration_ms,
        } => {
            let at = lease_timestamp(store, request)?;
            store
                .acquire_lease(
                    request.command_id.clone(),
                    task_id.clone(),
                    dispatch_id.clone(),
                    host_id.clone(),
                    *duration_ms,
                    principal.user_id().clone(),
                    at,
                )
                .and_then(|_| {
                    store
                        .task_lease(task_id)
                        .map(|lease| ResponseBody::TaskLease(Box::new(lease)))
                })
                .map_err(storage_error)
        }
        Operation::ReleaseTaskLease {
            task_id,
            dispatch_id,
            fencing_token,
        } => {
            let at = lease_timestamp(store, request)?;
            store
                .release_lease(
                    request.command_id.clone(),
                    task_id.clone(),
                    dispatch_id.clone(),
                    *fencing_token,
                    principal.user_id().clone(),
                    at,
                )
                .and_then(|_| {
                    store
                        .task_lease(task_id)
                        .map(|lease| ResponseBody::TaskLease(Box::new(lease)))
                })
                .map_err(storage_error)
        }
        Operation::ExpireStaleLeases {} => {
            let now = Timestamp(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
                    .as_millis()
                    .try_into()
                    .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
            );
            let expired = store
                .expire_stale_leases(principal.user_id().clone(), now)
                .map_err(storage_error)?;
            let projection = store.scheduling_projection(now).map_err(storage_error)?;
            Ok(ResponseBody::SchedulerSweep {
                expired: expired
                    .into_iter()
                    .map(|(task_id, fencing_token)| ExpiredLease {
                        task_id,
                        fencing_token,
                    })
                    .collect(),
                schedulable: projection.schedulable,
                blocked: projection.blocked,
            })
        }
        Operation::ReplaceProviderConnection {
            attribution,
            connection,
        } => {
            let at = provider_timestamp(store, request)?;
            store
                .replace_provider_connection(
                    request.command_id.clone(),
                    attribution.clone(),
                    connection.clone(),
                    principal.user_id().clone(),
                    at,
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::ReplaceBillingEntitlement {
            attribution,
            entitlement,
        } => {
            let at = provider_timestamp(store, request)?;
            store
                .replace_billing_entitlement(
                    request.command_id.clone(),
                    attribution.clone(),
                    (**entitlement).clone(),
                    principal.user_id().clone(),
                    at,
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::ReplaceModelDescriptor {
            attribution,
            descriptor,
        } => {
            let at = provider_timestamp(store, request)?;
            store
                .replace_model_descriptor(
                    request.command_id.clone(),
                    attribution.clone(),
                    (**descriptor).clone(),
                    principal.user_id().clone(),
                    at,
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::GetProviderConnection { provider_id } => store
            .provider_connection(provider_id)
            .map(|connection| ResponseBody::ProviderConnection(Box::new(connection)))
            .map_err(storage_error),
        Operation::GetBillingEntitlement { entitlement_id } => store
            .billing_entitlement(entitlement_id)
            .map(|entitlement| ResponseBody::BillingEntitlement(Box::new(entitlement)))
            .map_err(storage_error),
        Operation::GetModelDescriptor { model_id } => store
            .model_descriptor(model_id)
            .map(|descriptor| ResponseBody::ModelDescriptor(Box::new(descriptor)))
            .map_err(storage_error),
        Operation::PrepareDispatch { task_id } => {
            let at = match store
                .preparation_command_timestamp(&request.command_id)
                .map_err(storage_error)?
            {
                Some(at) => at,
                None => Timestamp(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
                        .as_millis()
                        .try_into()
                        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
                ),
            };
            let (receipt, preparation) = store
                .prepare_dispatch(
                    request.command_id.clone(),
                    task_id.clone(),
                    principal.user_id().clone(),
                    at,
                )
                .map_err(storage_error)?;
            let _ = receipt;
            Ok(ResponseBody::DispatchPreparation(Box::new(preparation)))
        }
        Operation::StartPreparedTask { task_id, host_id } => {
            // The caller must be the Host itself; the request's host identity
            // is checked against the principal in authorize(). The Host record
            // carries this process's enforcement claims: the claims are the
            // operator-provisioned set documented in dispatch-preparation.md, with
            // evidence windows owned by the Host operator.
            let claims_host = host_record(
                host_id,
                inventory,
                workers.has_credential_broker(),
                workers.has_external_transport(),
            )?;
            let at = start_timestamp(store, request)?;
            // Dispatch and contract identities are minted by the Host from
            // its nonce-bearing inventory identity, deterministic per task.
            // Stable per task across daemon restarts (P2 from PR #494
            // review): the lease audit requires dispatch-id stability across
            // generations of a task's leases.
            let dispatch_id = DispatchId::new(format!("disp_{}", task_id.as_str()))
                .map_err(|_| ProtocolError::new(ErrorCode::Internal))?;
            let contract_id = RuntimeContractId::new(format!("rtc_{}", task_id.as_str()))
                .map_err(|_| ProtocolError::new(ErrorCode::Internal))?;
            let (receipt, dispatch) = store
                .start_prepared_task(
                    request.command_id.clone(),
                    task_id.clone(),
                    dispatch_id.clone(),
                    contract_id.clone(),
                    &claims_host,
                    principal.user_id().clone(),
                    at,
                )
                .map_err(storage_error)?;
            let _ = receipt;
            let lease = store.task_lease(task_id).map_err(storage_error)?;
            Ok(ResponseBody::StartedDispatch(Box::new(StartedDispatch {
                task_id: task_id.clone(),
                dispatch_id: dispatch.id().clone(),
                host_id: claims_host.id.clone(),
                stream_id: dispatch.contract().stream_id().clone(),
                fencing_token: lease.fencing_token,
                started_at: at,
            })))
        }
        Operation::RunStartedDispatch {
            task_id,
            dispatch_id,
        } => {
            // Activation of an already-started dispatch. The runner
            // re-checks every precondition against journaled state (Running
            // under this dispatch, contract validity, runtime-kind match)
            // and files completion evidence only on a genuinely finished
            // turn. Transports come from the Host's configured factory:
            // with none configured the activation refuses rather than
            // running anything, because live execution requires explicit
            // operator/user authorization for credentials and billing.
            let at = now_timestamp()?;
            // Both preconditions are checked before any transport is built:
            // the dispatch-id binding (a foreign id never reaches a factory)
            // and the task's Running state (re-activating an already-filed
            // task must not launch a transport it will never use).
            let task_record = store.task(task_id).map_err(storage_error)?;
            let current = task_record
                .current_dispatch()
                .ok_or_else(dispatch_binding_refused)?;
            if current.id() != dispatch_id
                || task_record.state() != &symbiote_domain::TaskState::Running
            {
                return Err(dispatch_binding_refused());
            }
            // The third precondition, and the one that makes #447 real: what
            // executes is the contract's PINNED profile, so its registration
            // is re-validated here at execution time. A registration that was
            // valid at preparation and start can lapse before the run
            // (entitlement expiry, supersession), and an expired or unknown
            // entitlement must not execute. Checked before run configuration
            // and before any worktree side effect.
            let registration = store
                .provider_registration(current.contract().profile(), at)
                .map_err(storage_error)?;
            if let Some(refusal) = registration.refusal {
                // The same refusal a start would report, through the one
                // mapping that turns a dispatch refusal into the protocol's
                // answer.
                return Err(storage_error(StoreError::DispatchRefused(
                    symbiote_domain::DispatchRefusal::Provider(refusal),
                )));
            } // And the second execution-time precondition: the limits the
            // dispatch declared must bind the execution it is about to
            // produce. Every process a tool spawns is launched with the bound
            // they reduce to; a limit the Host has no kernel bound for — and a
            // contract that records no limits at all — refuses the run here,
            // naming the reason, rather than running unbound.
            let bound =
                crate::limits::bounds_for(current.contract().limits()).map_err(limit_unbounded)?;
            let runtime = current.contract().profile().runtime;
            // Worktree provisioning happens after the precondition checks
            // and before any transport is built: the #211 composition
            // verifies the stream's reserved location, validates the source
            // repository's HEAD against the stream's recorded base, and
            // materializes the derived worktree. A moved base or tampered
            // location refuses before any transport factory runs.
            // The reservation base is Host configuration, not store state:
            // its absence is the production refusal, and it fires before the
            // provisioning reads and git calls below — but after the dispatch
            // preconditions above, which read recorded state (the task, its
            // pinned profile's registration) and touch no worktree, git call
            // or transport.
            let reservation_base = workers.reservation_base().map_err(worker_error)?;
            let this_host = inventory.host_id().clone();
            let provisioned = crate::runner::provision_worktree(
                store,
                task_id,
                &this_host,
                &mut workers.git(),
                &reservation_base,
            )
            .map_err(worker_error)?;
            let context = resolve_run_context(store, task_id, current)?;
            let prompt = symbiote_context::render_prompt(&context);
            match runtime {
                symbiote_domain::RuntimeKind::NativeSymbiote => {
                    let mut transport = workers.native_build().map_err(worker_error)?;
                    // Credential leases (#217): the dispatch's profile
                    // references a credential, so the operator's broker
                    // must be configured and THIS dispatch's binding
                    // access must grant `UseCredential` — the typed
                    // authority that a credential lease may issue at all.
                    // Cross-project references are refused by the broker's
                    // own project check; with no broker the run refuses
                    // rather than proceeding without a credential it
                    // declared. The lease is issued, held for this run's
                    // scope check, and DROPPED UNMATERIALIZED: the native
                    // loop has no environment-injection mechanism yet, so
                    // no value moves anywhere. Values never enter the
                    // journal — the lease lives in process memory only.
                    let credential = current.contract().profile().credential.clone();
                    let scope = symbiote_context::LeaseScope {
                        dispatch_id: current.id().clone(),
                        project_id: context.project_id.clone(),
                        role_id: current.contract().binding().role_id.clone(),
                        profile_id: current.contract().profile().id.clone(),
                        host_id: this_host.clone(),
                    };
                    let profile_refs = [credential.clone()];
                    // The typed authority to hold a credential lease:
                    // the binding's own grant, or an explicit elevation
                    // lease the owner approved for THIS dispatch and
                    // whose window is still open at the run's clock.
                    let use_credential_granted = current
                        .contract()
                        .effective_access()
                        .grants
                        .contains(&symbiote_domain::Permission::UseCredential)
                        || store
                            .active_elevation(
                                current.id(),
                                &symbiote_domain::Permission::UseCredential,
                                at,
                            )
                            .map_err(storage_error)?;
                    let lease = workers
                        .resolve_native_credential(
                            store,
                            task_id,
                            current,
                            &credential,
                            symbiote_context::BrokerRequest {
                                scope: &scope,
                                profile_credential_refs: &profile_refs,
                                use_credential_granted,
                                at,
                            },
                            at,
                        )
                        .map_err(worker_error)?;
                    drop(lease);
                    // Shell tool execution is the operator's composition:
                    // with a configured executor factory, declared shell
                    // tools run through the sandbox inside the provisioned
                    // worktree under the operator's consent authority; with
                    // none, declared tools stay propose-only (recorded,
                    // never executed). The inputs come from the dispatch
                    // contract and the provisioning outcome — never from
                    // loop or model input. The current policy is the
                    // binding's access, widened with ExecuteProcess only
                    // while an owner-decided elevation lease for this
                    // dispatch is active (the sandbox independently
                    // re-checks every grant it requires).
                    let shell_access = crate::runner::resolve_shell_access(store, current, at)
                        .map_err(worker_error)?;
                    let inputs = crate::runner::ShellExecutorInputs {
                        root_id: &provisioned.root_id,
                        worktree: &provisioned.worktree,
                        host: &this_host,
                        project_id: task_record.project_id(),
                        role_id: &current.contract().binding().role_id,
                        profile_id: &current.contract().profile().id,
                        access: &shell_access,
                        user_id: principal.user_id(),
                        bound,
                    };
                    let tool_execution = match workers.shell_build(inputs) {
                        Ok(Some(executor)) => Some(crate::runner::ToolExecution {
                            executor,
                            worktree: provisioned.worktree.clone(),
                        }),
                        Ok(None) => None,
                        Err(error) => return Err(worker_error(error)),
                    };
                    let outcome = crate::runner::run_native_boxed(
                        store,
                        task_id,
                        current,
                        &prompt,
                        transport.as_mut(),
                        tool_execution,
                        at,
                    )
                    .map_err(worker_error)?;
                    Ok(ResponseBody::WorkerRun(Box::new(WorkerRun {
                        task_id: task_id.clone(),
                        dispatch_id: outcome.dispatch_id,
                        runtime,
                        completed: outcome.completion_filed,
                    })))
                }
                symbiote_domain::RuntimeKind::ExternalHarness => {
                    let mut transport = workers.external_build().map_err(worker_error)?;
                    // The sandboxed launcher mounts the reserved worktree at
                    // /workspace; that is the only cwd the harness sees.
                    let outcome = crate::runner::run_external_boxed(
                        store,
                        task_id,
                        current,
                        &prompt,
                        "/workspace",
                        transport.as_mut(),
                        at,
                    )
                    .map_err(worker_error)?;
                    Ok(ResponseBody::WorkerRun(Box::new(WorkerRun {
                        task_id: task_id.clone(),
                        dispatch_id: outcome.dispatch_id,
                        runtime,
                        completed: outcome.completion_filed,
                    })))
                }
            }
        }
        Operation::RequestTaskCompletion {
            task_id,
            dispatch_id,
            report,
        } => {
            // Route the worker's report through the domain lifecycle. The
            // local owner is proxying: the Host constructs the Worker actor
            // from caller input, and safety comes from the domain binding
            // that actor to the task's actual current dispatch. The task can
            // only reach CompletionRequested; verification and independent
            // review remain Host gates after this.
            let task_record = store.task(task_id).map_err(storage_error)?;
            let command = symbiote_domain::TaskCommand {
                id: request.command_id.clone(),
                expected_revision: task_record.revision(),
                actor: Actor::Worker(dispatch_id.clone()),
                at: Timestamp(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
                        .as_millis()
                        .try_into()
                        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
                ),
                action: symbiote_domain::TaskAction::RequestCompletion {
                    dispatch_id: dispatch_id.clone(),
                    report: report.clone(),
                },
            };
            store
                .apply_task(task_id, command)
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::GetDispatchPreparation { task_id } => store
            .dispatch_preparation(task_id)
            .map(|preparation| ResponseBody::DispatchPreparation(Box::new(preparation)))
            .map_err(storage_error),
        Operation::GetSchedulingProjection {} => {
            let now = Timestamp(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
                    .as_millis()
                    .try_into()
                    .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
            );
            let projection = store.scheduling_projection(now).map_err(storage_error)?;
            Ok(ResponseBody::SchedulerSweep {
                expired: Vec::new(),
                schedulable: projection.schedulable,
                blocked: projection.blocked,
            })
        }
        Operation::GetBindingReadiness {
            project_id,
            binding_id,
        } => {
            let binding = store
                .get_binding(project_id, binding_id)
                .map_err(storage_error)?;
            let team = store.get_team(project_id).map_err(storage_error)?;
            let pulse = inventory.pulse()?;
            let now = Timestamp(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
                    .as_millis()
                    .try_into()
                    .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
            );
            // The report observes what the execution boundary decides: the
            // candidate's registration is resolved through the registry's own
            // validation (a refusal is recorded, never an error), and the
            // operator's declared runtime for that exact profile is observed
            // with this Host's identity and a fresh window. Neither is invented
            // here; an absent declaration stays a missing observation.
            let registration = store
                .provider_registration(&binding.primary.profile, now)
                .map_err(storage_error)?;
            let descriptor = match workers.declared_runtime_for(&binding.primary.profile) {
                Some(declared) => Some(
                    declared
                        .observe(inventory.host_id(), now)
                        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
                ),
                None => None,
            };
            Ok(ResponseBody::BindingReadiness(Box::new(
                symbiote_workforce::assess_readiness(
                    &binding,
                    &team,
                    Some(&pulse),
                    descriptor.as_ref(),
                    Some(&registration),
                    now,
                ),
            )))
        }
        Operation::GetHostPulse {} => inventory
            .pulse()
            .map(|pulse| ResponseBody::HostPulse(Box::new(pulse))),
        Operation::ReplaceTeam {
            expected_revision,
            team,
        } => {
            let at = match store
                .team_command_timestamp(&request.command_id)
                .map_err(storage_error)?
            {
                Some(at) => at,
                None => Timestamp(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
                        .as_millis()
                        .try_into()
                        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
                ),
            };
            store
                .replace_team(
                    request.command_id.clone(),
                    *expected_revision,
                    *team.clone(),
                    principal.user_id().clone(),
                    at,
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::GetTeam { project_id } => store
            .get_team(project_id)
            .map(|team| ResponseBody::Team(Box::new(team)))
            .map_err(storage_error),
        Operation::CreateWork { work } => {
            let at = work_timestamp(store, request)?;
            let item = WorkItem::new(work.clone(), principal.user_id().clone(), at)
                .map_err(|_| ProtocolError::new(ErrorCode::InvalidRequest))?;
            store
                .create_work_item(request.command_id.clone(), item)
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::ChangeWork {
            project_id,
            id,
            expected_revision,
            edit,
        } => {
            let item = store.work_item(project_id, id).map_err(storage_error)?;
            authorize_work_resource(principal, project_id, &item)?;
            let command = WorkCommand {
                id: request.command_id.clone(),
                expected_revision: *expected_revision,
                actor: Actor::User(principal.user_id().clone()),
                at: work_timestamp(store, request)?,
                action: edit.clone().into_action(),
            };
            store
                .apply_work_command(project_id, id, command)
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::GetWork { project_id, id } => {
            let item = store.work_item(project_id, id).map_err(storage_error)?;
            authorize_work_resource(principal, project_id, &item)?;
            Ok(ResponseBody::Work(Box::new(item)))
        }
        Operation::AssignTaskOrigin {
            project_id,
            task_id,
            origin,
        } => {
            let at = work_timestamp(store, request)?;
            store
                .assign_task_origin(
                    request.command_id.clone(),
                    project_id,
                    task_id,
                    origin.clone(),
                    principal.user_id().clone(),
                    at,
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::GetTaskOrigin {
            project_id,
            task_id,
        } => store
            .task_origin(project_id, task_id)
            .map(ResponseBody::TaskOrigin)
            .map_err(storage_error),
        Operation::Hello { supported_versions } => {
            Ok(ResponseBody::Hello(negotiate(supported_versions)?))
        }
        Operation::Health {} => Ok(ResponseBody::Hello(negotiate(&[CURRENT_VERSION])?)),
        Operation::Shutdown {} => Ok(ResponseBody::Shutdown {}),
        Operation::RequestElevation {
            task_id,
            dispatch_id,
            permission,
            reason,
        } => {
            // A worker's ask is evidence. The Host attributes it to a
            // Running dispatch and journals it; it NEVER licenses. Only
            // decide_elevation can grant a lease. The durable timestamp
            // keeps retries byte-identical.
            let task_record = store.task(task_id).map_err(storage_error)?;
            let at = match store
                .elevation_request_timestamp(&request.command_id)
                .map_err(storage_error)?
            {
                Some(at) => at,
                None => now_timestamp()?,
            };
            let ask = symbiote_domain::ElevationRequest {
                id: request.command_id.clone(),
                project_id: task_record.project_id().clone(),
                task_id: task_id.clone(),
                dispatch_id: dispatch_id.clone(),
                permission: permission.clone(),
                reason: reason.clone(),
                requested_at: at,
            };
            store
                .request_elevation(request.command_id.clone(), ask)
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::DecideElevation { lease } => {
            // The deciding authority is the local owner; the store
            // verifies the lease's attribution (this dispatch, running),
            // that the permission is not already granted, and the Team
            // ceiling. The client-supplied decision time keeps retries
            // byte-identical and the window is bounded by domain law.
            store
                .decide_elevation(request.command_id.clone(), lease.as_ref().clone())
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::RevokeElevation {
            project_id,
            elevation_id,
        } => {
            let at = match store
                .elevation_revocation_timestamp(&request.command_id)
                .map_err(storage_error)?
            {
                Some(at) => at,
                None => now_timestamp()?,
            };
            store
                .revoke_elevation(
                    request.command_id.clone(),
                    project_id,
                    elevation_id,
                    principal.user_id(),
                    at,
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::RecordResourceConsent {
            snapshot,
            expires_at,
        } => {
            let consent = ResourceConsent {
                id: request.command_id.clone(),
                snapshot: snapshot.clone(),
                user_id: principal.user_id().clone(),
                issued_at: consent_timestamp(store, request)?,
                expires_at: *expires_at,
                revoked_at: None,
            };
            store
                .record_resource_consent(request.command_id.clone(), consent)
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::RevokeResourceConsent {
            project_id,
            consent_id,
        } => {
            let at = consent_timestamp(store, request)?;
            store
                .revoke_resource_consent(
                    request.command_id.clone(),
                    project_id,
                    consent_id,
                    principal.user_id(),
                    at,
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::GetResourceConsent {
            project_id,
            consent_id,
        } => store
            .resource_consent(project_id, consent_id)
            .map(|consent| ResponseBody::ResourceConsent(Box::new(consent)))
            .map_err(storage_error),
        Operation::RegisterProject { project } => {
            // Reuse the persisted authority timestamp on retry; a changed clock or
            // correlation ID must not change the identity of an acknowledged command.
            let at = match store
                .registration_timestamp(&request.command_id)
                .map_err(storage_error)?
            {
                Some(at) => at,
                None => Timestamp(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
                        .as_millis()
                        .try_into()
                        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
                ),
            };
            let (project, roots, roles) = project.clone().into_records(principal, at)?;
            store
                .register_project(request.command_id.clone(), project, roots, roles)
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::GetProject { project_id } => store
            .project(project_id)
            .map(ResponseBody::Project)
            .map_err(storage_error),
        Operation::ObserveRootPlacement {
            project_id,
            root_id,
            host_id,
            path,
            expected_revision,
        } => {
            // A placement names a repository location on a specific Host. The
            // claimed Host must be this Host's own inventory identity, and
            // the path must observe as a real git work tree here, before any
            // placement is recorded; no other Host's placement can be
            // asserted through this daemon.
            if inventory.host_id() != host_id {
                return Err(ProtocolError::new(ErrorCode::PermissionDenied));
            }
            let observed =
                std::fs::canonicalize(path).map_err(|_| ProtocolError::new(ErrorCode::NotFound))?;
            let canonical = observed
                .into_os_string()
                .into_string()
                .map_err(|_| ProtocolError::new(ErrorCode::NotFound))?;
            let mut git = symbiote_repo::SystemGit::new();
            symbiote_repo::observe_head(&mut git, std::path::Path::new(&canonical))
                .map_err(|_| ProtocolError::new(ErrorCode::NotFound))?;
            let at = placement_timestamp(store, request)?;
            store
                .observe_root_placement(
                    request.command_id.clone(),
                    project_id,
                    root_id,
                    host_id,
                    &canonical,
                    *expected_revision,
                    principal.user_id(),
                    at,
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::CreateTask { task } => {
            let origin = task.origin.clone();
            let (task, stream) = task.clone().into_records()?;
            store
                .create_task(request.command_id.clone(), task, stream, origin)
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::GetTask {
            project_id,
            task_id,
        } => {
            let task = store.task(task_id).map_err(storage_error)?;
            authorize_task_resource(principal, project_id, &task)?;
            Ok(ResponseBody::Task(Box::new(task)))
        }
        Operation::ReadJournal {
            project_id,
            after,
            limit,
        } => {
            let page = store
                .events(project_id, after.0, *limit)
                .map_err(storage_error)?;
            let events = page
                .events
                .into_iter()
                .map(|event| {
                    match &event.payload {
                        symbiote_store::EventPayload::WorkItemCreated { item }
                        | symbiote_store::EventPayload::WorkItemChanged { item, .. } => {
                            authorize_work_resource(principal, project_id, item)?
                        }
                        symbiote_store::EventPayload::TaskDependenciesSet {
                            edges,
                            project_id: owner,
                            ..
                        } => {
                            if owner != project_id {
                                return Err(ProtocolError::new(ErrorCode::PermissionDenied));
                            }
                            for edge in edges {
                                if !principal
                                    .permits(&edge.target.project_id, ProjectPermission::Read)
                                {
                                    return Err(ProtocolError::new(ErrorCode::PermissionDenied));
                                }
                            }
                        }
                        _ => {}
                    }
                    Ok(JournalEvent {
                        sequence: event.sequence,
                        project_id: event.project_id,
                        command_id: event.command_id,
                        revision: event.revision,
                        payload: match event.payload {
                            symbiote_store::EventPayload::BindingReplaced {
                                configuration,
                                expected_revision,
                                actor,
                                at,
                            } => EventPayload::BindingReplaced {
                                configuration,
                                expected_revision,
                                actor,
                                at,
                            },
                            symbiote_store::EventPayload::TeamReplaced {
                                team,
                                expected_revision,
                                actor,
                                at,
                            } => EventPayload::TeamReplaced {
                                team,
                                expected_revision,
                                actor,
                                at,
                            },
                            symbiote_store::EventPayload::WorkRouted {
                                decision,
                                actor,
                                at,
                            } => EventPayload::WorkRouted {
                                decision,
                                actor,
                                at,
                            },
                            registry_payload @ (
                                symbiote_store::EventPayload::ProviderRegistered { .. }
                                | symbiote_store::EventPayload::EntitlementRegistered { .. }
                                | symbiote_store::EventPayload::ModelRegistered { .. }
                            ) => {
                                // Registry records are global identity
                                // disclosed only to the owner authority; a
                                // project's journal reader learns that
                                // *something* was registered, never the
                                // record contents.
                                if !principal.is_local_owner() {
                                    return Err(ProtocolError::new(ErrorCode::PermissionDenied));
                                }
                                match registry_payload {
                                    symbiote_store::EventPayload::ProviderRegistered {
                                        attribution,
                                        connection,
                                        actor,
                                        at,
                                    } => EventPayload::ProviderRegistered {
                                        attribution,
                                        connection,
                                        actor,
                                        at,
                                    },
                                    symbiote_store::EventPayload::EntitlementRegistered {
                                        attribution,
                                        entitlement,
                                        actor,
                                        at,
                                    } => EventPayload::EntitlementRegistered {
                                        attribution,
                                        entitlement,
                                        actor,
                                        at,
                                    },
                                    symbiote_store::EventPayload::ModelRegistered {
                                        attribution,
                                        descriptor,
                                        actor,
                                        at,
                                    } => EventPayload::ModelRegistered {
                                        attribution,
                                        descriptor,
                                        actor,
                                        at,
                                    },
                                    _ => unreachable!("matched above"),
                                }
                            }
                            symbiote_store::EventPayload::DispatchPrepared {
                                preparation,
                                actor,
                                at,
                            } => EventPayload::DispatchPrepared {
                                preparation,
                                actor,
                                at,
                            },
                            symbiote_store::EventPayload::TaskLeased { lease, actor, at } => {
                                EventPayload::TaskLeased { lease, actor, at }
                            }
                            symbiote_store::EventPayload::TaskDependenciesSet {
                                task_id,
                                project_id,
                                edges,
                                actor,
                                at,
                            } => EventPayload::TaskDependenciesSet {
                                task_id,
                                project_id,
                                edges,
                                actor,
                                at,
                            },
                            symbiote_store::EventPayload::WorkItemCreated { item } => {
                                EventPayload::WorkItemCreated { item }
                            }
                            symbiote_store::EventPayload::WorkItemChanged {
                                project_id,
                                work_id,
                                command,
                                item,
                            } => EventPayload::WorkItemChanged {
                                project_id,
                                work_id,
                                command,
                                item,
                            },
                            symbiote_store::EventPayload::TaskOriginAssigned {
                                task_id,
                                project_id,
                                origin,
                                actor,
                                at,
                            } => EventPayload::TaskOriginAssigned {
                                task_id,
                                project_id,
                                origin,
                                actor,
                                at,
                            },
                            symbiote_store::EventPayload::ResourceConsentRecorded { consent } => {
                                EventPayload::ResourceConsentRecorded { consent }
                            }
                            symbiote_store::EventPayload::ResourceConsentRevoked {
                                consent,
                                revoked_by,
                            } => EventPayload::ResourceConsentRevoked {
                                consent,
                                revoked_by,
                            },
                            symbiote_store::EventPayload::ElevationDecided { lease } => {
                                EventPayload::ElevationDecided { lease }
                            }
                            symbiote_store::EventPayload::ElevationRevoked { lease, revoked_by } => {
                                EventPayload::ElevationRevoked { lease, revoked_by }
                            }
                            symbiote_store::EventPayload::ElevationRequested { ask } => {
                                EventPayload::ElevationRequested { ask }
                            }
                            symbiote_store::EventPayload::ProjectRegistered {
                                project,
                                roots,
                                roles,
                            } => EventPayload::ProjectRegistered {
                                project: *project,
                                roots,
                                roles,
                            },
                            symbiote_store::EventPayload::RootPlacementObserved {
                                root,
                                expected_revision,
                                host_id,
                                path,
                                actor,
                                at,
                            } => EventPayload::RootPlacementObserved {
                                root,
                                expected_revision,
                                host_id,
                                path,
                                actor,
                                at,
                            },
                            symbiote_store::EventPayload::TaskCreated {
                                task,
                                stream,
                                origin,
                            } => EventPayload::TaskCreated {
                                task,
                                stream,
                                origin,
                            },
                            symbiote_store::EventPayload::TaskChanged {
                                task_id,
                                command,
                                task,
                            } => EventPayload::TaskChanged {
                                task_id,
                                command: *command,
                                task,
                            },
                        },
                    })
                })
                .collect::<Result<Vec<_>, ProtocolError>>()?;
            let page = JournalPage {
                events,
                next_cursor: JournalCursor(page.next_cursor),
                has_more: page.has_more,
            };
            page.validate(project_id, *after, *limit)?;
            Ok(ResponseBody::Journal(page))
        }
    }
}

pub fn handle(
    store: &mut Store,
    principal: &Principal,
    inventory: &mut crate::inventory::InventoryService,
    workers: &mut crate::runner::WorkerTransports,
    request: Request,
) -> (Response, bool) {
    let result = execute(store, principal, inventory, workers, &request);
    let shutdown = result.is_ok() && matches!(request.operation, Operation::Shutdown {});
    (
        Response {
            version: CURRENT_VERSION,
            correlation_id: Some(request.correlation_id),
            result,
        },
        shutdown,
    )
}

fn now_timestamp() -> Result<Timestamp, ProtocolError> {
    Ok(Timestamp(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
            .as_millis()
            .try_into()
            .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
    ))
}

/// The worker's task context comes from canonical state — the task origin's
/// classified work item (description, requirements, constraints, risks,
/// acceptance) plus the dispatch contract's own budget and access — never
/// from caller input. Resolution failures map to stable protocol errors
/// that carry no store or task text.
fn resolve_run_context(
    store: &Store,
    task_id: &TaskId,
    dispatch: &symbiote_domain::Dispatch,
) -> Result<symbiote_context::ResolvedContext, ProtocolError> {
    let (project_id, origin_work) =
        crate::context_resolution::task_origin_work(store, task_id).map_err(context_error)?;
    let contract = dispatch.contract();
    let inputs = symbiote_context::ResolutionInputs {
        dispatch_id: dispatch.id(),
        task_id,
        project_id: &project_id,
        origin_work: &origin_work,
        context: contract.binding().context.clone(),
        access: contract.effective_access().clone(),
    };
    let source = crate::context_resolution::StoreContextSource { store };
    symbiote_context::resolve_context(&source, inputs).map_err(context_error)
}

fn context_error(error: symbiote_context::ResolutionError) -> ProtocolError {
    let mut protocol_error = ProtocolError::new(ErrorCode::FailedPrecondition);
    protocol_error.message = match error {
        symbiote_context::ResolutionError::NoPrompt => "task origin provides no run prompt".into(),
        symbiote_context::ResolutionError::Overbound => {
            "task origin text exceeds structural bounds".into()
        }
        symbiote_context::ResolutionError::StoreFailed => "store operation refused".into(),
    };
    protocol_error
}

/// Why the Host cannot bind this dispatch's declared limits, named for the
/// operator without carrying any input value.
fn limit_unbounded(refusal: crate::limits::BoundRefusal) -> ProtocolError {
    let mut protocol_error = ProtocolError::new(ErrorCode::FailedPrecondition);
    protocol_error.message = refusal.refusal().into();
    protocol_error
}

fn dispatch_binding_refused() -> ProtocolError {
    let mut protocol_error = ProtocolError::new(ErrorCode::FailedPrecondition);
    protocol_error.message = "task is not running under the requested dispatch".into();
    protocol_error
}

fn worker_error(error: crate::runner::RunnerError) -> ProtocolError {
    // The runner's error kinds are stable identifiers, not input values;
    // they reach the operator without leaking task content.
    let mut protocol_error = ProtocolError::new(ErrorCode::FailedPrecondition);
    protocol_error.message = match &error {
        crate::runner::RunnerError::RuntimeMismatch => "runtime kind does not match loop".into(),
        crate::runner::RunnerError::NotRunning => "task is not running under this dispatch".into(),
        crate::runner::RunnerError::InvalidContract => "dispatch contract no longer valid".into(),
        crate::runner::RunnerError::LoopFailed(_) => "worker loop halted without finishing".into(),
        crate::runner::RunnerError::NoReport => "worker loop finished without a report".into(),
        crate::runner::RunnerError::Store(payload) => {
            // Stage-honest by payload: activation-time store failures
            // (task/stream/root reads, integrity refusals) must not
            // masquerade as completion-filing failures.
            if payload.contains("policy seed") {
                "worktree policy seed rejected".into()
            } else if payload.contains("root id integrity") {
                "root record integrity refused".into()
            } else {
                "store operation refused".into()
            }
        }
        crate::runner::RunnerError::NoTransport => {
            "no worker transport configured for this runtime kind".into()
        }
        crate::runner::RunnerError::TransportBuild(_) => {
            "worker transport factory refused to build".into()
        }
        crate::runner::RunnerError::ShellExecutorBuild(_) => {
            "worker shell executor factory refused to build".into()
        }
        crate::runner::RunnerError::NoCredentialBroker => {
            "no credential broker configured for this dispatch's credential reference".into()
        }
        crate::runner::RunnerError::CredentialRefused(reason) => {
            format!("credential lease refused: {reason}")
        }
        crate::runner::RunnerError::NoHostPath => "no repository placement for this host".into(),
        crate::runner::RunnerError::NoReservationBase => {
            "no worktree reservation base configured".into()
        }
        crate::runner::RunnerError::InvalidSeed => "worktree policy seed rejected".into(),
        crate::runner::RunnerError::ProvisioningStore(_) => {
            "provisioning store operation refused".into()
        }
        crate::runner::RunnerError::Provisioning(error) => {
            // ProvisionError is a Copy enum of stage identifiers — never
            // task content.
            format!("worktree provisioning refused: {error:?}")
        }
    };
    protocol_error
}

/// Resolves routing against the stored Team for the referenced canonical work
/// item. The Host never accepts a client-supplied decision; inputs are the
/// authenticated request only.
fn resolve_route(
    store: &Store,
    request: &symbiote_workforce::RouteRequest,
) -> Result<symbiote_workforce::RouteDecision, ProtocolError> {
    store
        .work_item(&request.project_id, &request.work_id)
        .map_err(storage_error)?;
    let team = store.get_team(&request.project_id).map_err(storage_error)?;
    symbiote_workforce::resolve_route(&team, request)
        .map_err(|_| ProtocolError::new(ErrorCode::InvalidRequest))
}
