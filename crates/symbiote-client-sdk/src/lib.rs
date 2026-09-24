//! Shared client state for every Symbiote controller (#182 foundation):
//! one robust connection/state implementation instead of each client
//! recreating synchronization logic. The SDK owns the envelope facts —
//! version negotiation input, command-id idempotency, correlation, journal
//! cursor resume — and the deterministic classification of daemon
//! responses. It performs no I/O itself: the transport is injected as a
//! [`FrameExchange`] so CLI, desktop and tests share identical resync
//! logic, and chaos tests drive the same state machine with scripted
//! failure sequences.
//!
//! Boundary: nothing here completes canonical work or assumes worker
//! success. A UI disconnect is a transport fact, never a task outcome —
//! lost responses are recovered by replaying the same command id (the
//! daemon's journal returns the durable receipt), never by guessing.
use serde_json::Value;
use std::collections::BTreeMap;
use symbiote_domain::{
    CommandId, Project, ProjectId, RequestId, Role, RoleId, Root, RootId, Task, TaskId,
};
use symbiote_protocol::JournalCursor;
use symbiote_protocol::{
    CURRENT_VERSION, EventPayload, JournalEvent, MAX_PAGE_SIZE, MAX_REQUEST_BYTES, ProjectSnapshot,
    ProtocolError, ProtocolVersion, Request, Response, ResponseBody, parse_request,
};

/// One connection's durable position in a Project's journal. Resume means:
/// re-read from `cursor` after reconnect; pages advance it only over events
/// actually returned. Cursor 0 means "bootstrap from the beginning" — the
/// caller decides whether a full bootstrap or a tail read is appropriate.
/// The JSON form is what a desktop shell persists across its own restart.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JournalPosition {
    pub project: ProjectId,
    pub cursor: JournalCursor,
}

/// The transport boundary: send one bounded request frame, receive one
/// bounded response. `Err` means the transport failed — the SDK classifies
/// whether the request may have been committed (nothing is known) and the
/// caller decides retry policy. Implementations wrap the Host's
/// authenticated Unix socket; tests script frames and failures.
pub trait FrameExchange {
    fn exchange(&mut self, request: &[u8]) -> Result<Vec<u8>, TransportFailure>;
}

/// A transport failure. Nothing distinguishes "request never arrived" from
/// "response lost after commit" — that ambiguity is exactly why command ids
/// are idempotency keys and why this SDK never treats a disconnect as an
/// outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransportFailure;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientError {
    /// The daemon refused the command; the typed error is preserved for the
    /// caller to branch on (stale revision, permission, conflict, …).
    Refused(ProtocolError),
    /// The daemon's response did not parse as a protocol response for this
    /// version. Never treated as a refusal; the command's disposition is
    /// unknown and recoverable by id replay.
    Unparseable,
    /// The transport failed before/after an unknown disposition.
    Transport,
    /// The caller's request exceeded the protocol's 64 KiB bound and was
    /// never sent.
    RequestTooLarge,
    /// The caller's command id is not a legal `CommandId` (charset or
    /// length); nothing was sent.
    InvalidCommandId,
    /// The operation does not deserialize into a protocol operation (wrong
    /// `kind`, unknown fields) or fails the request validation the Host
    /// would apply; nothing was sent.
    InvalidOperation,
}

/// The client's reduced view of one Project: the records a snapshot seeds and
/// [`ClientSession::resume`] keeps current by applying journal events. It
/// models exactly the record kinds a snapshot carries — the Project, its
/// Roots, its Roles and its Tasks — and names every other durable event
/// [`EventOutcome::NotModeled`] rather than dropping it silently: the durable
/// cursor still advances over those events, and the state never claims a
/// record no read or event provided.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClientState {
    project: Option<Project>,
    roots: BTreeMap<RootId, Root>,
    roles: BTreeMap<RoleId, Role>,
    tasks: BTreeMap<TaskId, Task>,
}

/// What one durable event did to a [`ClientState`]. Every event lands in
/// exactly one of these; none is silently skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventOutcome {
    /// The event's record was set (or replaced an earlier revision of it).
    Applied,
    /// The event belongs to a different Project than this state holds; it is
    /// named, never mixed in.
    ForeignProject,
    /// The event is durable history this state carries no record for. The
    /// cursor still advances over it; the state claims nothing about it.
    NotModeled,
}

/// What one [`ClientSession::resume`] did. Every page and event is counted by
/// the classification it got; nothing is inferred.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct ResumeOutcome {
    /// Pages the daemon answered.
    pub pages: usize,
    /// Events applied to the state (a record was set or replaced).
    pub applied: usize,
    /// Durable events this state carries no record for.
    pub not_modeled: usize,
    /// Events belonging to a different Project than this state holds. They
    /// are counted and never mixed in, so a state seeded for one Project
    /// cannot absorb another's history.
    pub foreign: usize,
    /// Events at or below the durable cursor: already applied, never applied
    /// twice.
    pub duplicates: usize,
    /// Pages whose events were all at or below the durable cursor: the daemon
    /// answered from behind this client's position, which is an observable
    /// degraded state, not a silent one.
    pub stale_pages: usize,
    /// True only when the daemon reported no further events after the last
    /// page. A resume stopped by its page bound reports false.
    pub caught_up: bool,
    /// The durable cursor after the resume.
    pub cursor: JournalCursor,
}

/// One resume's bounds, carried together so the page loop is a loop over
/// pages rather than over arguments.
#[derive(Clone, Copy)]
struct ResumeBounds<'a> {
    project: &'a ProjectId,
    page_limit: u32,
    max_pages: usize,
    command_prefix: &'a str,
}

impl ClientState {
    /// The Project record once the state has one (seeded or replayed).
    pub fn project(&self) -> Option<&Project> {
        self.project.as_ref()
    }
    pub fn roots(&self) -> &BTreeMap<RootId, Root> {
        &self.roots
    }
    pub fn roles(&self) -> &BTreeMap<RoleId, Role> {
        &self.roles
    }
    pub fn tasks(&self) -> &BTreeMap<TaskId, Task> {
        &self.tasks
    }

    /// Replaces this state with a snapshot's records. Seeding is total rather
    /// than a patch: the snapshot is a point in the log, so a state can never
    /// keep a record no read returned.
    fn seed(&mut self, snapshot: &ProjectSnapshot) {
        self.project = Some(snapshot.project.clone());
        self.roots = snapshot
            .roots
            .iter()
            .cloned()
            .map(|root| (root.id.clone(), root))
            .collect();
        self.roles = snapshot
            .roles
            .iter()
            .cloned()
            .map(|role| (role.id.clone(), role))
            .collect();
        self.tasks = snapshot
            .tasks
            .iter()
            .cloned()
            .map(|task| (task.id().clone(), task))
            .collect();
    }

