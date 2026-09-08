use std::time::{SystemTime, UNIX_EPOCH};
use symbiote_domain::{Actor, DomainError, Timestamp, WorkCommand, WorkError, WorkItem};
use symbiote_protocol::*;
use symbiote_store::{Store, StoreError};

fn storage_error(error: StoreError) -> ProtocolError {
    let code = match error {
        StoreError::NotFound => ErrorCode::NotFound,
        StoreError::AlreadyExists => ErrorCode::Conflict,
        StoreError::IdempotencyConflict => ErrorCode::IdempotencyConflict,
        StoreError::InvalidPage => ErrorCode::InvalidCursor,
        StoreError::InvalidInitialState
        | StoreError::RelationshipMismatch
        | StoreError::InvalidConsent => ErrorCode::InvalidRequest,
        StoreError::Domain(DomainError::RevisionConflict) => ErrorCode::StaleRevision,
        StoreError::Work(WorkError::RevisionConflict) => ErrorCode::StaleRevision,
        StoreError::Work(WorkError::IdempotencyConflict) => ErrorCode::IdempotencyConflict,
        StoreError::Work(WorkError::ResourceLimit) => ErrorCode::ResourceExhausted,
        StoreError::Work(_) | StoreError::UnclassifiedTask => ErrorCode::InvalidRequest,
        StoreError::Domain(_) => ErrorCode::InvalidRequest,
        StoreError::Sqlite(_) => ErrorCode::Unavailable,
        _ => ErrorCode::Internal,
    };
    ProtocolError::new(code)
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
    request: &Request,
) -> Result<ResponseBody, ProtocolError> {
    authorize(principal, request)?;
    match &request.operation {
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
                        _ => {}
                    }
                    Ok(JournalEvent {
                        sequence: event.sequence,
                        project_id: event.project_id,
                        command_id: event.command_id,
                        revision: event.revision,
                        payload: match event.payload {
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
                            symbiote_store::EventPayload::ProjectRegistered {
                                project,
                                roots,
                                roles,
                            } => EventPayload::ProjectRegistered {
                                project: *project,
                                roots,
                                roles,
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

pub fn handle(store: &mut Store, principal: &Principal, request: Request) -> (Response, bool) {
    let result = execute(store, principal, &request);
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
