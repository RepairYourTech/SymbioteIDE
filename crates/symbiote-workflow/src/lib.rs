//! The desktop workflow shell (#54): the control-plane driver that turns
//! the daemon's existing capabilities into the first-release demonstration
//! — open a repository, describe a coding task, execute it through a
//! started dispatch, and inspect the diff and completion evidence — over
//! `symbiote-client-sdk`, with restart/resume proven against the daemon's
//! durable journal.
//!
//! Boundary: this crate is a SEQUENCER over the protocol. It grants
//! nothing, executes nothing, and completes nothing — worker completion is
//! evidence, and Host verification plus independent review remain the
//! completion gates (the first-release demo stops at `CompletionRequested`).
//! The model turn in the daemon-side demo configuration is the operator's
//! explicitly labeled fixture transport; a live model turn remains gated on
//! explicit user authorization for credentials and billing. The worktree
//! status this crate reads is a real filesystem fact of what the run
//! produced through the sandboxed shell executor.
pub mod demo;
pub mod socket;

pub use demo::DemoWorkflow;

use std::path::{Path, PathBuf};
use symbiote_client_sdk::{ClientError, ClientSession, FrameExchange, TransportFailure};
use symbiote_protocol::ResponseBody;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkflowError {
    /// The daemon refused a step; the typed protocol error is preserved
    /// for the caller to branch on.
    Refused(symbiote_protocol::ProtocolError),
    /// The transport failed; the command's disposition is unknown and
    /// [`Driver::recover`] replays the command id.
    Transport,
    /// The response did not parse, or the daemon's version moved under us.
    Unparseable,
    /// The operation or command id was refused client-side (never sent).
    InvalidOperation,
    /// The step's response did not carry the body the sequence requires —
    /// a daemon composition bug, surfaced loudly rather than guessed at.
    UnexpectedBody,
    /// Local filesystem/socket framing failure before anything was sent.
    Socket,
}

impl From<ClientError> for WorkflowError {
    fn from(error: ClientError) -> Self {
        match error {
            ClientError::Refused(error) => Self::Refused(error),
            ClientError::Transport => Self::Transport,
            ClientError::Unparseable => Self::Unparseable,
            // The client-side envelope/operation validation: surface the
            // real identity instead of collapsing it into Unparseable.
            ClientError::RequestTooLarge
            | ClientError::InvalidCommandId
            | ClientError::InvalidOperation => Self::InvalidOperation,
        }
    }
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "workflow: {self:?}")
    }
}
impl std::error::Error for WorkflowError {}

/// One connection's durable journal position, restored across driver
/// restarts via [`ClientSession::with_positions`].
pub struct Driver {
    session: ClientSession,
    exchange: SocketExchange,
}

impl Driver {
    /// Connects to the daemon serving `state_directory/host.sock`.
    pub fn connect(state_directory: &Path) -> Result<Self, WorkflowError> {
        Ok(Self {
            session: ClientSession::new(),
            exchange: SocketExchange::new(state_directory)?,
        })
    }

    /// Restores journal positions after a driver restart.
    pub fn with_positions(mut self, positions: Vec<symbiote_client_sdk::JournalPosition>) -> Self {
        self.session = self.session.with_positions(positions);
        self
    }

    pub fn journal_position(
        &self,
        project: &symbiote_domain::ProjectId,
    ) -> symbiote_protocol::JournalCursor {
        self.session.journal_position(project)
    }

    /// Sends one operation; a transport failure is recovered by replaying
    /// the same command id (the daemon's journal returns the durable
    /// receipt). Refusals are surfaced, never retried.
    pub fn call(
        &mut self,
        command_id: &str,
        operation: serde_json::Value,
        project: Option<&symbiote_domain::ProjectId>,
    ) -> Result<ResponseBody, WorkflowError> {
        let request = self
            .session
            .build_request(command_id, operation)
            .map_err(WorkflowError::from)?;
        match self.session.call(&mut self.exchange, &request, project) {
            Ok(body) => Ok(body),
            Err(ClientError::Transport) => self
                .session
                .recover(&mut self.exchange, &request, project, 2)
                .map_err(WorkflowError::from),
            Err(error) => Err(error.into()),
        }
    }
}

/// The injected socket transport.
pub struct SocketExchange {
    directory: PathBuf,
}

impl SocketExchange {
    pub fn new(state_directory: &Path) -> Result<Self, WorkflowError> {
        Ok(Self {
            directory: state_directory.to_path_buf(),
        })
    }
}

impl FrameExchange for SocketExchange {
    fn exchange(&mut self, request: &[u8]) -> Result<Vec<u8>, TransportFailure> {
        socket::exchange(&self.directory, request).map_err(|_| TransportFailure)
    }
}

/// The current worktree status, observed from the REAL filesystem via
/// `symbiote-repo` — the run's diff evidence. The lists are the uncommitted
/// and untracked paths the run produced inside the reserved worktree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeEvidence {
    pub worktree: PathBuf,
    pub uncommitted: Vec<String>,
    pub untracked: Vec<String>,
}

impl WorktreeEvidence {
    /// Whether the run's evidence includes a specific produced path.
    pub fn contains(&self, path: &str) -> bool {
        self.uncommitted.iter().any(|p| p == path) || self.untracked.iter().any(|p| p == path)
    }
}

/// Observes the run's worktree: the derived location under the operator's
/// reservation base, from the same public derivation the provisioning
/// uses (stream-id digest seed → derived worktree name).
pub fn observe_worktree_evidence(
    reservation_base: &Path,
    project_id: &symbiote_domain::ProjectId,
    root_id: &symbiote_domain::RootId,
    stream_id: &symbiote_domain::ChangeStreamId,
) -> Result<WorktreeEvidence, WorkflowError> {
    let digest = symbiote_trust::Fingerprint::of(stream_id.as_str().as_bytes());
    let seed =
        symbiote_worktrees::policy_seed(digest.as_str()).map_err(|_| WorkflowError::Socket)?;
    let derived = symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
        project_id,
        root_id,
        stream_id,
        seed,
    })
    .map_err(|_| WorkflowError::Socket)?;
    let worktree = derived.worktree_path(reservation_base);
    let mut git = symbiote_repo::SystemGit::new();
    let status =
        symbiote_repo::observe_status(&mut git, &worktree).map_err(|_| WorkflowError::Socket)?;
    Ok(WorktreeEvidence {
        worktree,
        uncommitted: status.uncommitted,
        untracked: status.untracked,
    })
}