    /// Applies one durable event to this state and reports what it did. A
    /// state that holds no Project yet adopts the event's Project (the
    /// cold-replay path, where the Project is created by its own registration
    /// event); once a Project is held, another Project's events are
    /// [`EventOutcome::ForeignProject`] and change nothing.
    pub fn apply(&mut self, event: &JournalEvent) -> EventOutcome {
        if self
            .project
            .as_ref()
            .is_some_and(|held| held.id != event.project_id)
        {
            return EventOutcome::ForeignProject;
        }
        match &event.payload {
            EventPayload::ProjectRegistered {
                project,
                roots,
                roles,
            } => {
                if project.id != event.project_id {
                    return EventOutcome::ForeignProject;
                }
                self.project = Some(project.clone());
                for root in roots {
                    self.roots.insert(root.id.clone(), root.clone());
                }
                for role in roles {
                    self.roles.insert(role.id.clone(), role.clone());
                }
                EventOutcome::Applied
            }
            EventPayload::RootPlacementObserved { root, .. } => {
                if root.project_id != event.project_id {
                    return EventOutcome::ForeignProject;
                }
                self.roots.insert(root.id.clone(), (**root).clone());
                EventOutcome::Applied
            }
            EventPayload::TaskCreated { task, .. } | EventPayload::TaskChanged { task, .. } => {
                if task.project_id() != &event.project_id {
                    return EventOutcome::ForeignProject;
                }
                self.tasks.insert(task.id().clone(), (**task).clone());
                EventOutcome::Applied
            }
            _ => EventOutcome::NotModeled,
        }
    }
}

/// A client session: mints protocol-versioned requests with caller-supplied
/// command ids (idempotency keys) and unique correlations, classifies
/// responses, and tracks per-Project journal positions for resume.
#[derive(Debug)]
pub struct ClientSession {
    /// Rejected at construction if not exactly the supported version: the
    /// SDK pins CURRENT_VERSION and does not guess compatibility.
    supported: ProtocolVersion,
    positions: Vec<JournalPosition>,
    serial: u64,
    pid: u32,
}

impl ClientSession {
    /// The production session configuration.
    pub fn new() -> Self {
        Self {
            supported: CURRENT_VERSION,
            positions: Vec::new(),
            serial: 0,
            pid: std::process::id(),
        }
    }

    /// Restores a session's journal positions after process restart. A
    /// position for an unknown Project is created by the first
    /// [`Self::journal_position`] read.
    pub fn with_positions(mut self, positions: Vec<JournalPosition>) -> Self {
        self.positions = positions;
        self
    }

    /// The durable cursor for a Project, or 0 (bootstrap) when never read.
    pub fn journal_position(&self, project: &ProjectId) -> JournalCursor {
        self.positions
            .iter()
            .find(|p| &p.project == project)
            .map(|p| p.cursor)
            .unwrap_or(JournalCursor(0))
    }

    /// The positions to persist across a process restart; restore them
    /// with [`Self::with_positions`].
    pub fn positions(&self) -> &[JournalPosition] {
        &self.positions
    }

    /// Observes a page's cursor without ever moving a durable position
    /// backwards: a page behind the position this session already read (a
    /// delayed answer, or a replayed response) is observed as the maximum, not
    /// as a reset, so resume can never re-apply history it already applied.
    fn observe_page(&mut self, next: JournalCursor, project: &ProjectId) {
        match self.positions.iter_mut().find(|p| &p.project == project) {
            Some(position) => position.cursor = position.cursor.max(next),
            None => self.positions.push(JournalPosition {
                project: project.clone(),
                cursor: next,
            }),
        }
    }

    /// Pins a Project's durable cursor to one point. Used only where the
    /// caller's state was just replaced wholesale (bootstrap): the cursor
    /// must name the same point the state was read at, even if a stale
    /// session held a position beyond it.
    fn pin_position(&mut self, cursor: JournalCursor, project: &ProjectId) {
        match self.positions.iter_mut().find(|p| &p.project == project) {
            Some(position) => position.cursor = cursor,
            None => self.positions.push(JournalPosition {
                project: project.clone(),
                cursor,
            }),
        }
    }

