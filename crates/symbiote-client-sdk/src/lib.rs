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
use symbiote_domain::{CommandId, ProjectId, RequestId};
use symbiote_protocol::JournalCursor;
use symbiote_protocol::{
    CURRENT_VERSION, MAX_REQUEST_BYTES, ProtocolError, ProtocolVersion, Request, Response,
    ResponseBody, parse_request,
};

/// One connection's durable position in a Project's journal. Resume means:
/// re-read from `cursor` after reconnect; pages advance it only over events
/// actually returned. Cursor 0 means "bootstrap from the beginning" — the
/// caller decides whether a full bootstrap or a tail read is appropriate.
#[derive(Clone, Debug, PartialEq, Eq)]
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

    fn observe_page(&mut self, page: &Value, project: &ProjectId) {
        let Some(next) = page.get("next_cursor").and_then(Value::as_u64) else {
            return;
        };
        match self.positions.iter_mut().find(|p| &p.project == project) {
            Some(position) => position.cursor = JournalCursor(next),
            None => self.positions.push(JournalPosition {
                project: project.clone(),
                cursor: JournalCursor(next),
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

    /// Sends one request and classifies the response. On `Ok`, the typed
    /// body is returned; the SDK also updates journal positions it observes
    /// (a `Journal` body), so resume works without callers plumbing cursors
    /// by hand.
    pub fn call(
        &mut self,
        exchange: &mut impl FrameExchange,
        request: &Request,
        project: Option<&ProjectId>,
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
            Ok(body) => {
                if let (Some(project), symbiote_protocol::ResponseBody::Journal(page)) =
                    (project, &body)
                {
                    // The caller's project must agree with the operation's
                    // own — a mismatched hand-off would advance one
                    // project's cursor with another's next_cursor.
                    if request.operation.project_id() == Some(project) {
                        self.observe_page(
                            &serde_json::json!({
                                "next_cursor": page.next_cursor.0
                            }),
                            project,
                        );
                    }
                }
                Ok(body)
            }
            Err(error) => Err(ClientError::Refused(error)),
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
}
