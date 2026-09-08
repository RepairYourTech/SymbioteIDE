//! Internal trusted SQLite adapter. Transport authentication, external effects,
//! vault access and runtime dispatch are deliberately outside this service.
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::Path,
    time::Duration,
};
use symbiote_domain::*;
use symbiote_trust::ResourceConsent;
use symbiote_workforce::{BindingConfiguration, RouteDecision};
mod binding;
mod dependency;
mod lease;
mod route;
mod team;
mod work;

const APPLICATION_ID: i64 = 0x53594d42;
const DATABASE_VERSION: i64 = 8;
const MIGRATION_V2: &str = "CREATE TABLE resource_consents (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    role_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision IN (0,1)),
    body TEXT NOT NULL CHECK(json_valid(body)),
    FOREIGN KEY(role_id, project_id) REFERENCES roles(id, project_id)
) STRICT;
PRAGMA user_version = 2;";
pub const MAX_EVENT_PAGE: u32 = 256;

#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    Json(serde_json::Error),
    Domain(DomainError),
    UnsupportedVersion,
    Integrity(String),
    NotFound,
    AlreadyExists,
    IdempotencyConflict,
    InvalidInitialState,
    RelationshipMismatch,
    InvalidPage,
    InvalidConsent,
    Work(WorkError),
    UnclassifiedTask,
    InvalidTeam,
    TeamRevisionConflict,
    InvalidBinding,
    BindingRevisionConflict,
    InvalidRoute,
    InvalidDependency,
    DependenciesUnresolved,
    InvalidLease,
    LeaseConflict(symbiote_domain::LeaseError),
    ResourceExhausted,
}
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "SQLite operation failed: {e}"),
            Self::Json(e) => write!(f, "Stored contract is invalid: {e}"),
            Self::Domain(e) => write!(f, "Domain transition rejected: {e}"),
            Self::Integrity(message) => write!(f, "Storage integrity failure: {message}"),
            error => write!(f, "{error:?}"),
        }
    }
}
impl std::error::Error for StoreError {}
impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}
impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl From<DomainError> for StoreError {
    fn from(e: DomainError) -> Self {
        Self::Domain(e)
    }
}
impl From<WorkError> for StoreError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}
pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub sequence: u64,
    pub revision: Revision,
    pub replayed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum EventPayload {
    BindingReplaced {
        configuration: Box<BindingConfiguration>,
        expected_revision: Option<Revision>,
        actor: UserId,
        at: Timestamp,
    },
    TeamReplaced {
        team: Box<TeamConfiguration>,
        expected_revision: Option<Revision>,
        actor: UserId,
        at: Timestamp,
    },
    ProjectRegistered {
        project: Box<Project>,
        roots: Vec<Root>,
        roles: Vec<Role>,
    },
    TaskCreated {
        task: Box<Task>,
        stream: Box<ChangeStream>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin: Option<TaskOrigin>,
    },
    WorkItemCreated {
        item: Box<WorkItem>,
    },
    WorkItemChanged {
        project_id: ProjectId,
        work_id: WorkId,
        command: Box<WorkCommand>,
        item: Box<WorkItem>,
    },
    TaskOriginAssigned {
        task_id: TaskId,
        project_id: ProjectId,
        origin: TaskOrigin,
        actor: UserId,
        at: Timestamp,
    },
    TaskDependenciesSet {
        task_id: TaskId,
        project_id: ProjectId,
        edges: Vec<TaskDependencyEdge>,
        actor: UserId,
        at: Timestamp,
    },
    TaskLeased {
        lease: Box<symbiote_domain::TaskLease>,
        actor: UserId,
        at: Timestamp,
    },
    TaskChanged {
        task_id: TaskId,
        command: Box<TaskCommand>,
        task: Box<Task>,
    },
    WorkRouted {
        decision: Box<RouteDecision>,
        actor: UserId,
        at: Timestamp,
    },
    ResourceConsentRecorded {
        consent: Box<ResourceConsent>,
    },
    ResourceConsentRevoked {
        consent: Box<ResourceConsent>,
        revoked_by: UserId,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredEvent {
    pub sequence: u64,
    pub project_id: ProjectId,
    pub command_id: CommandId,
    pub revision: Revision,
    pub payload: EventPayload,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventPage {
    pub events: Vec<StoredEvent>,
    pub next_cursor: u64,
    pub has_more: bool,
}

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if !metadata.is_file() => {
                return Err(StoreError::Integrity(
                    "database path must be a regular file, never a symlink".into(),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(StoreError::Integrity(error.to_string())),
        }
        Self::initialize(Connection::open(path)?)
    }
    pub fn memory() -> Result<Self> {
        Self::initialize(Connection::open_in_memory()?)
    }

    fn initialize(mut connection: Connection) -> Result<Self> {
        // Older bundles can corrupt WAL during concurrent checkpoint/write races.
        // This adapter deliberately supports the fixed mainline, not backport detection.
        if rusqlite::version_number() < 3_051_003 {
            return Err(StoreError::UnsupportedVersion);
        }
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 = transaction.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let application: i64 =
            transaction.pragma_query_value(None, "application_id", |r| r.get(0))?;
        if version == 0 && application == 0 {
            let tables: i64 = transaction.query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
                [],
                |r| r.get(0),
            )?;
            if tables != 0 {
                return Err(StoreError::UnsupportedVersion);
            }
            transaction.execute_batch(include_str!("schema.sql"))?;
            #[cfg(test)]
            tests::fault_boundary("migration_before_commit");
        } else if !(1..=DATABASE_VERSION).contains(&version) || application != APPLICATION_ID {
            return Err(StoreError::UnsupportedVersion);
        }
        if version < 2 {
            transaction.execute_batch(MIGRATION_V2)?;
        }
        if version < 3 {
            transaction.execute_batch(work::MIGRATION_V3)?;
        }
        if version < 4 {
            transaction.execute_batch(team::MIGRATION_V4)?;
        }
        if version < 5 {
            transaction.execute_batch(binding::MIGRATION_V5)?;
        }
        if version < 6 {
            transaction.execute_batch(route::MIGRATION_V6)?;
        }
        if version < 7 {
            transaction.execute_batch(dependency::MIGRATION_V7)?;
        }
        if version < 8 {
            transaction.execute_batch(lease::MIGRATION_V8)?;
        }
        // Refuse corrupt input before committing any schema migration. A failed
        // audit must roll back the version and schema as well as record changes.
        audit_connection(&transaction)?;
        transaction.commit()?;
        // WAL remains local to this bounded adapter; it is not the System Graph.
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        let store = Self { connection };
        store.integrity_check()?;
        Ok(store)
    }

    pub fn sqlite_version() -> &'static str {
        rusqlite::version()
    }

    /// Recover the authority-assigned creation time when rebuilding a retry's
    /// identical draft. This is not a general command-payload lookup API.
    pub fn registration_timestamp(&self, command_id: &CommandId) -> Result<Option<Timestamp>> {
        let body: Option<String> = self
            .connection
            .query_row(
                "SELECT payload FROM journal WHERE command_id=?1",
                [command_id.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        match body {
            None => Ok(None),
            Some(body) => match serde_json::from_str::<EventPayload>(&body)? {
                EventPayload::ProjectRegistered { project, .. } => {
                    Ok(Some(project.provenance.created_at))
                }
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }

    pub fn register_project(
        &mut self,
        command_id: CommandId,
        project: Project,
        roots: Vec<Root>,
        roles: Vec<Role>,
    ) -> Result<Receipt> {
        let payload = EventPayload::ProjectRegistered {
            project: Box::new(project.clone()),
            roots: roots.clone(),
            roles: roles.clone(),
        };
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        if project.revision != Revision(0)
            || project.disposition != RecordDisposition::Active
            || roots.iter().any(|r| r.revision != Revision(0))
            || roles.iter().any(|r| r.revision != Revision(0))
        {
            return Err(StoreError::InvalidInitialState);
        }
        let root_ids: BTreeSet<_> = roots.iter().map(|r| r.id.clone()).collect();
        let role_ids: BTreeSet<_> = roles.iter().map(|r| r.id.clone()).collect();
        if roots.is_empty()
            || root_ids.len() != roots.len()
            || role_ids.len() != roles.len()
            || project.roots != root_ids
            || !role_ids.contains(&project.lead)
            || roots.iter().any(|r| r.project_id != project.id)
            || roles.iter().any(|r| r.project_id != project.id)
        {
            return Err(StoreError::RelationshipMismatch);
        }
        if exists(
            &transaction,
            "SELECT 1 FROM projects WHERE id=?1",
            project.id.as_str(),
        )? {
            return Err(StoreError::AlreadyExists);
        }
        for root in &roots {
            if exists(
                &transaction,
                "SELECT 1 FROM roots WHERE id=?1",
                root.id.as_str(),
            )? {
                return Err(StoreError::AlreadyExists);
            }
        }
        for role in &roles {
            if exists(
                &transaction,
                "SELECT 1 FROM roles WHERE id=?1",
                role.id.as_str(),
            )? {
                return Err(StoreError::AlreadyExists);
            }
        }
        transaction.execute(
            "INSERT INTO projects(id, revision, lead_id, body) VALUES (?1,0,?2,?3)",
            params![
                project.id.as_str(),
                project.lead.as_str(),
                serde_json::to_string(&project)?
            ],
        )?;
        for root in &roots {
            transaction.execute(
                "INSERT INTO roots(id,project_id,body) VALUES (?1,?2,?3)",
                params![
                    root.id.as_str(),
                    project.id.as_str(),
                    serde_json::to_string(root)?
                ],
            )?;
        }
        for role in &roles {
            transaction.execute(
                "INSERT INTO roles(id,project_id,body) VALUES (?1,?2,?3)",
                params![
                    role.id.as_str(),
                    project.id.as_str(),
                    serde_json::to_string(role)?
                ],
            )?;
        }
        #[cfg(test)]
        tests::fault_boundary("state_before_journal");
        let receipt = append(
            &transaction,
            &project.id,
            &command_id,
            Revision(0),
            &request,
            &payload,
        )?;
        #[cfg(test)]
        tests::fault_boundary("journal_before_commit");
        transaction.commit()?;
        #[cfg(test)]
        tests::fault_boundary("after_commit");
        Ok(receipt)
    }

    pub fn create_task(
        &mut self,
        command_id: CommandId,
        task: Task,
        stream: ChangeStream,
        origin: TaskOrigin,
    ) -> Result<Receipt> {
        let payload = EventPayload::TaskCreated {
            task: Box::new(task.clone()),
            stream: Box::new(stream.clone()),
            origin: Some(origin.clone()),
        };
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        work::validate_origin(&transaction, task.project_id(), &origin)?;
        if task.revision() != Revision(0)
            || task.state() != &TaskState::Ready
            || !task.history().is_empty()
            || stream.revision() != Revision(0)
            || stream.state() != &StreamState::Active
            || !stream.history().is_empty()
        {
            return Err(StoreError::InvalidInitialState);
        }
        if stream.id() != task.stream_id()
            || stream.project_id() != task.project_id()
            || stream.root_id() != task.root_id()
            || stream.tasks() != &BTreeSet::from([task.id().clone()])
        {
            return Err(StoreError::RelationshipMismatch);
        }
        if !exists(
            &transaction,
            "SELECT 1 FROM projects WHERE id=?1",
            task.project_id().as_str(),
        )? {
            return Err(StoreError::NotFound);
        }
        for (query, id) in [
            (
                "SELECT project_id FROM roots WHERE id=?1",
                task.root_id().as_str(),
            ),
            (
                "SELECT project_id FROM roles WHERE id=?1",
                task.role_id().as_str(),
            ),
        ] {
            let owner: Option<String> = transaction
                .query_row(query, [id], |r| r.get(0))
                .optional()?;
            if owner.as_deref() != Some(task.project_id().as_str()) {
                return Err(StoreError::RelationshipMismatch);
            }
        }
        if exists(
            &transaction,
            "SELECT 1 FROM tasks WHERE id=?1",
            task.id().as_str(),
        )? || exists(
            &transaction,
            "SELECT 1 FROM streams WHERE id=?1",
            stream.id().as_str(),
        )? {
            return Err(StoreError::AlreadyExists);
        }
        if exists(
            &transaction,
            "SELECT 1 FROM streams WHERE worktree_id=?1",
            stream.worktree.as_str(),
        )? || transaction
            .query_row(
                "SELECT 1 FROM streams WHERE root_id=?1 AND branch=?2",
                params![task.root_id().as_str(), stream.branch],
                |_| Ok(()),
            )
            .optional()?
            .is_some()
        {
            return Err(StoreError::AlreadyExists);
        }
        if let StreamLineage::Stacked {
            parent,
            parent_head,
        } = stream.lineage()
        {
            let parent_stream = read_stream(&transaction, parent)?;
            if parent_stream.project_id() != task.project_id()
                || parent_stream.root_id() != task.root_id()
                || parent_stream.head() != parent_head
            {
                return Err(StoreError::RelationshipMismatch);
            }
        }
        transaction.execute("INSERT INTO streams(id,project_id,root_id,worktree_id,branch,body) VALUES (?1,?2,?3,?4,?5,?6)",
            params![stream.id().as_str(), task.project_id().as_str(), task.root_id().as_str(), stream.worktree.as_str(), stream.branch, serde_json::to_string(&stream)?])?;
        transaction.execute("INSERT INTO tasks(id,project_id,root_id,role_id,stream_id,revision,body) VALUES (?1,?2,?3,?4,?5,0,?6)",
            params![task.id().as_str(), task.project_id().as_str(), task.root_id().as_str(), task.role_id().as_str(), task.stream_id().as_str(), serde_json::to_string(&task)?])?;
        work::insert_origin(&transaction, task.id(), task.project_id(), &origin)?;
        let receipt = append(
            &transaction,
            task.project_id(),
            &command_id,
            task.revision(),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// Internal trusted Host boundary. The transport must authenticate/authorize
    /// the caller before constructing the Actor; this method does not authenticate.
    pub fn apply_task(&mut self, task_id: &TaskId, command: TaskCommand) -> Result<Receipt> {
        let request = mutation_request(task_id, &command)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command.id, &request)? {
            return Ok(receipt);
        }
        let mut task = read_task(&transaction, task_id)?;
        if matches!(command.action, TaskAction::Start { .. })
            && work::read_origin(&transaction, task.project_id(), task_id)?.is_none()
        {
            return Err(StoreError::UnclassifiedTask);
        }
        if let TaskAction::Complete { stream, .. } = &command.action {
            if **stream != read_stream(&transaction, task.stream_id())? {
                return Err(StoreError::RelationshipMismatch);
            }
            // Completion never bypasses unresolved blocking dependencies.
            dependency::completion_gate(&transaction, task.project_id(), task_id)?;
        }
        let old_revision = task.revision();
        task.apply(command.clone())?;
        let updated = transaction.execute(
            "UPDATE tasks SET revision=?1,body=?2 WHERE id=?3 AND revision=?4",
            params![
                sql_revision(task.revision())?,
                serde_json::to_string(&task)?,
                task_id.as_str(),
                sql_revision(old_revision)?
            ],
        )?;
        if updated != 1 {
            return Err(StoreError::Domain(DomainError::RevisionConflict));
        }
        let payload = EventPayload::TaskChanged {
            task_id: task_id.clone(),
            command: Box::new(command.clone()),
            task: Box::new(task.clone()),
        };
        let receipt = append(
            &transaction,
            task.project_id(),
            &command.id,
            task.revision(),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// Rebuild server-assigned timestamps for identical command retries.
    pub fn consent_command_timestamp(&self, command_id: &CommandId) -> Result<Option<Timestamp>> {
        let body: Option<String> = self
            .connection
            .query_row(
                "SELECT payload FROM journal WHERE command_id=?1",
                [command_id.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        match body {
            None => Ok(None),
            Some(body) => match serde_json::from_str::<EventPayload>(&body)? {
                EventPayload::ResourceConsentRecorded { consent } => Ok(Some(consent.issued_at)),
                EventPayload::ResourceConsentRevoked { consent, .. } => consent
                    .revoked_at
                    .map(Some)
                    .ok_or(StoreError::InvalidConsent),
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }

    /// Internal trusted service: authentication is the Host's responsibility.
    /// Profile and Host ids are bound in the snapshot but have no registry in this adapter.
    pub fn record_resource_consent(
        &mut self,
        command_id: CommandId,
        consent: ResourceConsent,
    ) -> Result<Receipt> {
        consent.validate().map_err(|_| StoreError::InvalidConsent)?;
        if consent.id != command_id || consent.revoked_at.is_some() {
            return Err(StoreError::InvalidConsent);
        }
        let payload = EventPayload::ResourceConsentRecorded {
            consent: Box::new(consent.clone()),
        };
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        validate_consent_relationships(&transaction, &consent)?;
        transaction.execute("INSERT INTO resource_consents(id,project_id,role_id,revision,body) VALUES (?1,?2,?3,0,?4)", params![consent.id.as_str(),consent.snapshot.project_id.as_str(),consent.snapshot.role_id.as_str(),serde_json::to_string(&consent)?])?;
        let receipt = append(
            &transaction,
            &consent.snapshot.project_id,
            &command_id,
            Revision(0),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    pub fn revoke_resource_consent(
        &mut self,
        command_id: CommandId,
        project_id: &ProjectId,
        consent_id: &CommandId,
        user_id: &UserId,
        at: Timestamp,
    ) -> Result<Receipt> {
        let request = revocation_request(project_id, consent_id, user_id, at)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        let mut consent = read_consent(&transaction, project_id, consent_id)?;
        if consent.revoked_at.is_some() {
            return Err(StoreError::InvalidConsent);
        }
        consent.revoked_at = Some(at);
        consent.validate().map_err(|_| StoreError::InvalidConsent)?;
        let count = transaction.execute("UPDATE resource_consents SET body=?1,revision=1 WHERE id=?2 AND project_id=?3 AND revision=0", params![serde_json::to_string(&consent)?,consent_id.as_str(),project_id.as_str()])?;
        if count != 1 {
            return Err(StoreError::InvalidConsent);
        }
        let payload = EventPayload::ResourceConsentRevoked {
            consent: Box::new(consent),
            revoked_by: user_id.clone(),
        };
        let receipt = append(
            &transaction,
            project_id,
            &command_id,
            Revision(1),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    pub fn resource_consent(
        &self,
        project_id: &ProjectId,
        consent_id: &CommandId,
    ) -> Result<ResourceConsent> {
        read_consent(&self.connection, project_id, consent_id)
    }

    pub fn project(&self, id: &ProjectId) -> Result<Project> {
        let body: String = self
            .connection
            .query_row(
                "SELECT body FROM projects WHERE id=?1",
                [id.as_str()],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(StoreError::NotFound)?;
        let project: Project = serde_json::from_str(&body)?;
        if &project.id != id {
            return Err(StoreError::Integrity(
                "project key differs from body".into(),
            ));
        }
        Ok(project)
    }
    pub fn task(&self, id: &TaskId) -> Result<Task> {
        read_task(&self.connection, id)
    }

    pub fn events(&self, project_id: &ProjectId, after: u64, limit: u32) -> Result<EventPage> {
        if limit == 0 || limit > MAX_EVENT_PAGE || after > i64::MAX as u64 {
            return Err(StoreError::InvalidPage);
        }
        let snapshot = self.connection.unchecked_transaction()?;
        self.project(project_id)?;
        let current_head: u64 =
            snapshot.query_row("SELECT coalesce(max(sequence),0) FROM journal", [], |r| {
                sql_u64(r, 0)
            })?;
        if after > current_head {
            return Err(StoreError::InvalidPage);
        }
        let mut query = snapshot.prepare("SELECT sequence,command_id,revision,payload FROM journal WHERE project_id=?1 AND sequence>?2 ORDER BY sequence LIMIT ?3")?;
        let rows = query.query_map(
            params![project_id.as_str(), after as i64, limit as i64 + 1],
            |row| {
                Ok((
                    sql_u64(row, 0)?,
                    row.get::<_, String>(1)?,
                    sql_u64(row, 2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )?;
        let mut events = Vec::new();
        for row in rows {
            let (sequence, command_id, revision, payload) = row?;
            events.push(StoredEvent {
                sequence,
                project_id: project_id.clone(),
                command_id: CommandId::new(command_id)
                    .map_err(|e| StoreError::Integrity(e.to_string()))?,
                revision: Revision(revision),
                payload: serde_json::from_str(&payload)?,
            });
        }
        let has_more = events.len() > limit as usize;
        if has_more {
            events.pop();
        }
        let next_cursor = events.last().map_or(after, |event| event.sequence);
        Ok(EventPage {
            events,
            next_cursor,
            has_more,
        })
    }

    pub fn integrity_check(&self) -> Result<()> {
        let snapshot = self.connection.unchecked_transaction()?;
        audit_connection(&snapshot)?;
        snapshot.commit()?;
        Ok(())
    }
}

fn read_lease_snapshot(
    connection: &Connection,
    task: &TaskId,
) -> Result<Option<symbiote_domain::TaskLease>> {
    lease::read(connection, task)
}

fn audit_connection(connection: &Connection) -> Result<()> {
    let check: String = connection.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if check != "ok" {
        return Err(StoreError::Integrity(check));
    }
    if connection.prepare("PRAGMA foreign_key_check")?.exists([])? {
        return Err(StoreError::Integrity("foreign key check failed".into()));
    }
    audit_journal(connection)
}

fn validate_consent_relationships(
    connection: &Connection,
    consent: &ResourceConsent,
) -> Result<()> {
    let snapshot = &consent.snapshot;
    let role_exists = connection
        .query_row(
            "SELECT 1 FROM roles WHERE id=?1 AND project_id=?2",
            params![snapshot.role_id.as_str(), snapshot.project_id.as_str()],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !role_exists {
        return Err(StoreError::RelationshipMismatch);
    }
    for root in &snapshot.access.roots {
        if connection
            .query_row(
                "SELECT 1 FROM roots WHERE id=?1 AND project_id=?2",
                params![root.as_str(), snapshot.project_id.as_str()],
                |_| Ok(()),
            )
            .optional()?
            .is_none()
        {
            return Err(StoreError::RelationshipMismatch);
        }
    }
    Ok(())
}

fn read_consent(
    connection: &Connection,
    project_id: &ProjectId,
    id: &CommandId,
) -> Result<ResourceConsent> {
    let (role, revision, body): (String, u64, String) = connection
        .query_row(
            "SELECT role_id,revision,body FROM resource_consents WHERE id=?1 AND project_id=?2",
            params![id.as_str(), project_id.as_str()],
            |r| Ok((r.get(0)?, sql_u64(r, 1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or(StoreError::NotFound)?;
    let consent: ResourceConsent = serde_json::from_str(&body)?;
    consent.validate().map_err(|_| StoreError::InvalidConsent)?;
    if &consent.id != id
        || &consent.snapshot.project_id != project_id
        || consent.snapshot.role_id.as_str() != role
        || revision != u64::from(consent.revoked_at.is_some())
    {
        return Err(StoreError::Integrity(
            "consent indexed state differs from body".into(),
        ));
    }
    validate_consent_relationships(connection, &consent)?;
    Ok(consent)
}

fn revocation_request(
    project: &ProjectId,
    consent: &CommandId,
    user: &UserId,
    at: Timestamp,
) -> Result<String> {
    Ok(serde_json::to_string(&(
        "revoke_resource_consent",
        project,
        consent,
        user,
        at,
    ))?)
}

fn mutation_request(task_id: &TaskId, command: &TaskCommand) -> Result<String> {
    #[derive(Serialize)]
    struct Mutation<'a> {
        operation: &'static str,
        task_id: &'a TaskId,
        command: &'a TaskCommand,
    }
    Ok(serde_json::to_string(&Mutation {
        operation: "apply_task",
        task_id,
        command,
    })?)
}

/// Reconstruct the supported records in a read snapshot, then compare them to
/// indexed current state. Never repair or reset a mismatched record automatically.
fn audit_journal(connection: &Connection) -> Result<()> {
    let mut work_audit = work::Audit::default();
    let mut team_audit = team::Audit::default();
    let mut binding_audit = binding::Audit::default();
    let mut route_audit = route::Audit::default();
    let mut dependency_audit = dependency::Audit::default();
    let mut leases: BTreeMap<TaskId, symbiote_domain::TaskLease> = BTreeMap::new();
    let mut projects = BTreeMap::new();
    let mut tasks: BTreeMap<TaskId, Task> = BTreeMap::new();
    let mut streams = BTreeMap::new();
    let mut consents: BTreeMap<CommandId, ResourceConsent> = BTreeMap::new();
    let mut statement = connection.prepare("SELECT sequence,project_id,command_id,revision,request,payload FROM journal ORDER BY sequence")?;
    let rows = statement.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            sql_u64(r, 3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
        ))
    })?;
    let mut expected_sequence = 1_i64;
    for row in rows {
        let (sequence, project_key, command_key, revision, request, body) = row?;
        if sequence != expected_sequence {
            return Err(StoreError::Integrity(
                "journal sequence must be positive and contiguous".into(),
            ));
        }
        expected_sequence = expected_sequence
            .checked_add(1)
            .ok_or_else(|| StoreError::Integrity("journal sequence exhausted".into()))?;
        let event: EventPayload = serde_json::from_str(&body)?;
        match &event {
            EventPayload::BindingReplaced {
                configuration,
                expected_revision,
                actor,
                at,
            } => {
                binding_audit.replaced(
                    configuration,
                    *expected_revision,
                    actor,
                    *at,
                    (&project_key, revision, &request),
                    &team_audit,
                )?;
            }
            EventPayload::TeamReplaced {
                team,
                expected_revision,
                actor,
                at,
            } => {
                team_audit.replaced(
                    team,
                    *expected_revision,
                    actor,
                    *at,
                    (&project_key, revision, &request),
                    &projects,
                )?;
            }
            EventPayload::WorkItemCreated { item } => {
                work_audit.created(item, &project_key, revision, &request, &event, &projects)?;
            }
            EventPayload::WorkItemChanged {
                project_id,
                work_id,
                command,
                item,
            } => {
                work_audit.changed(
                    project_id,
                    work_id,
                    command,
                    item,
                    (&project_key, &command_key, revision, &request),
                )?;
            }
            EventPayload::TaskOriginAssigned {
                task_id,
                project_id,
                origin,
                ..
            } => {
                let task = tasks.get(task_id).ok_or(StoreError::NotFound)?;
                if task.project_id() != project_id
                    || project_id.as_str() != project_key
                    || revision != task.revision().0
                    || request != serde_json::to_string(&event)?
                {
                    return Err(StoreError::RelationshipMismatch);
                }
                work_audit.origin(task, origin)?;
            }
            EventPayload::ResourceConsentRecorded { consent } => {
                consent.validate().map_err(|_| StoreError::InvalidConsent)?;
                validate_consent_relationships(connection, consent)?;
                if consent.id.as_str() != command_key
                    || consent.snapshot.project_id.as_str() != project_key
                    || revision != 0
                    || consent.revoked_at.is_some()
                    || !projects.contains_key(&consent.snapshot.project_id)
                    || request != serde_json::to_string(&event)?
                    || consents
                        .insert(consent.id.clone(), *consent.clone())
                        .is_some()
                {
                    return Err(StoreError::Integrity(
                        "invalid consent registration lineage".into(),
                    ));
                }
            }
            EventPayload::ResourceConsentRevoked {
                consent,
                revoked_by,
            } => {
                let at = consent.revoked_at.ok_or(StoreError::InvalidConsent)?;
                let previous = consents
                    .get_mut(&consent.id)
                    .ok_or_else(|| StoreError::Integrity("revocation precedes consent".into()))?;
                if previous.revoked_at.is_some() {
                    return Err(StoreError::Integrity("consent already revoked".into()));
                }
                previous.revoked_at = Some(at);
                previous
                    .validate()
                    .map_err(|_| StoreError::InvalidConsent)?;
                if previous != consent.as_ref()
                    || consent.snapshot.project_id.as_str() != project_key
                    || revision != 1
                    || request
                        != revocation_request(
                            &consent.snapshot.project_id,
                            &consent.id,
                            revoked_by,
                            at,
                        )?
                {
                    return Err(StoreError::Integrity(
                        "invalid consent revocation lineage".into(),
                    ));
                }
            }
            EventPayload::ProjectRegistered {
                project,
                roots,
                roles,
            } => {
                if project.id.as_str() != project_key
                    || project.revision.0 != revision
                    || request != serde_json::to_string(&event)?
                    || projects
                        .insert(
                            project.id.clone(),
                            (*project.clone(), roots.clone(), roles.clone()),
                        )
                        .is_some()
                {
                    return Err(StoreError::Integrity(
                        "invalid project registration journal lineage".into(),
                    ));
                }
            }
            EventPayload::TaskCreated {
                task,
                stream,
                origin,
            } => {
                if let Some(origin) = origin {
                    work_audit.origin(task, origin)?;
                }
                if task.project_id().as_str() != project_key
                    || task.revision().0 != revision
                    || request != serde_json::to_string(&event)?
                    || tasks.insert(task.id().clone(), *task.clone()).is_some()
                    || streams
                        .insert(stream.id().clone(), *stream.clone())
                        .is_some()
                {
                    return Err(StoreError::Integrity(
                        "invalid task creation journal lineage".into(),
                    ));
                }
            }
            EventPayload::WorkRouted {
                decision,
                actor,
                at,
            } => {
                route_audit.routed(
                    decision,
                    actor,
                    *at,
                    (&project_key, revision, &request),
                    &work_audit,
                    &team_audit,
                )?;
            }
            EventPayload::TaskLeased { lease, actor, at } => {
                let lease = lease.as_ref();
                lease
                    .validate_shape()
                    .map_err(|_| StoreError::InvalidLease)?;
                if lease.project_id.as_str() != project_key
                    || at.0 > i64::MAX as u64
                    || request != serde_json::to_string(&lease::event(lease, actor, *at))?
                {
                    return Err(StoreError::Integrity(
                        "invalid lease journal lineage".into(),
                    ));
                }
                let task = tasks.get(&lease.task_id).ok_or(StoreError::NotFound)?;
                if task.project_id() != &lease.project_id
                    || task.state() != &TaskState::Running
                    || task
                        .current_dispatch()
                        .map(|d| d.id() != &lease.dispatch_id)
                        .unwrap_or(true)
                {
                    return Err(StoreError::Integrity("lease task lineage mismatch".into()));
                }
                leases.insert(lease.task_id.clone(), lease.clone());
            }
            EventPayload::TaskDependenciesSet {
                task_id,
                project_id,
                edges,
                actor,
                at,
            } => {
                dependency_audit.set(
                    task_id,
                    &Some(project_id.clone()),
                    edges,
                    actor,
                    *at,
                    (&project_key, revision, &request),
                )?;
            }
            EventPayload::TaskChanged {
                task_id,
                command,
                task,
            } => {
                let previous = tasks.get_mut(task_id).ok_or_else(|| {
                    StoreError::Integrity("transition precedes task creation".into())
                })?;
                previous.apply(*command.clone())?;
                if previous != task.as_ref()
                    || task.project_id().as_str() != project_key
                    || task.revision().0 != revision
                    || command.id.as_str() != command_key
                    || request != mutation_request(task_id, command)?
                {
                    return Err(StoreError::Integrity(
                        "task journal does not replay to recorded state".into(),
                    ));
                }
            }
        }
    }
    let dependency_targets = |project: &ProjectId, task: &TaskId| {
        tasks
            .get(task)
            .is_some_and(|task| task.project_id() == project)
    };
    dependency_audit.validate_graph(dependency_targets)?;
    for (task, lease) in leases {
        if read_lease_snapshot(connection, &task)?.as_ref() != Some(&lease) {
            return Err(StoreError::Integrity(
                "lease state differs from journal".into(),
            ));
        }
    }
    work_audit.finish(connection)?;
    team_audit.finish(connection)?;
    binding_audit.finish(connection)?;
    route_audit.finish(connection)?;
    dependency_audit.finish(connection)?;
    for (query, expected) in [
        ("SELECT count(*) FROM projects", projects.len()),
        ("SELECT count(*) FROM tasks", tasks.len()),
        ("SELECT count(*) FROM streams", streams.len()),
        ("SELECT count(*) FROM resource_consents", consents.len()),
    ] {
        if connection.query_row(query, [], |r| sql_usize(r, 0))? != expected {
            return Err(StoreError::Integrity(
                "current record count differs from journal".into(),
            ));
        }
    }
    let mut root_count = 0;
    let mut role_count = 0;
    for (id, (project, roots, roles)) in projects {
        let (revision, lead, body): (u64, String, String) = connection.query_row(
            "SELECT revision,lead_id,body FROM projects WHERE id=?1",
            [id.as_str()],
            |r| Ok((sql_u64(r, 0)?, r.get(1)?, r.get(2)?)),
        )?;
        if serde_json::from_str::<Project>(&body)? != project
            || project.revision.0 != revision
            || project.lead.as_str() != lead
        {
            return Err(StoreError::Integrity(
                "project state differs from journal".into(),
            ));
        }
        root_count += roots.len();
        role_count += roles.len();
        for root in roots {
            let (owner, body): (String, String) = connection.query_row(
                "SELECT project_id,body FROM roots WHERE id=?1",
                [root.id.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            if serde_json::from_str::<Root>(&body)? != root
                || root.project_id != id
                || owner != id.as_str()
            {
                return Err(StoreError::Integrity(
                    "root state differs from journal".into(),
                ));
            }
        }
        for role in roles {
            let (owner, body): (String, String) = connection.query_row(
                "SELECT project_id,body FROM roles WHERE id=?1",
                [role.id.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            if serde_json::from_str::<Role>(&body)? != role
                || role.project_id != id
                || owner != id.as_str()
            {
                return Err(StoreError::Integrity(
                    "role state differs from journal".into(),
                ));
            }
        }
    }
    for (query, expected) in [
        ("SELECT count(*) FROM roots", root_count),
        ("SELECT count(*) FROM roles", role_count),
    ] {
        if connection.query_row(query, [], |r| sql_usize(r, 0))? != expected {
            return Err(StoreError::Integrity(
                "related record count differs from journal".into(),
            ));
        }
    }
    for (id, task) in tasks {
        if read_task(connection, &id)? != task {
            return Err(StoreError::Integrity(
                "task state differs from journal".into(),
            ));
        }
    }
    for (id, stream) in streams {
        if read_stream(connection, &id)? != stream {
            return Err(StoreError::Integrity(
                "stream state differs from journal".into(),
            ));
        }
    }
    for (id, consent) in consents {
        if read_consent(connection, &consent.snapshot.project_id, &id)? != consent {
            return Err(StoreError::Integrity(
                "consent state differs from journal".into(),
            ));
        }
    }
    Ok(())
}

fn sql_revision(revision: Revision) -> Result<i64> {
    i64::try_from(revision.0).map_err(|_| StoreError::Domain(DomainError::RevisionExhausted))
}
fn exists(connection: &Connection, query: &str, id: &str) -> Result<bool> {
    Ok(connection
        .query_row(query, [id], |_| Ok(()))
        .optional()?
        .is_some())
}
fn replay(transaction: &Transaction<'_>, id: &CommandId, request: &str) -> Result<Option<Receipt>> {
    let existing = transaction
        .query_row(
            "SELECT sequence,revision,request FROM journal WHERE command_id=?1",
            [id.as_str()],
            |row| Ok((sql_u64(row, 0)?, sql_u64(row, 1)?, row.get::<_, String>(2)?)),
        )
        .optional()?;
    match existing {
        Some((sequence, revision, old)) if old == request => Ok(Some(Receipt {
            sequence,
            revision: Revision(revision),
            replayed: true,
        })),
        Some(_) => Err(StoreError::IdempotencyConflict),
        None => Ok(None),
    }
}
fn append(
    transaction: &Transaction<'_>,
    project: &ProjectId,
    command: &CommandId,
    revision: Revision,
    request: &str,
    payload: &EventPayload,
) -> Result<Receipt> {
    transaction.execute("INSERT INTO journal(project_id,command_id,revision,request,payload) VALUES (?1,?2,?3,?4,?5)",
        params![project.as_str(),command.as_str(),sql_revision(revision)?,request,serde_json::to_string(payload)?])?;
    Ok(Receipt {
        sequence: u64::try_from(transaction.last_insert_rowid())
            .map_err(|error| StoreError::Integrity(error.to_string()))?,
        revision,
        replayed: false,
    })
}
fn read_task(connection: &Connection, id: &TaskId) -> Result<Task> {
    let (project, root, role, stream, revision, body): (
        String,
        String,
        String,
        String,
        u64,
        String,
    ) = connection
        .query_row(
            "SELECT project_id,root_id,role_id,stream_id,revision,body FROM tasks WHERE id=?1",
            [id.as_str()],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    sql_u64(r, 4)?,
                    r.get(5)?,
                ))
            },
        )
        .optional()?
        .ok_or(StoreError::NotFound)?;
    let task: Task = serde_json::from_str(&body)?;
    if task.id() != id
        || task.project_id().as_str() != project
        || task.root_id().as_str() != root
        || task.role_id().as_str() != role
        || task.stream_id().as_str() != stream
        || task.revision().0 != revision
    {
        return Err(StoreError::Integrity(
            "task columns differ from validated body".into(),
        ));
    }
    Ok(task)
}
fn read_stream(connection: &Connection, id: &ChangeStreamId) -> Result<ChangeStream> {
    let (project, root, worktree, branch, body): (String, String, String, String, String) =
        connection
            .query_row(
                "SELECT project_id,root_id,worktree_id,branch,body FROM streams WHERE id=?1",
                [id.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()?
            .ok_or(StoreError::NotFound)?;
    let stream: ChangeStream = serde_json::from_str(&body)?;
    if stream.id() != id
        || stream.project_id().as_str() != project
        || stream.root_id().as_str() != root
        || stream.worktree.as_str() != worktree
        || stream.branch != branch
    {
        return Err(StoreError::Integrity(
            "stream columns differ from validated body".into(),
        ));
    }
    Ok(stream)
}

#[cfg(test)]
mod tests;

fn sql_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(index)?;
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn sql_usize(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let value: i64 = row.get(index)?;
    usize::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}