    /// Builds a request. `command_id` is the caller's idempotency key: the
    /// same id with byte-identical intent replays the durable receipt
    /// instead of re-executing. The correlation id is unique per call and
    /// carries no authority.
    pub fn build_request(
        &mut self,
        command_id: &str,
        operation: Value,
    ) -> Result<Request, ClientError> {
        self.serial += 1;
        let request = Request {
            version: self.supported,
            correlation_id: RequestId::new(format!(
                "sdk-{}-{}-{}",
                self.pid,
                self.serial,
                command_id.chars().take(40).collect::<String>()
            ))
            .map_err(|_| ClientError::InvalidCommandId)?,
            command_id: CommandId::new(command_id.to_owned())
                .map_err(|_| ClientError::InvalidCommandId)?,
            operation: serde_json::from_value(operation)
                .map_err(|_| ClientError::InvalidOperation)?,
        };
        let bytes = serde_json::to_vec(&request).map_err(|_| ClientError::RequestTooLarge)?;
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(ClientError::RequestTooLarge);
        }
        // The daemon parses the same bytes; catching malformed operations
        // client-side keeps offline misfires from becoming transport noise.
        parse_request(&bytes).map_err(|_| ClientError::InvalidOperation)?;
        Ok(request)
    }

    /// Sends one built request and classifies the response without touching
    /// durable state; the caller decides what may be observed from it.
    fn exchange_body(
        &self,
        exchange: &mut impl FrameExchange,
        request: &Request,
    ) -> Result<ResponseBody, ClientError> {
        let bytes = serde_json::to_vec(request).map_err(|_| ClientError::RequestTooLarge)?;
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(ClientError::RequestTooLarge);
        }
        let response_bytes = exchange
            .exchange(&bytes)
            .map_err(|_| ClientError::Transport)?;
        let response: Response =
            serde_json::from_slice(&response_bytes).map_err(|_| ClientError::Unparseable)?;
        if response.version != self.supported {
            // A version mismatch means the daemon was upgraded or replaced
            // under us; the caller must re-handshake rather than guess.
            return Err(ClientError::Unparseable);
        }
        match response.result {
            Ok(body) => Ok(body),
            Err(error) => Err(ClientError::Refused(error)),
        }
    }

    /// Sends one request and classifies the response. On `Ok`, the typed body
    /// is returned; the SDK also observes the cursor of a journal page the
    /// caller was handed (never moving it backwards), so a direct
    /// `read_journal` call resumes from the right point without plumbing
    /// cursors by hand.
    pub fn call(
        &mut self,
        exchange: &mut impl FrameExchange,
        request: &Request,
        project: Option<&ProjectId>,
    ) -> Result<ResponseBody, ClientError> {
        let body = self.exchange_body(exchange, request)?;
        if let (Some(project), ResponseBody::Journal(page)) = (project, &body) {
            // The caller's project must agree with the operation's own — a
            // mismatched hand-off would advance one project's cursor with
            // another's next_cursor.
            if request.operation.project_id() == Some(project) {
                self.observe_page(page.next_cursor, project);
            }
        }
        Ok(body)
    }

    /// Bootstraps a Project from one consistent read: asks for the snapshot,
    /// requires it to name the Project this call asked about (a snapshot for
    /// another Project is not an answer to this request, refused as
    /// `Unparseable`), seeds a state from it, and pins the Project's durable
    /// cursor to the snapshot's own cursor.
    ///
    /// The returned state is the snapshot: every record the Project's own
    /// reads answered at that cursor, with nothing carried over from an
    /// earlier state. A refusal returns no state and leaves the cursor where
    /// it was.
    pub fn bootstrap(
        &mut self,
        exchange: &mut impl FrameExchange,
        command_id: &str,
        project: &ProjectId,
    ) -> Result<ClientState, ClientError> {
        let request = self.build_request(
            command_id,
            serde_json::json!({"kind": "snapshot", "project_id": project}),
        )?;
        let body = self.exchange_body(exchange, &request)?;
        let ResponseBody::Snapshot(snapshot) = body else {
            return Err(ClientError::Unparseable);
        };
        if &snapshot.project.id != project {
            return Err(ClientError::Unparseable);
        }
        let mut state = ClientState::default();
        state.seed(&snapshot);
        self.pin_position(snapshot.cursor, project);
        Ok(state)
    }

    /// Resumes a Project from its durable cursor, applying journal pages to
    /// `state` until the daemon reports no further events. The rules:
    ///
    /// - Only events after the durable cursor are applied. An event at or
    ///   below it is a duplicate the caller has already applied: counted
    ///   ([`ResumeOutcome::duplicates`]) and skipped, never applied twice.
    /// - A page that breaks the daemon's own page contract is refused as
    ///   `Unparseable` with nothing applied, so neither the state nor the
    ///   durable cursor moves on a page that was not an answer to this
    ///   request. Duplicate events are the one tolerated exception: filtered
    ///   out, they leave the remainder under the same contract, and a page
    ///   that holds nothing new is a stale page ([`ResumeOutcome::stale_pages`])
    ///   that stops the resume instead of being re-asked forever.
    /// - A transport failure stops the resume with the durable cursor at the
    ///   last applied event, so calling `resume` again continues from exactly
    ///   there; a refusal is surfaced immediately and unapplied.
    /// - The loop is bounded by `max_pages`. A resume that stops at the bound
    ///   reports `caught_up: false` rather than claiming the tail was read.
    ///
    /// Each page is a distinct request and so carries a distinct command id
    /// derived from `command_prefix`.
    pub fn resume(
        &mut self,
        exchange: &mut impl FrameExchange,
        project: &ProjectId,
        state: &mut ClientState,
        page_limit: u32,
        max_pages: usize,
        command_prefix: &str,
    ) -> Result<ResumeOutcome, ClientError> {
        if page_limit == 0 || page_limit > MAX_PAGE_SIZE {
            return Err(ClientError::InvalidOperation);
        }
        let mut outcome = ResumeOutcome {
            cursor: self.journal_position(project),
            ..ResumeOutcome::default()
        };
        let bounds = ResumeBounds {
            project,
            page_limit,
            max_pages,
            command_prefix,
        };
        let result = self.resume_pages(exchange, state, bounds, &mut outcome);
        // The durable cursor is wherever the last applied event left it, on
        // failure as on success: a transport failure cannot un-apply what the
        // caller already observed.
        self.observe_page(outcome.cursor, project);
        result.map(|()| outcome)
    }

    fn resume_pages(
        &mut self,
        exchange: &mut impl FrameExchange,
        state: &mut ClientState,
        bounds: ResumeBounds<'_>,
        outcome: &mut ResumeOutcome,
    ) -> Result<(), ClientError> {
        let ResumeBounds {
            project,
            page_limit,
            max_pages,
            command_prefix,
        } = bounds;
        for index in 0..max_pages {
            let after = outcome.cursor;
            let request = self.build_request(
                &format!("{command_prefix}-{index}"),
                serde_json::json!({
                    "kind": "read_journal",
                    "project_id": project,
                    "after": after.0,
                    "limit": page_limit,
                }),
            )?;
            let body = self.exchange_body(exchange, &request)?;
            let ResponseBody::Journal(page) = body else {
                return Err(ClientError::Unparseable);
            };
            outcome.pages += 1;
            // The daemon's own page contract comes first: a conforming page
            // is strictly above the cursor, so nothing needs filtering.
            if page.validate(project, after, page_limit).is_ok() {
                for event in &page.events {
                    Self::apply_event(state, event, outcome);
                }
                outcome.cursor = page.next_cursor;
                if !page.has_more {
                    outcome.caught_up = true;
                    break;
                }
                continue;
            }
            // A page that fails the contract is tolerated for exactly one
            // fact: events at or below the cursor are duplicates already
            // applied — a delayed answer. Filter those; the remainder must
            // then obey the same contract.
            let remaining: Vec<&JournalEvent> = page
                .events
                .iter()
                .filter(|event| event.sequence > after.0)
                .collect();
            outcome.duplicates += page.events.len() - remaining.len();
            if remaining.is_empty() {
                outcome.stale_pages += 1;
                break;
            }
            let strictly_increasing = remaining
                .windows(2)
                .all(|pair| pair[0].sequence < pair[1].sequence);
            let last = remaining.last().expect("checked non-empty").sequence;
            if !strictly_increasing
                || page.next_cursor.0 != last
                || remaining.len() > page_limit as usize
                || page.events.iter().any(|event| &event.project_id != project)
            {
                return Err(ClientError::Unparseable);
            }
            for event in &remaining {
                Self::apply_event(state, event, outcome);
            }
            outcome.cursor = page.next_cursor;
            if !page.has_more {
                outcome.caught_up = true;
                break;
            }
        }
        Ok(())
    }

    /// Applies one event the resume rules accepted and counts what it did.
    fn apply_event(state: &mut ClientState, event: &JournalEvent, outcome: &mut ResumeOutcome) {
        match state.apply(event) {
            EventOutcome::Applied => outcome.applied += 1,
            EventOutcome::ForeignProject => outcome.foreign += 1,
            EventOutcome::NotModeled => outcome.not_modeled += 1,
        }
    }

    /// Recovery for a lost response: resend the same command id and
    /// byte-identical operation. The daemon's journal replays the durable
    /// receipt (or refuses an idempotency conflict if the intent differs —
    /// surfaced, never silently re-executed).
    pub fn recover(
        &mut self,
        exchange: &mut impl FrameExchange,
        request: &Request,
        project: Option<&ProjectId>,
        attempts: usize,
    ) -> Result<ResponseBody, ClientError> {
        let mut last = ClientError::Transport;
        for _ in 0..attempts {
            match self.call(exchange, request, project) {
                Ok(value) => return Ok(value),
                Err(error @ ClientError::Refused(_)) => return Err(error),
                Err(error) => last = error,
            }
        }
        Err(last)
    }
}

