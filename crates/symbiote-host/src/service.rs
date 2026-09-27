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
        | StoreError::InvalidForeignLink
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
    // A lifecycle refusal is a closed vocabulary too, and it is the answer to
    // the question the caller asked — "why will my completion not file?" —
    // so it is named rather than collapsed into the generic `invalid_request`
    // sentence. Without this, a claim that does not cover the contract, a
    // review by the Task's own Role and a stream that is not validated all
    // arrive as the same words, and a caller can act on none of them.
    if let StoreError::Domain(refusal) = &error {
        protocol_error.message = format!("lifecycle refused the transition ({refusal:?})");
    }
    protocol_error
}

/// A dossier read this Host cannot answer. The message names a closed
/// vocabulary — which refusal this Host hit, never a path, a version, a platform
/// or an identity the document said — so an operator learns why without the wire
/// echoing a pack's own file back at it.
fn dossier_refusal(name: &str) -> ProtocolError {
    let mut error = ProtocolError::new(ErrorCode::FailedPrecondition);
    error.message = format!("compatibility dossier not served ({name})");
    error
}

/// A runtime inventory this Host will not serve, named by the same closed
/// vocabulary the reader applies. The name is [`crate::runtime_inventory::Refusal::name`]
/// rather than a second spelling of it, so an operator reads the same answer
/// from the wire that a case reads from the module, and the message carries no
/// path, no record and no value the document said.
fn inventory_refusal(refusal: crate::runtime_inventory::Refusal) -> ProtocolError {
    let mut error = ProtocolError::new(ErrorCode::FailedPrecondition);
    error.message = format!("runtime inventory not served ({})", refusal.name());
    error
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

fn foreign_link_timestamp(store: &Store, request: &Request) -> Result<Timestamp, ProtocolError> {
    if let Some(at) = store
        .foreign_link_command_timestamp(&request.command_id)
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
        Operation::SetTaskForeignLinks {
            project_id,
            task_id,
            links,
        } => {
            // The Host owns the observation time and the journal entry; the
            // wire carries no actor, no time and no canonical status, so a
            // foreign harness's own `complete` is stored as that harness's
            // claim, stamped when this Host recorded it, and the Task it is
            // held against does not move.
            let at = foreign_link_timestamp(store, request)?;
            store
                .set_task_foreign_links(
                    request.command_id.clone(),
                    project_id.clone(),
                    task_id.clone(),
                    links.iter().map(|claim| claim.observed_at(at)).collect(),
                    principal.user_id().clone(),
                    at,
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::GetTaskForeignLinks {
            project_id,
            task_id,
        } => {
            let links = store
                .task_foreign_links(project_id, task_id)
                .map_err(storage_error)?;
            Ok(ResponseBody::TaskForeignLinks(links.into_iter().collect()))
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
        Operation::GetSchedulingProjection { project_id } => {
            let now = lease_timestamp(store, request)?;
            let projection = store
                .scheduling_projection(project_id, now)
                .map_err(storage_error)?;
            // The cross-Project rule from #94, applied to a whole answer: a gate
            // that names a task in a Project the caller cannot read is refused
            // rather than served with that side dropped, because an unnamed
            // blocker is a reason the reader cannot act on. Redaction would also
            // let the answer's shape report what it withheld. The set is the
            // whole answer's, so a gate on a task outside the remaining closure
            // and outside the critical path is covered too.
            for referenced in projection.referenced_projects() {
                if !principal.permits(&referenced, ProjectPermission::Read) {
                    return Err(ProtocolError::new(ErrorCode::PermissionDenied));
                }
            }
            Ok(ResponseBody::SchedulingProjection(Box::new(projection)))
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
            // Only the expiry travels here. The sweep used to repeat the
            // projection's schedulable/blocked lists beside it, which put a
            // second copy of the scheduling answer on a different surface from
            // a caller's point of view; a caller that wants to know what can
            // start asks the projection for the Project it cares about.
            Ok(ResponseBody::SchedulerSweep {
                expired: expired
                    .into_iter()
                    .map(|(task_id, fencing_token)| ExpiredLease {
                        task_id,
                        fencing_token,
                    })
                    .collect(),
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
            if current.id() != dispatch_id {
                // A foreign dispatch id never reaches a factory, and never
                // reads as a replay of this task's own run.
                return Err(dispatch_binding_refused());
            }
            if task_record.state() != &symbiote_domain::TaskState::Running {
                // A run whose response was lost is recovered by replaying its
                // command id, and the daemon's own contract on a disconnected
                // response is that the retry "recovers the durable receipt".
                // This command has no journaled receipt of its own — the run's
                // completion is filed under the worker-completion id — so the
                // durable evidence of it is the task's own state: a task that
                // is at or past `CompletionRequested` under THIS dispatch had
                // its run complete, and the outcome the caller is asking for
                // is exactly the one already recorded. Re-deriving it here is
                // not a claim: the run either filed completion under this
                // dispatch (so `completed` was true) or it returned an error
                // and left the task Running. Refusing instead would report the
                // replay of a finished run as if the run never happened.
                return match task_record.state() {
                    symbiote_domain::TaskState::CompletionRequested
                    | symbiote_domain::TaskState::Verifying
                    | symbiote_domain::TaskState::Completed => {
                        Ok(ResponseBody::WorkerRun(Box::new(WorkerRun {
                            task_id: task_id.clone(),
                            dispatch_id: current.id().clone(),
                            runtime: current.contract().profile().runtime,
                            completed: true,
                        })))
                    }
                    _ => Err(dispatch_binding_refused()),
                };
            }
            // Everything past the two request-identity preconditions above is
            // an execution attempt, and an attempt that refuses has to leave
            // the task somewhere honest. The failure is filed first and the
            // original refusal is returned unchanged, so the operator sees
            // why it failed and the record says it too.
            match run_started_dispatch_body(
                store,
                workers,
                inventory,
                principal,
                &task_record,
                task_id,
                at,
            ) {
                Ok(body) => Ok(body),
                Err(error) => Err(file_run_failure(
                    store,
                    principal,
                    inventory,
                    request,
                    &task_record,
                    task_id,
                    error,
                )?),
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
        Operation::ValidateChangeStream {
            task_id,
            verification_run,
        } => {
            // The head is observed, not accepted. A validation of a commit
            // nobody looked at is a validation of nothing, so the caller
            // names the run and the Host reads the provisioned worktree
            // itself — and if the work is not there, or is somewhere else,
            // the stream stays where it was and the refusal says which.
            let task_record = store.task(task_id).map_err(storage_error)?;
            let stream = store
                .change_stream(task_record.stream_id())
                .map_err(storage_error)?;
            let run = symbiote_domain::VerificationRunId::new(verification_run.as_str())
                .map_err(|_| ProtocolError::new(ErrorCode::InvalidRequest))?;
            let dispatch = task_record
                .current_dispatch()
                .ok_or_else(dispatch_binding_refused)?;
            let reservation_base = workers.reservation_base().map_err(worker_error)?;
            // The Host's own derivation, not a second spelling of it: the
            // worktree a run provisioned is the worktree validation reads.
            let derived = crate::runner::derived_worktree(
                task_record.project_id(),
                task_record.root_id(),
                task_record.stream_id(),
            )
            .map_err(worker_error)?;
            let worktree = derived.worktree_path(&reservation_base);
            let mut git = workers.git();
            let head = symbiote_repo::observe_head(&mut git, &worktree).map_err(|error| {
                storage_error(StoreError::Integrity(format!(
                    "stream worktree head is not observable: {error:?}"
                )))
            })?;
            let observed = symbiote_domain::CommitSha::new(head.commit.as_str())
                .map_err(|_| ProtocolError::new(ErrorCode::Internal))?;
            let at = start_timestamp(store, request)?;
            let command = symbiote_domain::StreamCommand {
                id: request.command_id.clone(),
                expected_revision: stream.revision(),
                host_id: inventory.host_id().clone(),
                at,
                action: symbiote_domain::StreamAction::Validate {
                    head: observed,
                    target: stream.target().clone(),
                    run,
                },
            };
            store
                .apply_stream(
                    task_record.stream_id(),
                    command,
                    dispatch.contract().effective_access(),
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::BeginTaskVerification { task_id } => {
            // There is a dispatch to verify: the same refusal a run gives for
            // a task that is not running under one, because a task with no
            // dispatch has no contract whose gates could be covered.
            let task_record = store.task(task_id).map_err(storage_error)?;
            task_record
                .current_dispatch()
                .ok_or_else(dispatch_binding_refused)?;
            let command = symbiote_domain::TaskCommand {
                id: request.command_id.clone(),
                expected_revision: task_record.revision(),
                actor: Actor::Host(inventory.host_id().clone()),
                at: start_timestamp(store, request)?,
                action: symbiote_domain::TaskAction::BeginVerification,
            };
            store
                .apply_task(task_id, command)
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::CompleteTask {
            task_id,
            gates,
            reviewer,
        } => {
            // The caller claims which gates passed; the Host binds each claim
            // to this dispatch, this stream's head and target and this Host's
            // identity, and the domain refuses a claim that does not cover
            // the contract. Neither side gets the other's half: a Host that
            // invented the gates would be filing a runtime-local success
            // flag as evidence, and a caller that supplied the whole row
            // would be filing a claim the Host never checked.
            let task_record = store.task(task_id).map_err(storage_error)?;
            let dispatch = task_record
                .current_dispatch()
                .ok_or_else(dispatch_binding_refused)?;
            let stream = store
                .change_stream(task_record.stream_id())
                .map_err(storage_error)?;
            let at = start_timestamp(store, request)?;
            let this_host = inventory.host_id().clone();
            let run =
                symbiote_domain::VerificationRunId::new(format!("verify-{}", task_id.as_str()))
                    .map_err(|_| ProtocolError::new(ErrorCode::Internal))?;
            let artifact = symbiote_domain::ArtifactId::new(format!(
                "artifact-{}",
                task_record.stream_id().as_str()
            ))
            .map_err(|_| ProtocolError::new(ErrorCode::Internal))?;
            let mut evidence = Vec::new();
            for gate in gates {
                let id = symbiote_domain::EvidenceId::new(format!(
                    "evidence-{}-{}",
                    task_id.as_str(),
                    serde_json::to_string(gate)
                        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?
                        .replace('"', "")
                ))
                .map_err(|_| ProtocolError::new(ErrorCode::Internal))?;
                evidence.push(symbiote_domain::VerificationEvidence {
                    id,
                    run_id: run.clone(),
                    artifact_id: artifact.clone(),
                    project_id: task_record.project_id().clone(),
                    task_id: task_id.clone(),
                    dispatch_id: dispatch.id().clone(),
                    stream_id: task_record.stream_id().clone(),
                    gate: *gate,
                    outcome: symbiote_domain::VerificationOutcome::Passed,
                    head: stream.head().clone(),
                    target: stream.target().clone(),
                    verified_by: this_host.clone(),
                    reviewer: if *gate == symbiote_domain::Gate::IndependentReview {
                        reviewer.clone()
                    } else {
                        None
                    },
                    recorded_at: at,
                });
            }
            let command = symbiote_domain::TaskCommand {
                id: request.command_id.clone(),
                expected_revision: task_record.revision(),
                actor: Actor::Host(this_host.clone()),
                at,
                action: symbiote_domain::TaskAction::Complete {
                    stream: Box::new(stream),
                    evidence,
                },
            };
            store
                .apply_task(task_id, command)
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::IntegrateChangeStream { task_id } => {
            // A completed Task's work is on a branch in a worktree. Nothing
            // else moves it: without this the canonical state says the work
            // is done and the repository says otherwise, and the two
            // disagree for the life of the store. The Host merges rather than
            // the caller, because "the branch is the stream's" and "the root
            // is where it lands" are both facts only the Host can hold.
            let task_record = store.task(task_id).map_err(storage_error)?;
            let dispatch = task_record
                .current_dispatch()
                .ok_or_else(dispatch_binding_refused)?;
            let stream = store
                .change_stream(task_record.stream_id())
                .map_err(storage_error)?;
            let this_host = inventory.host_id().clone();
            let root = store.root(task_record.root_id()).map_err(storage_error)?;
            let checkout = root
                .host_paths
                .get(&this_host)
                .ok_or_else(|| worker_error(crate::runner::RunnerError::NoHostPath))?;
            let mut git = workers.git();
            // The Root must be where the stream recorded it, before and
            // after. A root that moved under the stream is not a merge this
            // Host should attempt, and refusing before the merge leaves both
            // sides where they were rather than half-applied.
            let before = symbiote_repo::observe_head(&mut git, std::path::Path::new(checkout))
                .map_err(|_| worker_error(crate::runner::RunnerError::NoHostPath))?;
            let target = symbiote_domain::CommitSha::new(before.commit.as_str())
                .map_err(|_| ProtocolError::new(ErrorCode::Internal))?;
            if target != *stream.target() {
                return Err(ProtocolError {
                    code: ErrorCode::FailedPrecondition,
                    message: "root checkout is not at the stream's recorded target".into(),
                });
            }
            // A fast-forward only: a merge the Host would have to resolve is a
            // merge the Host has no authority to decide.
            symbiote_repo::GitExecutor::run(
                &mut git,
                std::path::Path::new(checkout),
                &["merge", "--ff-only", stream.branch.as_str()],
            )
            .map_err(|error| ProtocolError {
                code: ErrorCode::FailedPrecondition,
                message: format!("fast-forward integration refused: {error:?}"),
            })?;
            let after = symbiote_repo::observe_head(&mut git, std::path::Path::new(checkout))
                .map_err(|_| worker_error(crate::runner::RunnerError::NoHostPath))?;
            let head = symbiote_domain::CommitSha::new(after.commit.as_str())
                .map_err(|_| ProtocolError::new(ErrorCode::Internal))?;
            if head != *stream.head() {
                return Err(ProtocolError {
                    code: ErrorCode::FailedPrecondition,
                    message: "integrated checkout is not at the stream's recorded head".into(),
                });
            }
            let command = symbiote_domain::StreamCommand {
                id: request.command_id.clone(),
                expected_revision: stream.revision(),
                host_id: this_host,
                at: start_timestamp(store, request)?,
                action: symbiote_domain::StreamAction::Integrate {
                    head: stream.head().clone(),
                    target: stream.target().clone(),
                },
            };
            store
                .apply_stream(
                    task_record.stream_id(),
                    command,
                    dispatch.contract().effective_access(),
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::ReleaseChangeStream { task_id } => {
            // The last step of a stream's life: the work landed and the
            // checkout that held it is reclaimed. Without it every run left a
            // full git worktree on the operator's disk forever, and
            // `symbiote_worktrees::release` — a policy the crate designed and
            // documented — had no production caller at all.
            let task_record = store.task(task_id).map_err(storage_error)?;
            let dispatch = task_record
                .current_dispatch()
                .ok_or_else(dispatch_binding_refused)?;
            let stream = store
                .change_stream(task_record.stream_id())
                .map_err(storage_error)?;
            // The Host's own derivation, and the domain refuses a stream that
            // is not `Integrated` — the same check the worktree removal's own
            // safety depends on, made once in the layer that owns the rule.
            let reservation_base = workers.reservation_base().map_err(worker_error)?;
            let derived = crate::runner::derived_worktree(
                task_record.project_id(),
                task_record.root_id(),
                task_record.stream_id(),
            )
            .map_err(worker_error)?;
            if derived.worktree_id != *stream.worktree_id() {
                return Err(ProtocolError {
                    code: ErrorCode::FailedPrecondition,
                    message: "recorded worktree identity is not the one this Host derives".into(),
                });
            }
            // The same rule the domain applies, asked BEFORE anything is
            // removed rather than after. `StreamAction::Release` is still the
            // authority for the write; this ordering is what stops a caller
            // who asks twice from running the irreversible half twice and
            // only then hearing that the stream is already released.
            if stream.state() != &symbiote_domain::StreamState::Integrated {
                return Err(storage_error(StoreError::Domain(
                    symbiote_domain::DomainError::StreamNotReady,
                )));
            }
            let checkout = store
                .root(task_record.root_id())
                .map_err(storage_error)?
                .host_paths
                .get(inventory.host_id())
                .ok_or_else(|| worker_error(crate::runner::RunnerError::NoHostPath))?
                .clone();
            let mut git = workers.git();
            let head = symbiote_repo::observe_head(&mut git, std::path::Path::new(&checkout))
                .map_err(|error| {
                    storage_error(StoreError::Integrity(format!(
                        "root checkout head is not observable: {error:?}"
                    )))
                })?;
            let descendant = symbiote_domain::CommitSha::new(head.commit.as_str())
                .map_err(|_| ProtocolError::new(ErrorCode::Internal))?;
            // The evidence, checked before anything is removed: the stream's
            // recorded head must still be reachable from the root. Equality
            // is not required — the root may have moved on, and a descendant
            // is exactly as good: the integrated commit survives either way.
            if !symbiote_repo::is_ancestor(
                &mut git,
                std::path::Path::new(&checkout),
                stream.head().as_str(),
                descendant.as_str(),
            )
            .map_err(|error| {
                storage_error(StoreError::Integrity(format!(
                    "integrated commit ancestry is not observable: {error:?}"
                )))
            })? {
                return Err(ProtocolError {
                    code: ErrorCode::FailedPrecondition,
                    message: "the stream's head is not reachable from the root checkout".into(),
                });
            }
            reclaim_worktree(
                &mut git,
                std::path::Path::new(&checkout),
                &symbiote_worktrees::Reservation {
                    derived,
                    base: reservation_base,
                },
                // No `--force`: git itself refuses a checkout holding
                // uncommitted or untracked content, so this cannot destroy
                // work the branch does not already hold.
                symbiote_repo::RemovalForce::Refuse,
            )?;
            let command = symbiote_domain::StreamCommand {
                id: request.command_id.clone(),
                expected_revision: stream.revision(),
                host_id: inventory.host_id().clone(),
                at: start_timestamp(store, request)?,
                action: symbiote_domain::StreamAction::Release,
            };
            store
                .apply_stream(
                    task_record.stream_id(),
                    command,
                    dispatch.contract().effective_access(),
                )
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::AbandonChangeStream { task_id } => {
            // The other end of the cleanup policy, and the only operation in
            // the product that destroys work. A run that failed leaves commits
            // and uncommitted files in a location nothing else can reclaim: the
            // release path needs an integrated stream, and the stream of a
            // failed run never becomes one. Without this, every failure is a
            // permanent reservation and the operator's only answer is to edit
            // the filesystem by hand.
            //
            // The caller's word is the evidence the Change Stream contract asks
            // for — separate evidence and authorized confirmation — and it is
            // confirmed twice over: the kind is dangerous, and the branch is
            // never deleted, so every commit survives this. Only the checkout
            // goes, and only with it uncommitted files.
            let task_record = store.task(task_id).map_err(storage_error)?;
            let stream = store
                .change_stream(task_record.stream_id())
                .map_err(storage_error)?;
            // Which states may be abandoned, asked BEFORE anything is removed
            // rather than after. `StreamAction::Cancel` is still the authority
            // for the write; this ordering is what stops a caller who asks
            // twice from running the irreversible half twice.
            //
            // An integrated stream's work landed, so abandoning it would be a
            // second, weaker way to do what `release_change_stream` does — and
            // that one refuses a dirty checkout instead of forcing it. A
            // cancelled or already released stream is terminal, and there is
            // nothing left in it to abandon.
            match stream.state() {
                symbiote_domain::StreamState::Integrated => {
                    return Err(ProtocolError {
                        code: ErrorCode::FailedPrecondition,
                        message: "an integrated stream is released, not abandoned".into(),
                    });
                }
                symbiote_domain::StreamState::Released
                | symbiote_domain::StreamState::Cancelled => {
                    return Err(storage_error(StoreError::Domain(
                        symbiote_domain::DomainError::IllegalTransition,
                    )));
                }
                symbiote_domain::StreamState::Active
                | symbiote_domain::StreamState::NeedsRevalidation
                | symbiote_domain::StreamState::Collided
                | symbiote_domain::StreamState::Validated => {}
            }
            let reservation_base = workers.reservation_base().map_err(worker_error)?;
            let derived = crate::runner::derived_worktree(
                task_record.project_id(),
                task_record.root_id(),
                task_record.stream_id(),
            )
            .map_err(worker_error)?;
            if derived.worktree_id != *stream.worktree_id() {
                return Err(ProtocolError {
                    code: ErrorCode::FailedPrecondition,
                    message: "recorded worktree identity is not the one this Host derives".into(),
                });
            }
            let checkout = store
                .root(task_record.root_id())
                .map_err(storage_error)?
                .host_paths
                .get(inventory.host_id())
                .ok_or_else(|| worker_error(crate::runner::RunnerError::NoHostPath))?
                .clone();
            // The cancel is journalled and audited like every other stream
            // transition, under the same access the task's dispatch carried. A
            // task with no dispatch has no snapshot to record, and a stream
            // nobody ever dispatched has nothing to abandon.
            //
            // Asked HERE, before the removal below, and that ordering is the
            // point: this arm is the only one that destroys work, so a refusal
            // it has already acted on would leave the operator with a refused
            // call and a deleted checkout. The state check above is the same
            // argument, and it is why both precede the irreversible half.
            let access = match task_record.current_dispatch() {
                Some(dispatch) => dispatch.contract().effective_access().clone(),
                None => {
                    return Err(dispatch_binding_refused());
                }
            };
            let mut git = workers.git();
            reclaim_worktree(
                &mut git,
                std::path::Path::new(&checkout),
                &symbiote_worktrees::Reservation {
                    derived,
                    base: reservation_base,
                },
                // `--force` here and nowhere else: the caller has authorized
                // discarding the uncommitted contents of a checkout whose work
                // landed nowhere.
                symbiote_repo::RemovalForce::Allow,
            )?;
            let command = symbiote_domain::StreamCommand {
                id: request.command_id.clone(),
                expected_revision: stream.revision(),
                host_id: inventory.host_id().clone(),
                at: start_timestamp(store, request)?,
                action: symbiote_domain::StreamAction::Cancel,
            };
            store
                .apply_stream(task_record.stream_id(), command, &access)
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::RecoverTask { task_id } => {
            // The way out of a failure, and the only one: `Failed` and
            // `Interrupted` are terminal for the run that produced them, and
            // the lifecycle reaches `Ready` only through this. The dispatch
            // binding is dropped by the domain, so the recovered task is
            // dispatched afresh rather than resuming a run that is not
            // running.
            let task_record = store.task(task_id).map_err(storage_error)?;
            let at = start_timestamp(store, request)?;
            let command = symbiote_domain::TaskCommand {
                id: request.command_id.clone(),
                expected_revision: task_record.revision(),
                actor: Actor::Host(inventory.host_id().clone()),
                at,
                action: symbiote_domain::TaskAction::Recover,
            };
            store
                .apply_task(task_id, command)
                .map(receipt)
                .map_err(storage_error)
        }
        Operation::InterruptTask { task_id, reason } => {
            // The case no run can file for itself. A refused run observes its
            // own failure; a Host that died with a task `Running` has no run
            // left to observe anything, and the task would otherwise sit there
            // for the life of the store claiming to be executed. The Host
            // cannot tell a crash from an operator walking away, so the
            // reason is the caller's — the only party that knows which.
            //
            // The lease goes with it, for the same reason a failure releases
            // one: it is the claim that a live owner holds this task, and
            // there is no live owner.
            let task_record = store.task(task_id).map_err(storage_error)?;
            let at = start_timestamp(store, request)?;
            let command = symbiote_domain::TaskCommand {
                id: request.command_id.clone(),
                expected_revision: task_record.revision(),
                actor: Actor::Host(inventory.host_id().clone()),
                at,
                action: symbiote_domain::TaskAction::Interrupt {
                    reason: reason.clone(),
                },
            };
            store.apply_task(task_id, command).map_err(storage_error)?;
            if let Ok(lease) = store.task_lease(task_id) {
                let lease_command = symbiote_domain::CommandId::new(format!(
                    "{}-interrupted-lease",
                    request.command_id.as_str()
                ))
                .map_err(|_| ProtocolError::new(ErrorCode::Internal))?;
                store
                    .release_lease(
                        lease_command,
                        task_id.clone(),
                        lease.dispatch_id.clone(),
                        lease.fencing_token,
                        principal.user_id().clone(),
                        now_timestamp()?,
                    )
                    .map_err(storage_error)?;
            }
            // A receipt, not a task: the caller asked the run to end, and the
            // task's new state is what it reads next.
            store
                .task(task_id)
                .map(|task| ResponseBody::Task(Box::new(task)))
                .map_err(storage_error)
        }
        Operation::GetDispatchPreparation { task_id } => store
            .dispatch_preparation(task_id)
            .map(|preparation| ResponseBody::DispatchPreparation(Box::new(preparation)))
            .map_err(storage_error),
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
            // Every candidate the binding names, not only the primary: the
            // declaration for each of their own profiles, observed with this
            // Host's identity and a fresh window, in the binding's own order.
            // The pairing is checked where the report is built rather than
            // assumed here, so one binding's declarations cannot be read onto
            // another's candidates.
            let mut observed = Vec::with_capacity(1 + binding.fallbacks.len());
            for candidate in std::iter::once(&binding.primary).chain(&binding.fallbacks) {
                observed.push(match workers.declared_runtime_for(&candidate.profile) {
                    Some(declared) => Some(
                        declared
                            .observe(inventory.host_id(), now)
                            .map_err(|_| ProtocolError::new(ErrorCode::Internal))?,
                    ),
                    None => None,
                });
            }
            let observations =
                symbiote_workforce::CandidateObservations::for_configuration(&binding, &observed)
                    .ok_or_else(|| ProtocolError::new(ErrorCode::Internal))?;
            Ok(ResponseBody::BindingReadiness(Box::new(
                symbiote_workforce::assess_readiness(
                    &binding,
                    &team,
                    Some(&pulse),
                    &observations,
                    Some(&registration),
                    now,
                ),
            )))
        }
        Operation::GetHostPulse {} => inventory
            .pulse()
            .map(|pulse| ResponseBody::HostPulse(Box::new(pulse))),
        // The inventory is re-read from this Host's own private file and
        // re-validated whole on every read: the reader gets what this Host
        // published, or a refusal that names which of the ways it declined. It
        // is never partially served, and a stale record is served with its own
        // `expires_at` rather than filtered out here.
        Operation::GetRuntimeInventory {} => inventory
            .runtime_inventory()
            .map(|inventory| ResponseBody::RuntimeInventory(Box::new(inventory)))
            .map_err(inventory_refusal),
        Operation::GetCompatibilityDossier {
            project_id,
            binding_id,
        } => {
            let binding = store
                .get_binding(project_id, binding_id)
                .map_err(storage_error)?;
            // The runtime a start would use: the binding's own primary candidate.
            // A fallback is another pack's record, and asking about that binding
            // asks about that pack.
            let declared = workers
                .declared_runtime_for(&binding.primary.profile)
                .ok_or_else(|| dossier_refusal("no_runtime_declared"))?;
            let dossier = workers.dossier_for(declared).ok_or_else(|| {
                // One refusal, named for which of the two ways it is true: an
                // operator installed no record at all, or the one they installed
                // is another pack's. Both leave this Host holding nothing for
                // this runtime, and both say so.
                dossier_refusal(if workers.has_dossiers() {
                    "another_pack"
                } else {
                    "no_published_dossier"
                })
            })?;
            // The record is held before it is served: the SDK compares what the
            // document claims about itself with the runtime this Host actually
            // holds, and the refusal it names is the whole message. A record
            // that cannot be held is never answered with a partial one.
            let held = dossier
                .hold(declared)
                .map_err(|error| dossier_refusal(error.name()))?;
            Ok(ResponseBody::CompatibilityDossier(Box::new(
                DossierHolding {
                    // The response is owned, so the parsed record is copied into
                    // it; the Host keeps its own parsed copy for the next read
                    // rather than re-reading the file.
                    dossier: Box::new(dossier.clone()),
                    upstream_version: held.run.upstream_version.clone(),
                    platform: held.run.platform.clone(),
                },
            )))
        }
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
        Operation::ListProjects {} => {
            // The registry is scoped record by record, not by the connection:
            // `authorized_projects` applies the same per-Project Read rule
            // `GetProject` applies to one record. Order is the store's
            // registration order and is preserved.
            let projects = store.projects().map_err(storage_error)?;
            Ok(ResponseBody::Projects {
                projects: authorized_projects(principal, projects),
            })
        }
        Operation::Snapshot { project_id } => {
            // One consistent read: the Project's canonical records with the
            // cursor of the Project's own journal at the moment of the read.
            // `authorize` has already required this Principal's Project Read,
            // and every record here belongs to that Project.
            let snapshot = store.snapshot(project_id).map_err(storage_error)?;
            Ok(ResponseBody::Snapshot(Box::new(ProjectSnapshot {
                project: snapshot.project,
                roots: snapshot.roots,
                roles: snapshot.roles,
                tasks: snapshot.tasks,
                cursor: JournalCursor(snapshot.cursor),
            })))
        }
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
                            symbiote_store::EventPayload::TaskForeignLinksSet {
                                task_id,
                                project_id,
                                links,
                                actor,
                                at,
                            } => EventPayload::TaskForeignLinksSet {
                                task_id,
                                project_id,
                                links,
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
                            // The access snapshot the transition was accepted
                            // under stays in the store: it is the audit's
                            // evidence, not a fact a client can act on, and
                            // putting it on the wire would make it one.
                            symbiote_store::EventPayload::StreamChanged {
                                stream_id,
                                command,
                                stream,
                                ..
                            } => EventPayload::StreamChanged {
                                stream_id,
                                command: *command,
                                stream,
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

/// The execution half of `run_started_dispatch`, split out so every refusal
/// it can produce is recorded against the task before it reaches the caller.
///
/// It starts after the two request-identity preconditions — a foreign dispatch
/// id, and a task that is not `Running` — because those refuse the *request*,
/// not the run: nothing has been claimed and nothing has executed, so no
/// failure belongs to the task. Everything here has: the worktree is reserved,
/// a transport may be built, a worker may run. A refusal in this half that
/// left the task `Running` would be a task claiming to be executed with
/// nothing executing it, and no operation could ever move it out, because
/// nothing constructed a `TaskAction::Fail`.
fn run_started_dispatch_body(
    store: &mut Store,
    workers: &mut crate::runner::WorkerTransports,
    inventory: &mut crate::inventory::InventoryService,
    principal: &Principal,
    task_record: &symbiote_domain::Task,
    task_id: &TaskId,
    at: symbiote_domain::Timestamp,
) -> Result<ResponseBody, ProtocolError> {
    let current = task_record
        .current_dispatch()
        .ok_or_else(dispatch_binding_refused)?;
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
    let bound = crate::limits::bounds_for(current.contract().limits()).map_err(limit_unbounded)?;
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
            let shell_access =
                crate::runner::resolve_shell_access(store, current, at).map_err(worker_error)?;
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
            // The Host launches the sandboxed harness itself, under
            // the dispatch's declared ceiling: the bound is applied by
            // the launcher, not asserted by a transport, so the lane
            // cannot promise a bound it does not apply. The one
            // process this lane starts is the one the Host bounds.
            let harness_access = current.contract().effective_access().clone();
            let mut transport = workers
                .external_build(crate::runner::ExternalHarnessInputs {
                    root_id: &provisioned.root_id,
                    worktree: &provisioned.worktree,
                    host: &this_host,
                    project_id: task_record.project_id(),
                    role_id: &current.contract().binding().role_id,
                    profile_id: &current.contract().profile().id,
                    access: &harness_access,
                    user_id: principal.user_id(),
                    at,
                    bound,
                })
                .map_err(worker_error)?;
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

/// Reclaims a stream's reserved location: the git worktree if one is
/// registered there, and the reservation marker with it.
///
/// The two callers disagree only about `force` and about what evidence they
/// have proved before arriving here. Everything that decides *what is deleted*
/// lives in this one function, so the integrated path and the abandoned path
/// cannot drift apart in the only respect that matters.
///
/// The registration check is the part that is easy to get wrong. A
/// provisioning that refused before materializing leaves the reserved location
/// and its marker on disk with no git worktree behind them, and `worktree
/// remove` against a path git does not know refuses for a reason that has
/// nothing to do with work in the way — a refusal the caller must not read as
/// "someone's work is in there". Conversely, a directory that holds content
/// and is *not* a registered worktree is something this Host did not put
/// there, and `DeleteIfEmpty` refuses it rather than deleting a stranger's
/// files.
fn reclaim_worktree(
    git: &mut impl symbiote_repo::GitExecutor,
    checkout: &std::path::Path,
    reservation: &symbiote_worktrees::Reservation,
    force: symbiote_repo::RemovalForce,
) -> Result<(), ProtocolError> {
    let worktree = reservation.derived.worktree_path(&reservation.base);
    // The reclaim is complete when the marker is gone — that is the
    // reservation crate's own definition of released, and a directory left
    // behind without one is an abandoned location it explicitly never
    // reclaims. Asking again must therefore succeed rather than report
    // `NotFound`, or a caller whose FIRST attempt reclaimed the location and
    // then failed to record its stream transition is stuck: the retry would
    // refuse a filesystem error for work it had already done.
    //
    // Only an absent marker counts. A marker that exists and cannot be read
    // is a tamper signal, and it is not this function's to absorb.
    let marker = reservation
        .base
        .join(reservation.derived.project_id.as_str())
        .join(format!("{}.json", reservation.derived.worktree_id));
    if !marker.exists() {
        return Ok(());
    }
    let registered = symbiote_repo::registered_worktrees(git, checkout)
        .map_err(|error| {
            storage_error(StoreError::Integrity(format!(
                "registered worktrees are not observable: {error:?}"
            )))
        })?
        .contains(&worktree);
    if registered {
        symbiote_repo::remove_worktree(git, checkout, &worktree, force).map_err(|error| {
            ProtocolError {
                code: ErrorCode::FailedPrecondition,
                message: format!("worktree removal refused: {error:?}"),
            }
        })?;
    }
    match symbiote_worktrees::release(
        reservation,
        symbiote_worktrees::ReleasePolicy::DeleteIfEmpty,
    ) {
        Ok(symbiote_worktrees::Outcome::Deleted) => Ok(()),
        Ok(symbiote_worktrees::Outcome::Retained) => Err(ProtocolError {
            code: ErrorCode::FailedPrecondition,
            message: "worktree location was retained rather than released".into(),
        }),
        Err(error) => Err(ProtocolError {
            code: ErrorCode::FailedPrecondition,
            message: format!("worktree reservation release refused: {error:?}"),
        }),
    }
}

/// Records a run refusal against the task it refused to run, and hands the
/// original error back. A caller must not see success for a run that did not
/// happen, and a task must not stay `Running` for one.
///
/// The reason is the same content-free sentence the caller receives, so the
/// journal and the operator read one classification rather than two
/// vocabularies that can drift.
///
/// The command id is derived from the run's own, so a retry that fails the
/// same way files the same failure and the store replays it instead of
/// appending a second one.
fn file_run_failure(
    store: &mut Store,
    principal: &Principal,
    inventory: &crate::inventory::InventoryService,
    request: &Request,
    task_record: &symbiote_domain::Task,
    task_id: &TaskId,
    error: ProtocolError,
) -> Result<ProtocolError, ProtocolError> {
    // A task with no current dispatch is not `Running` — the domain already
    // proved that when the state was read — so there is nothing to fail.
    if task_record.current_dispatch().is_none() {
        return Ok(error);
    }
    let command_id =
        symbiote_domain::CommandId::new(format!("{}-run-failed", request.command_id.as_str()))
            .map_err(|_| ProtocolError::new(ErrorCode::Internal))?;
    let command = symbiote_domain::TaskCommand {
        id: command_id,
        expected_revision: task_record.revision(),
        actor: Actor::Host(inventory.host_id().clone()),
        at: now_timestamp()?,
        action: symbiote_domain::TaskAction::Fail {
            reason: error.message.clone(),
        },
    };
    store.apply_task(task_id, command).map_err(storage_error)?;
    // The lease too. A lease is the claim that a live owner is running this
    // task, and the run that held it just ended by refusing — so leaving it
    // held is a second thing that says a run is in progress when none is, and
    // it fences the task's own recovery: a re-dispatch is refused while the
    // old lease is held. Releasing it is the same claim withdrawn.
    //
    // A missing lease is not a failure: a run can refuse before a lease
    // exists, and there is then nothing to withdraw.
    if let Ok(lease) = store.task_lease(task_id) {
        let lease_command = symbiote_domain::CommandId::new(format!(
            "{}-run-failed-lease",
            request.command_id.as_str()
        ))
        .map_err(|_| ProtocolError::new(ErrorCode::Internal))?;
        store
            .release_lease(
                lease_command,
                task_id.clone(),
                lease.dispatch_id.clone(),
                lease.fencing_token,
                // The actor that acquired it. The lease audit attributes a
                // release the same way it attributed the acquisition, and the
                // acquisition was recorded against the authenticated caller.
                principal.user_id().clone(),
                now_timestamp()?,
            )
            .map_err(storage_error)?;
    }
    Ok(error)
}

#[cfg(test)]
mod storage_error_tests {
    use super::*;

    /// The refusal vocabulary the Host's storage mapping publishes. A store
    /// refusal that exists is a refusal a client can be told about: each entry
    /// here is a case that would otherwise be the generic `internal`, so a
    /// caller sees what was refused rather than that something was.
    #[test]
    fn every_named_store_refusal_maps_to_a_typed_client_error() {
        for (error, code) in [
            (StoreError::NotFound, ErrorCode::NotFound),
            (StoreError::InvalidForeignLink, ErrorCode::InvalidRequest),
            (StoreError::InvalidDependency, ErrorCode::InvalidRequest),
            (StoreError::InvalidLease, ErrorCode::InvalidRequest),
            (StoreError::InvalidPreparation, ErrorCode::InvalidRequest),
            (StoreError::InvalidProvider, ErrorCode::InvalidRequest),
            (StoreError::InvalidElevation, ErrorCode::InvalidRequest),
            (StoreError::InvalidBinding, ErrorCode::InvalidRequest),
            (StoreError::InvalidRoute, ErrorCode::InvalidRequest),
            (StoreError::InvalidTeam, ErrorCode::InvalidRequest),
            (StoreError::InvalidInitialState, ErrorCode::InvalidRequest),
            (StoreError::InvalidConsent, ErrorCode::InvalidRequest),
            (StoreError::RelationshipMismatch, ErrorCode::InvalidRequest),
            (StoreError::DependenciesUnresolved, ErrorCode::Conflict),
            (StoreError::ResourceExhausted, ErrorCode::ResourceExhausted),
            (
                StoreError::IdempotencyConflict,
                ErrorCode::IdempotencyConflict,
            ),
            (StoreError::TeamRevisionConflict, ErrorCode::StaleRevision),
            (
                StoreError::BindingRevisionConflict,
                ErrorCode::StaleRevision,
            ),
            (StoreError::AlreadyExists, ErrorCode::Conflict),
            (
                StoreError::LeaseConflict(symbiote_domain::LeaseError::NotHeld),
                ErrorCode::Conflict,
            ),
        ] {
            assert_eq!(
                storage_error(error).code,
                code,
                "the mapping must name this refusal rather than collapse it into internal"
            );
        }
    }
}

#[cfg(test)]
mod reclaim_tests {
    use super::*;
    use std::cell::Cell;
    use std::os::unix::fs::PermissionsExt;

    /// Answers `git worktree list --porcelain` with a fixed registration and
    /// accepts every removal, while counting the invocations it was asked for.
    /// The count is the interesting part: a reclaim of a location that is
    /// already released must not consult git at all, and a stub that counted
    /// zero is how that is proven rather than assumed.
    struct ScriptedGit {
        registered: Vec<std::path::PathBuf>,
        calls: Cell<usize>,
    }
    impl ScriptedGit {
        fn new(registered: Vec<std::path::PathBuf>) -> Self {
            Self {
                registered,
                calls: Cell::new(0),
            }
        }
    }
    impl symbiote_repo::GitExecutor for ScriptedGit {
        fn run(
            &mut self,
            _worktree: &std::path::Path,
            arguments: &[&str],
        ) -> Result<Vec<u8>, symbiote_repo::GitError> {
            self.calls.set(self.calls.get() + 1);
            if arguments.starts_with(&["worktree", "list"]) {
                let mut text = String::new();
                for path in &self.registered {
                    text.push_str(&format!("worktree {}\n", path.display()));
                    text.push_str("HEAD 0000000000000000000000000000000000000000\n\n");
                }
                return Ok(text.into_bytes());
            }
            Ok(Vec::new())
        }
    }

    fn base_directory(tag: &str) -> std::path::PathBuf {
        let base =
            std::env::temp_dir().join(format!("symbiote-reclaim-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        // The reservation crate refuses a base anybody else can write to, and
        // the mode is the operator's own, not a test convenience.
        std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o700)).unwrap();
        base
    }

    fn derived(stream: &str) -> symbiote_worktrees::Derived {
        crate::runner::derived_worktree(
            &symbiote_domain::ProjectId::new("staffing-demo").unwrap(),
            &symbiote_domain::RootId::new("staffing-root").unwrap(),
            &symbiote_domain::ChangeStreamId::new(stream).unwrap(),
        )
        .unwrap()
    }

    /// The retry a caller is forced into when the reclaim succeeded and the
    /// journal write that should have followed it did not: the same stream, the
    /// same location, asked a second time.
    ///
    /// Before the guard this reported
    ///
    ///     worktree reservation release refused: NotFound
    ///
    /// which is a filesystem complaint about work the caller had already done,
    /// and it is worse than that: the stream stayed `Active` forever, because
    /// the only operation that could cancel it refused for a marker it had
    /// itself deleted one call earlier. This is the exact refusal a crashed
    /// daemon leaves behind, and the crash path is where an operator meets it.
    #[test]
    fn a_reclaim_of_a_location_that_is_already_released_succeeds_again() {
        let base = base_directory("released");
        let derived = derived("reclaim-retry");
        let reservation = symbiote_worktrees::reserve(
            &derived,
            &base,
            symbiote_domain::Timestamp(1_700_000_000_000),
        )
        .unwrap();
        let location = derived.worktree_path(&base);
        let mut git = ScriptedGit::new(vec![location.clone()]);
        // The first attempt does the work: git is consulted, the location goes,
        // and the marker with it.
        reclaim_worktree(
            &mut git,
            std::path::Path::new("/nonexistent-root"),
            &reservation,
            symbiote_repo::RemovalForce::Allow,
        )
        .unwrap();
        assert!(!location.exists(), "the first reclaim removed the location");
        let marker = base
            .join("staffing-demo")
            .join(format!("{}.json", derived.worktree_id));
        assert!(!marker.exists(), "and its reservation");
        let calls = git.calls.get();
        assert!(calls > 0, "and it did consult git to do that");

        // The retry. Same reservation, same location, nothing left to do — and
        // no git call, because there is nothing for git to be asked.
        reclaim_worktree(
            &mut git,
            std::path::Path::new("/nonexistent-root"),
            &reservation,
            symbiote_repo::RemovalForce::Allow,
        )
        .unwrap();
        assert_eq!(
            git.calls.get(),
            calls,
            "a location that is already released is answered without consulting git"
        );
        std::fs::remove_dir_all(&base).ok();
    }

    /// The other direction of the same question: a stream nobody ever
    /// dispatched has a reserved location by identity and no reservation on
    /// disk, and an operator asking to abandon it is asking about a location
    /// that does not exist. That is a clean answer, not a `NotFound`.
    #[test]
    fn a_reclaim_of_a_location_that_was_never_reserved_succeeds() {
        let base = base_directory("never");
        let derived = derived("reclaim-never");
        let reservation = symbiote_worktrees::Reservation {
            derived,
            base: base.clone(),
        };
        let mut git = ScriptedGit::new(Vec::new());
        reclaim_worktree(
            &mut git,
            std::path::Path::new("/nonexistent-root"),
            &reservation,
            symbiote_repo::RemovalForce::Allow,
        )
        .unwrap();
        assert_eq!(git.calls.get(), 0, "nothing existed to ask git about");
        std::fs::remove_dir_all(&base).ok();
    }
}