impl Default for ClientSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response_ok(body: ResponseBody) -> Vec<u8> {
        serde_json::to_vec(&Response {
            version: CURRENT_VERSION,
            correlation_id: Some(RequestId::new("corr").unwrap()),
            result: Ok(body),
        })
        .unwrap()
    }

    fn response_refused(code: ErrorCode) -> Vec<u8> {
        let mut error = ProtocolError::new(code);
        error.message = "static refusal".to_owned();
        serde_json::to_vec(&Response {
            version: CURRENT_VERSION,
            correlation_id: Some(RequestId::new("corr").unwrap()),
            result: Err(error),
        })
        .unwrap()
    }

    use symbiote_protocol::ErrorCode;

    /// Scripted exchange: frames consumed in order; empty script = failure.
    struct Scripted {
        frames: VecDeque<Result<Vec<u8>, TransportFailure>>,
        sent: Vec<Vec<u8>>,
    }
    impl Scripted {
        fn new(frames: Vec<Result<Vec<u8>, TransportFailure>>) -> Self {
            Self {
                frames: frames.into(),
                sent: Vec::new(),
            }
        }
    }
    impl FrameExchange for Scripted {
        fn exchange(&mut self, request: &[u8]) -> Result<Vec<u8>, TransportFailure> {
            self.sent.push(request.to_vec());
            self.frames.pop_front().unwrap_or(Err(TransportFailure))
        }
    }

    use std::collections::VecDeque;

    fn operation(kind: &str) -> Value {
        serde_json::json!({"kind": kind})
    }

    fn project() -> ProjectId {
        ProjectId::new("proj").unwrap()
    }

    #[test]
    fn session_builds_versioned_requests_with_unique_correlations() {
        let mut session = ClientSession::new();
        let first = session.build_request("cmd-1", operation("health")).unwrap();
        let second = session.build_request("cmd-2", operation("health")).unwrap();
        assert_eq!(first.version, CURRENT_VERSION);
        assert_ne!(first.correlation_id, second.correlation_id);
        assert_ne!(first.command_id, second.command_id);
        // The same command id rebuilds byte-identical intent for recovery.
        let replay = session.build_request("cmd-1", operation("health")).unwrap();
        assert_eq!(replay.command_id, first.command_id);
    }

    #[test]
    fn oversized_requests_are_refused_before_any_transport() {
        let mut session = ClientSession::new();
        // A valid operation (request_task_completion with a huge report)
        // padded past the 64 KiB bound: the size check must be what fires,
        // not an earlier shape rejection. The transport is never touched.
        let transport = Scripted::new(vec![Ok(response_ok(ResponseBody::Hello(
            symbiote_protocol::negotiate(&[CURRENT_VERSION]).unwrap(),
        )))]);
        assert_eq!(
            session.build_request(
                "cmd-big",
                serde_json::json!({"kind":"request_task_completion",
                    "task_id":"task-one","dispatch_id":"disp-one",
                    "report":"x".repeat(MAX_REQUEST_BYTES + 1)}),
            ),
            Err(ClientError::RequestTooLarge)
        );
        assert!(transport.sent.is_empty());
    }

    #[test]
    fn invalid_command_ids_and_operations_have_their_own_error_identity() {
        let mut session = ClientSession::new();
        // Charset-invalid command id: never mapped to a size error.
        assert_eq!(
            session.build_request("bad id!", operation("health")),
            Err(ClientError::InvalidCommandId)
        );
        // Unknown operation kind: never mapped to a size error.
        assert_eq!(
            session.build_request("cmd-1", serde_json::json!({"kind":"raw"})),
            Err(ClientError::InvalidOperation)
        );
        // Semantic validation the Host would apply: refused client-side.
        assert_eq!(
            session.build_request(
                "cmd-2",
                serde_json::json!({"kind":"read_journal","project_id":"proj","after":0,"limit":0}),
            ),
            Err(ClientError::InvalidOperation)
        );
    }

    #[test]
    fn responses_classify_ok_refusal_transport_and_unparseable() {
        let mut session = ClientSession::new();
        let request = session.build_request("cmd-1", operation("health")).unwrap();

        let mut transport = Scripted::new(vec![Ok(response_ok(ResponseBody::Hello(
            symbiote_protocol::negotiate(&[CURRENT_VERSION]).unwrap(),
        )))]);
        let body = session.call(&mut transport, &request, None).unwrap();
        assert!(matches!(body, ResponseBody::Hello(_)));

        let mut transport = Scripted::new(vec![Ok(response_refused(ErrorCode::NotFound))]);
        assert!(matches!(
            session.call(&mut transport, &request, None),
            Err(ClientError::Refused(error)) if error.code == ErrorCode::NotFound
        ));

        let mut transport = Scripted::new(vec![Err(TransportFailure)]);
        assert_eq!(
            session.call(&mut transport, &request, None),
            Err(ClientError::Transport)
        );

        let mut transport = Scripted::new(vec![Ok(b"{not-json".to_vec())]);
        assert_eq!(
            session.call(&mut transport, &request, None),
            Err(ClientError::Unparseable)
        );
    }

    #[test]
    fn journal_pages_advance_the_project_cursor_for_resume() {
        let project = project();
        let mut session = ClientSession::new();
        assert_eq!(session.journal_position(&project), JournalCursor(0));
        let request = session
            .build_request(
                "cmd-read",
                serde_json::json!({"kind":"read_journal","project_id":"proj","after":0,"limit":10}),
            )
            .unwrap();
        let page = symbiote_protocol::JournalPage {
            events: vec![],
            next_cursor: JournalCursor(3),
            has_more: false,
        };
        let mut transport = Scripted::new(vec![Ok(response_ok(ResponseBody::Journal(page)))]);
        session
            .call(&mut transport, &request, Some(&project))
            .unwrap();
        assert_eq!(session.journal_position(&project), JournalCursor(3));
        // A non-Journal Ok body leaves positions untouched, as does a
        // Journal response with no project hand-off.
        let hello_request = session
            .build_request("cmd-hello", operation("health"))
            .unwrap();
        let mut transport = Scripted::new(vec![Ok(response_ok(ResponseBody::Hello(
            symbiote_protocol::negotiate(&[CURRENT_VERSION]).unwrap(),
        )))]);
        session
            .call(&mut transport, &hello_request, Some(&project))
            .unwrap();
        assert_eq!(session.journal_position(&project), JournalCursor(3));
        let journal_request = session
            .build_request(
                "cmd-read-2",
                serde_json::json!({"kind":"read_journal","project_id":"proj","after":3,"limit":10}),
            )
            .unwrap();
        let mut transport = Scripted::new(vec![Ok(response_ok(ResponseBody::Journal(
            symbiote_protocol::JournalPage {
                events: vec![],
                next_cursor: JournalCursor(9),
                has_more: false,
            },
        )))]);
        session
            .call(&mut transport, &journal_request, None)
            .unwrap();
        assert_eq!(session.journal_position(&project), JournalCursor(3));
        // A mismatched project hand-off never advances either cursor.
        let other = ProjectId::new("other").unwrap();
        let journal_request = session
            .build_request(
                "cmd-read-3",
                serde_json::json!({"kind":"read_journal","project_id":"proj","after":9,"limit":10}),
            )
            .unwrap();
        let mut transport = Scripted::new(vec![Ok(response_ok(ResponseBody::Journal(
            symbiote_protocol::JournalPage {
                events: vec![],
                next_cursor: JournalCursor(20),
                has_more: false,
            },
        )))]);
        session
            .call(&mut transport, &journal_request, Some(&other))
            .unwrap();
        assert_eq!(session.journal_position(&project), JournalCursor(3));
        assert_eq!(session.journal_position(&other), JournalCursor(0));
        // A restart restores positions; an unrelated project bootstraps at 0.
        let restored = ClientSession::new().with_positions(vec![JournalPosition {
            project: project.clone(),
            cursor: JournalCursor(3),
        }]);
        assert_eq!(restored.journal_position(&project), JournalCursor(3));
        assert_eq!(
            restored.journal_position(&ProjectId::new("other").unwrap()),
            JournalCursor(0)
        );
    }

    #[test]
    fn recovery_replays_the_same_command_id_and_never_treats_refusal_as_transport() {
        let mut session = ClientSession::new();
        let request = session
            .build_request("cmd-recover", operation("health"))
            .unwrap();
        // Transport failure, then success: the same bytes are resent.
        let mut transport = Scripted::new(vec![
            Err(TransportFailure),
            Ok(response_ok(ResponseBody::Hello(
                symbiote_protocol::negotiate(&[CURRENT_VERSION]).unwrap(),
            ))),
        ]);
        let body = session.recover(&mut transport, &request, None, 3).unwrap();
        assert!(matches!(body, ResponseBody::Hello(_)));
        assert_eq!(transport.sent.len(), 2);
        assert_eq!(transport.sent[0], transport.sent[1]);
        // A refusal is surfaced, not retried: the caller must not re-send a
        // command the daemon already answered authoritatively.
        let mut transport = Scripted::new(vec![
            Err(TransportFailure),
            Ok(response_refused(ErrorCode::StaleRevision)),
        ]);
        assert!(matches!(
            session.recover(&mut transport, &request, None, 3),
            Err(ClientError::Refused(_))
        ));
        // Exhausted attempts report the transport failure.
        let mut transport = Scripted::new(vec![Err(TransportFailure); 2]);
        assert_eq!(
            session.recover(&mut transport, &request, None, 2),
            Err(ClientError::Transport)
        );
    }

    #[test]
    fn deterministic_chaos_sequence_classifies_every_frame_honestly() {
        // A scripted hostile sequence over a fixed xorshift64 schedule
        // (pure arithmetic — no time, env, or RNG): every frame class is
        // consumed and classified, and the run's only durable mutation is
        // a cursor advanced by a genuinely observed Journal page. Includes
        // wrong-version responses and refusals.
        let project = project();
        let mut session = ClientSession::new();
        let request = session
            .build_request(
                "chaos-cmd",
                serde_json::json!({"kind":"read_journal","project_id":"proj","after":0,"limit":10}),
            )
            .unwrap();
        let mut state = 0x2545F4914F6CDD1Du64;
        let mut step = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % 6
        };
        let mut frames = Vec::new();
        for _round in 0..24 {
            match step() {
                0 | 3 => frames.push(Frame::Transport),
                1 => frames.push(Frame::Unparseable),
                2 => frames.push(Frame::Refused),
                4 => frames.push(Frame::WrongVersion),
                _ => frames.push(Frame::Journal(7)),
            }
        }
        // Guarantee every class appears at least once in the fixed schedule.
        assert!(frames.iter().any(|f| matches!(f, Frame::Transport)));
        assert!(frames.iter().any(|f| matches!(f, Frame::Unparseable)));
        assert!(frames.iter().any(|f| matches!(f, Frame::Refused)));
        assert!(frames.iter().any(|f| matches!(f, Frame::WrongVersion)));
        assert!(frames.iter().any(|f| matches!(f, Frame::Journal(_))));

        enum Frame {
            Transport,
            Unparseable,
            Refused,
            WrongVersion,
            Journal(u64),
        }
        let mut transport = Scripted::new(
            frames
                .iter()
                .map(|frame| match frame {
                    Frame::Transport => Err(TransportFailure),
                    Frame::Unparseable => Ok(b"{broken".to_vec()),
                    Frame::Refused => Ok(response_refused(ErrorCode::NotFound)),
                    Frame::WrongVersion => {
                        let mut bytes = response_ok(ResponseBody::Hello(
                            symbiote_protocol::negotiate(&[CURRENT_VERSION]).unwrap(),
                        ));
                        let mut response: Response = serde_json::from_slice(&bytes).unwrap();
                        response.version = ProtocolVersion {
                            major: 1,
                            minor: 12,
                        };
                        bytes = serde_json::to_vec(&response).unwrap();
                        Ok(bytes)
                    }
                    Frame::Journal(cursor) => Ok(response_ok(ResponseBody::Journal(
                        symbiote_protocol::JournalPage {
                            events: vec![],
                            next_cursor: JournalCursor(*cursor),
                            has_more: false,
                        },
                    ))),
                })
                .collect(),
        );
        let mut cursor_moves = 0usize;
        let mut transport_failures = 0usize;
        let mut unparseable = 0usize;
        let mut refusals = 0usize;
        let mut last: Option<ResponseBody> = None;
        for _frame in 0..frames.len() {
            match session.call(&mut transport, &request, Some(&project)) {
                Ok(ResponseBody::Journal(_)) => {
                    cursor_moves += 1;
                }
                Ok(body) => {
                    last = Some(body);
                    // A non-Journal Ok is possible (the wrong-version frames
                    // above are rejected before this point only if version
                    // matches); classify honestly.
                    unparseable += 0;
                }
                Err(ClientError::Transport) => transport_failures += 1,
                Err(ClientError::Unparseable) => unparseable += 1,
                Err(ClientError::Refused(_)) => refusals += 1,
                Err(other) => panic!("chaos produced an unexpected classification: {other:?}"),
            }
        }
        // Every frame was consumed and classified; nothing was invented.
        assert_eq!(
            cursor_moves + transport_failures + unparseable + refusals,
            frames.len()
        );
        assert!(cursor_moves >= 1);
        assert!(transport_failures >= 1);
        assert!(unparseable >= 1);
        assert!(refusals >= 1);
        // The durable cursor equals the LAST genuinely observed Journal
        // page's next_cursor — fabricated successes would have moved it
        // elsewhere.
        let last_journal = frames
            .iter()
            .rev()
            .find_map(|f| match f {
                Frame::Journal(cursor) => Some(*cursor),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            session.journal_position(&project),
            JournalCursor(last_journal)
        );
        let _ = last;
    }

    #[test]
    fn version_mismatch_after_daemon_replacement_is_not_a_refusal() {
        let mut session = ClientSession::new();
        let request = session.build_request("cmd-1", operation("health")).unwrap();
        let mut transport = Scripted::new(vec![Ok({
            let mut bytes = response_ok(ResponseBody::Hello(
                symbiote_protocol::negotiate(&[CURRENT_VERSION]).unwrap(),
            ));
            // Rewind the version to a stale one: the daemon was replaced
            // under us.
            let mut response: Response = serde_json::from_slice(&bytes).unwrap();
            response.version = ProtocolVersion {
                major: 1,
                minor: 12,
            };
            bytes = serde_json::to_vec(&response).unwrap();
            bytes
        })]);
        assert_eq!(
            session.call(&mut transport, &request, None),
            Err(ClientError::Unparseable)
        );
        let _ = symbiote_protocol::negotiate(&[CURRENT_VERSION]).unwrap();
    }

    use symbiote_domain::{
        Actor, ChangeStream, CommitSha, HostId, NewChangeStream, Provenance, RecordDisposition,
        Revision, RoleContractId, StreamLineage, TaskContractId, Timestamp, VersionedRoleContract,
        VersionedTaskContract,
    };

    fn user() -> symbiote_domain::UserId {
        symbiote_domain::UserId::new("owner").unwrap()
    }
    fn project_record(id: &str, roots: &[&str], lead: &str) -> Project {
        Project {
            id: ProjectId::new(id).unwrap(),
            revision: Revision(0),
            name: format!("Project {id}"),
            owner: user(),
            roots: roots.iter().map(|r| RootId::new(*r).unwrap()).collect(),
            lead: RoleId::new(lead).unwrap(),
            disposition: RecordDisposition::Active,
            provenance: Provenance {
                created_at: Timestamp(10),
                updated_at: Timestamp(10),
                actor: Actor::User(user()),
                external_references: vec![],
            },
        }
    }
    fn root_record(project: &str, id: &str) -> Root {
        Root {
            id: RootId::new(id).unwrap(),
            project_id: ProjectId::new(project).unwrap(),
            revision: Revision(0),
            repository: None,
            host_paths: BTreeMap::new(),
        }
    }
    fn role_record(project: &str, id: &str) -> Role {
        Role {
            id: RoleId::new(id).unwrap(),
            project_id: ProjectId::new(project).unwrap(),
            revision: Revision(0),
            name: "Engineer".into(),
            operating_contract: VersionedRoleContract {
                id: RoleContractId::new("role-contract").unwrap(),
                revision: Revision(1),
            },
        }
    }
    fn task_fixture(project: &str, task: &str) -> (Task, ChangeStream) {
        let project_id = ProjectId::new(project).unwrap();
        let root_id = RootId::new(format!("root-{project}")).unwrap();
        let role_id = RoleId::new(format!("role-{project}")).unwrap();
        let task_id = TaskId::new(task).unwrap();
        let stream_id = symbiote_domain::ChangeStreamId::new(format!("stream-{task}")).unwrap();
        let record = Task::new(
            task_id.clone(),
            project_id.clone(),
            root_id.clone(),
            role_id,
            stream_id.clone(),
            VersionedTaskContract {
                id: TaskContractId::new("task-contract").unwrap(),
                revision: Revision(1),
            },
        );
        let stream = ChangeStream::new(NewChangeStream {
            id: stream_id,
            project_id,
            root_id,
            tasks: [task_id].into(),
            originating_chat: symbiote_domain::ChatId::new(format!("chat-{task}")).unwrap(),
            worktree: symbiote_domain::WorktreeId::new(format!("worktree-{task}")).unwrap(),
            branch: format!("branch-{task}"),
            lineage: StreamLineage::Independent,
            base: CommitSha::new("a".repeat(40)).unwrap(),
            target: CommitSha::new("b".repeat(40)).unwrap(),
        })
        .unwrap();
        (record, stream)
    }
    fn snapshot_body(project: &str, cursor: u64, tasks: Vec<Task>) -> ResponseBody {
        ResponseBody::Snapshot(Box::new(ProjectSnapshot {
            project: project_record(
                project,
                &[&format!("root-{project}")],
                &format!("role-{project}"),
            ),
            roots: vec![root_record(project, &format!("root-{project}"))],
            roles: vec![role_record(project, &format!("role-{project}"))],
            tasks,
            cursor: JournalCursor(cursor),
        }))
    }
    fn event(sequence: u64, project: &str, payload: EventPayload) -> JournalEvent {
        JournalEvent {
            sequence,
            project_id: ProjectId::new(project).unwrap(),
            command_id: CommandId::new(format!("cmd-{sequence}")).unwrap(),
            revision: Revision(0),
            payload,
        }
    }
    fn registered(project: &str) -> EventPayload {
        EventPayload::ProjectRegistered {
            project: project_record(
                project,
                &[&format!("root-{project}")],
                &format!("role-{project}"),
            ),
            roots: vec![root_record(project, &format!("root-{project}"))],
            roles: vec![role_record(project, &format!("role-{project}"))],
        }
    }
    fn created(project: &str, task: &str) -> EventPayload {
        let (task, stream) = task_fixture(project, task);
        EventPayload::TaskCreated {
            task: Box::new(task),
            stream: Box::new(stream),
            origin: None,
        }
    }
    fn page_body(events: Vec<JournalEvent>, next_cursor: u64, has_more: bool) -> ResponseBody {
        ResponseBody::Journal(symbiote_protocol::JournalPage {
            events,
            next_cursor: JournalCursor(next_cursor),
            has_more,
        })
    }
    fn sent_after(frame: &[u8]) -> u64 {
        match serde_json::from_slice::<Request>(frame).unwrap().operation {
            symbiote_protocol::Operation::ReadJournal { after, .. } => after.0,
            other => panic!("expected a read_journal request, got {other:?}"),
        }
    }

    #[test]
    fn bootstrap_seeds_the_state_from_the_snapshot_and_pins_the_cursor() {
        let project = project();
        let (task, _) = task_fixture("proj", "task-one");
        let mut session = ClientSession::new();
        let mut transport = Scripted::new(vec![Ok(response_ok(snapshot_body(
            "proj",
            7,
            vec![task.clone()],
        )))]);
        let state = session
            .bootstrap(&mut transport, "boot-1", &project)
            .unwrap();
        assert_eq!(
            state.project(),
            Some(&project_record("proj", &["root-proj"], "role-proj"))
        );
        assert_eq!(state.roots().len(), 1);
        assert_eq!(state.roles().len(), 1);
        assert_eq!(state.tasks().get(task.id()), Some(&task));
        // The durable cursor is the snapshot's own cursor: resume continues
        // from the exact point the records were read at.
        assert_eq!(session.journal_position(&project), JournalCursor(7));
        // The request was the snapshot operation for this Project, and it is
        // not a mutation: nothing else was sent.
        let sent: Request = serde_json::from_slice(&transport.sent[0]).unwrap();
        assert!(matches!(
            sent.operation,
            symbiote_protocol::Operation::Snapshot { ref project_id } if project_id == &project
        ));
        assert_eq!(transport.sent.len(), 1);
    }

    #[test]
    fn bootstrap_refuses_a_snapshot_naming_another_project_and_leaves_the_cursor() {
        let project = project();
        let mut session = ClientSession::new();
        let mut transport = Scripted::new(vec![Ok(response_ok(snapshot_body("other", 4, vec![])))]);
        assert_eq!(
            session.bootstrap(&mut transport, "boot-2", &project),
            Err(ClientError::Unparseable)
        );
        assert_eq!(session.journal_position(&project), JournalCursor(0));
    }

    #[test]
    fn resume_applies_every_page_from_the_durable_cursor_and_is_bounded() {
        let project = project();
        let mut state = ClientState::default();
        state.seed(&match snapshot_body("proj", 0, vec![]) {
            ResponseBody::Snapshot(snapshot) => *snapshot,
            _ => unreachable!(),
        });
        let mut session = ClientSession::new();
        let mut transport = Scripted::new(vec![
            Ok(response_ok(page_body(
                vec![
                    event(1, "proj", registered("proj")),
                    event(2, "proj", created("proj", "task-one")),
                ],
                2,
                true,
            ))),
            Ok(response_ok(page_body(
                vec![event(3, "proj", created("proj", "task-two"))],
                3,
                false,
            ))),
        ]);
        let outcome = session
            .resume(&mut transport, &project, &mut state, 10, 8, "resume")
            .unwrap();
        assert_eq!(outcome.pages, 2);
        assert_eq!(outcome.applied, 3);
        assert_eq!(outcome.not_modeled, 0);
        assert_eq!(outcome.duplicates, 0);
        assert_eq!(outcome.stale_pages, 0);
        assert!(outcome.caught_up);
        assert_eq!(outcome.cursor, JournalCursor(3));
        assert_eq!(session.journal_position(&project), JournalCursor(3));
        assert_eq!(state.tasks().len(), 2);
        // Each page asked from where the previous one ended, and each page is
        // a distinct request carrying its own command id.
        assert_eq!(sent_after(&transport.sent[0]), 0);
        assert_eq!(sent_after(&transport.sent[1]), 2);
        let first: Request = serde_json::from_slice(&transport.sent[0]).unwrap();
        let second: Request = serde_json::from_slice(&transport.sent[1]).unwrap();
        assert_ne!(first.command_id, second.command_id);

        // A resume stopped by its page bound reports it rather than claiming
        // the tail was read.
        let mut state = ClientState::default();
        let mut session = ClientSession::new();
        let mut transport = Scripted::new(vec![Ok(response_ok(page_body(
            vec![event(1, "proj", registered("proj"))],
            1,
            true,
        )))]);
        let outcome = session
            .resume(&mut transport, &project, &mut state, 10, 1, "bounded")
            .unwrap();
        assert_eq!(outcome.pages, 1);
        assert!(!outcome.caught_up);
        assert_eq!(outcome.cursor, JournalCursor(1));
    }

    #[test]
    fn delayed_pages_are_counted_and_never_applied_twice() {
        let project = project();
        let mut state = ClientState::default();
        // The session already read this Project up to cursor 5.
        let mut session = ClientSession::new().with_positions(vec![JournalPosition {
            project: project.clone(),
            cursor: JournalCursor(5),
        }]);
        let mut transport = Scripted::new(vec![Ok(response_ok(page_body(
            vec![
                event(4, "proj", created("proj", "task-one")),
                event(5, "proj", registered("proj")),
            ],
            5,
            false,
        )))]);
        let outcome = session
            .resume(&mut transport, &project, &mut state, 10, 4, "delayed")
            .unwrap();
        assert_eq!(outcome.applied, 0);
        assert_eq!(outcome.duplicates, 2);
        assert_eq!(outcome.stale_pages, 1);
        assert!(!outcome.caught_up);
        assert_eq!(outcome.cursor, JournalCursor(5));
        assert_eq!(session.journal_position(&project), JournalCursor(5));
        // Nothing was applied: the delayed page is observable, not silently
        // replayed over state the caller already had.
        assert!(state.tasks().is_empty());
        assert!(state.project().is_none());
    }

    #[test]
    fn an_out_of_order_page_is_refused_with_nothing_applied() {
        let project = project();
        let mut state = ClientState::default();
        let mut session = ClientSession::new();
        let mut transport = Scripted::new(vec![Ok(response_ok(page_body(
            vec![
                event(7, "proj", registered("proj")),
                event(6, "proj", created("proj", "task-one")),
            ],
            6,
            false,
        )))]);
        assert_eq!(
            session.resume(&mut transport, &project, &mut state, 10, 4, "ordered"),
            Err(ClientError::Unparseable)
        );
        assert_eq!(session.journal_position(&project), JournalCursor(0));
        assert!(state.project().is_none());
        assert!(state.tasks().is_empty());
    }

    #[test]
    fn a_transport_failure_mid_resume_keeps_the_cursor_at_the_last_applied_event() {
        let project = project();
        let mut state = ClientState::default();
        let mut session = ClientSession::new();
        let mut transport = Scripted::new(vec![
            Ok(response_ok(page_body(
                vec![event(1, "proj", registered("proj"))],
                1,
                true,
            ))),
            Err(TransportFailure),
        ]);
        assert_eq!(
            session.resume(&mut transport, &project, &mut state, 10, 4, "first"),
            Err(ClientError::Transport)
        );
        // What was applied stays applied; the cursor names the last applied
        // event, so the next resume continues from exactly there.
        assert_eq!(session.journal_position(&project), JournalCursor(1));
        assert!(state.project().is_some());
        let mut transport = Scripted::new(vec![Ok(response_ok(page_body(
            vec![event(2, "proj", created("proj", "task-one"))],
            2,
            false,
        )))]);
        let outcome = session
            .resume(&mut transport, &project, &mut state, 10, 4, "second")
            .unwrap();
        assert!(outcome.caught_up);
        assert_eq!(sent_after(&transport.sent[0]), 1);
        assert_eq!(outcome.cursor, JournalCursor(2));

        // The state a reconnect reaches is the state an uninterrupted resume
        // reaches: the durable cursor carried the position across the failure.
        let mut uninterrupted = ClientState::default();
        let mut session = ClientSession::new();
        let mut transport = Scripted::new(vec![Ok(response_ok(page_body(
            vec![
                event(1, "proj", registered("proj")),
                event(2, "proj", created("proj", "task-one")),
            ],
            2,
            false,
        )))]);
        let outcome = session
            .resume(&mut transport, &project, &mut uninterrupted, 10, 4, "whole")
            .unwrap();
        assert!(outcome.caught_up);
        assert_eq!(state, uninterrupted);
    }

    #[test]
    fn the_reducer_models_the_snapshot_kinds_and_names_the_rest() {
        let mut state = ClientState::default();
        assert_eq!(
            state.apply(&event(1, "proj", registered("proj"))),
            EventOutcome::Applied
        );
        assert_eq!(state.project().unwrap().name, "Project proj");
        assert_eq!(state.roots().len(), 1);
        assert_eq!(state.roles().len(), 1);
        // A placement replaces the Root record it names, including the
        // observed path — the reducer applies the event's record, not a
        // guess about it.
        let mut placed = root_record("proj", "root-proj");
        placed
            .host_paths
            .insert(HostId::new("host-one").unwrap(), "/srv/repo".into());
        assert_eq!(
            state.apply(&event(
                2,
                "proj",
                EventPayload::RootPlacementObserved {
                    root: Box::new(placed.clone()),
                    expected_revision: Revision(0),
                    host_id: HostId::new("host-one").unwrap(),
                    path: "/srv/repo".into(),
                    actor: user(),
                    at: Timestamp(11),
                }
            )),
            EventOutcome::Applied
        );
        assert_eq!(state.roots().get(&placed.id), Some(&placed));
        assert_eq!(
            state.apply(&event(3, "proj", created("proj", "task-one"))),
            EventOutcome::Applied
        );
        assert_eq!(state.tasks().len(), 1);
        // Durable history this state carries no record for is named, never
        // dropped silently and never invented into a record.
        assert_eq!(
            state.apply(&event(
                4,
                "proj",
                EventPayload::TaskDependenciesSet {
                    task_id: TaskId::new("task-one").unwrap(),
                    project_id: ProjectId::new("proj").unwrap(),
                    edges: vec![],
                    actor: user(),
                    at: Timestamp(12),
                }
            )),
            EventOutcome::NotModeled
        );
        assert_eq!(state.tasks().len(), 1);
        // Another Project's history cannot be absorbed into this state.
        let before = state.clone();
        assert_eq!(
            state.apply(&event(5, "other", registered("other"))),
            EventOutcome::ForeignProject
        );
        assert_eq!(state, before);
    }

    #[test]
    fn bootstrap_then_resume_reaches_the_state_a_cold_replay_reaches() {
        let project = project();
        let log = vec![
            event(1, "proj", registered("proj")),
            event(2, "proj", created("proj", "task-one")),
            event(3, "proj", created("proj", "task-two")),
        ];
        // Cold: apply the whole log from the beginning.
        let mut cold = ClientState::default();
        for entry in &log {
            assert_eq!(cold.apply(entry), EventOutcome::Applied);
        }
        // Bootstrap: seed from the snapshot at cursor 1 (the state the
        // registration left) and resume the events after it.
        let mut session = ClientSession::new();
        let mut transport = Scripted::new(vec![
            Ok(response_ok(snapshot_body("proj", 1, vec![]))),
            Ok(response_ok(page_body(log[1..].to_vec(), 3, false))),
        ]);
        let mut booted = session.bootstrap(&mut transport, "boot", &project).unwrap();
        let outcome = session
            .resume(&mut transport, &project, &mut booted, 10, 4, "catch-up")
            .unwrap();
        assert!(outcome.caught_up);
        assert_eq!(outcome.applied, 2);
        assert_eq!(outcome.cursor, JournalCursor(3));
        // The two paths agree record for record: a bootstrap is not a
        // shortcut that quietly drops what a replay would have built.
        assert_eq!(booted, cold);
        assert_eq!(session.journal_position(&project), JournalCursor(3));
    }
}
