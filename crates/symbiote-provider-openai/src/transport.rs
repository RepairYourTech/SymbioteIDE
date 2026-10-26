//! The transport: one explicitly configured endpoint, one credential, one
//! allowlist, and the rule that the credential never leaves the origin it was
//! configured for.
//!
//! The endpoint is exact-match allowlisted — a host is allowed because the
//! operator named it, not because its name ends like one that was named, which is
//! the difference between `api.example.com` and `api.example.com.evil.test`. The
//! credential is supplied per request through a `CredentialSource` the adapter
//! never logs, never serializes and never places in an error, and it is written
//! only to the origin the endpoint names. A redirect — any 3xx — is refused as a
//! policy refusal, not followed, because following one is how a credential walks
//! to another origin while the adapter believes it is still where it was pointed
//! (#471: "secrets cannot follow cross-origin redirects"; here: no redirects at
//! all, which is the same rule held stricter than the words).
//!
//! Statuses are classified into the neutral vocabulary and the body of a refused
//! response is dropped with them: the provider's error prose is the provider's,
//! and the neutral error is payload-free by design. 401 and 403 are refused
//! credentials; 408, 429, 500, 502, 503 and 504 are faults a retry may answer —
//! the rate-limit case among them — and everything else is a request the
//! provider refused on its own terms.
//!
//! A streamed response is read to the connection's end and its server-sent events
//! are fed to the [`StreamAssembler`] piece by piece, so tool-call arguments that
//! arrive fragmented across chunks are the assembler's problem and not the loop's.
//! A connection that dies before the provider named its finish reason is an error
//! (`Unavailable`), never a fabricated completion — an interrupted generation is
//! not a completed one, and handing the loop a partial answer as a whole one is
//! how a retried run re-executes a tool the first attempt already committed.
use serde_json::Value;
use std::io::{Read, Write};
use std::time::Duration;

use symbiote_native_agent::InferenceTransport;
use symbiote_runtime_sdk::provider::{
    ModelDescriptor, ProviderError, ProviderRequest, ProviderResponse,
};

use crate::catalog::ProviderCatalog;
use crate::wire::{self, StreamAssembler};

/// The hosts the operator allows, exactly matched. No suffix rule exists on
/// purpose: `api.example.com` matching an entry `example.com` is how a
/// credential is pointed at a host the operator never named.
#[derive(Clone, Debug, Default)]
pub struct AllowedHosts {
    hosts: std::collections::BTreeSet<String>,
}

impl AllowedHosts {
    pub fn new<I, S>(hosts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            hosts: hosts.into_iter().map(Into::into).collect(),
        }
    }

    fn permits(&self, host: &str) -> bool {
        self.hosts.contains(host)
    }
}

/// Where the connection's credential comes from. A closure over the Host's
/// brokered lease: the adapter never holds the value longer than one request,
/// never formats it into an error, and never writes it anywhere but the
/// request line it belongs on.
pub trait CredentialSource: Send {
    fn credential(&self) -> Result<String, ProviderError>;
}

/// A fixed credential, for operator-configured local endpoints and fixtures.
pub struct FixedCredential(String);

impl FixedCredential {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl CredentialSource for FixedCredential {
    fn credential(&self) -> Result<String, ProviderError> {
        Ok(self.0.clone())
    }
}

/// The origin the credential is bound to: host, port, and the path prefix the
/// provider's API sits under. Every part is exact — the path must be absolute
/// and may carry no scheme, authority or control character, because a path is
/// the one part of a URL this adapter lets a configuration write.
#[derive(Clone, Debug)]
pub struct Endpoint {
    host: String,
    port: u16,
    path: String,
}

impl Endpoint {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        path: impl Into<String>,
    ) -> Result<Self, ProviderError> {
        let host: String = host.into();
        let path: String = path.into();
        let refused = host.is_empty()
            || !path.starts_with('/')
            || path.contains("://")
            || path
                .bytes()
                .any(|byte| byte == b'\r' || byte == b'\n' || byte == b' ');
        if refused {
            return Err(ProviderError::InvalidEnvelope);
        }
        Ok(Self { host, port, path })
    }

    pub fn host(&self) -> &str {
        &self.host
    }
}

/// The OpenAI-compatible transport over one endpoint.
pub struct OpenAiTransport {
    endpoint: Endpoint,
    credential: Box<dyn CredentialSource>,
    catalog: ProviderCatalog,
    /// The whole-request wall the adapter holds itself to. A provider that
    /// cannot answer within it leaves the run to the loop's retry, not to a
    /// connection the Host waits on forever.
    deadline: Duration,
}

impl OpenAiTransport {
    pub fn new(
        endpoint: Endpoint,
        allowed: AllowedHosts,
        credential: Box<dyn CredentialSource>,
        catalog: ProviderCatalog,
        deadline: Duration,
    ) -> Result<Self, ProviderError> {
        // An endpoint the allowlist does not name is refused at construction,
        // so a misconfiguration is a startup failure rather than a refusal
        // discovered mid-run with a dispatch already holding the lease.
        if !allowed.permits(endpoint.host()) {
            return Err(ProviderError::LocalPolicyRequired);
        }
        Ok(Self {
            endpoint,
            credential,
            catalog,
            deadline,
        })
    }

    /// The catalog gate, run before any byte is written. Public so a caller
    /// can reject a request it is only inspecting, and so the fixtures can
    /// prove the provider never saw the refused ones.
    pub fn authorize(&self, request: &ProviderRequest) -> Result<(), ProviderError> {
        self.catalog.authorize(request)
    }

    fn classify(status: u16) -> ProviderError {
        match status {
            401 | 403 => ProviderError::CredentialRequired,
            408 | 429 | 500 | 502 | 503 | 504 => ProviderError::Unavailable,
            _ => ProviderError::InvalidEnvelope,
        }
    }

    fn exchange(&self, request_body: &serde_json::Value) -> Result<String, ProviderError> {
        let credential = self.credential.credential()?;
        let body = serde_json::to_vec(request_body).map_err(|_| ProviderError::InvalidEnvelope)?;
        let mut stream =
            std::net::TcpStream::connect((self.endpoint.host.as_str(), self.endpoint.port))
                .map_err(|_| ProviderError::Unavailable)?;
        stream
            .set_read_timeout(Some(self.deadline))
            .map_err(|_| ProviderError::Unavailable)?;
        stream
            .set_write_timeout(Some(self.deadline))
            .map_err(|_| ProviderError::Unavailable)?;
        let request = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.endpoint.path,
            self.endpoint.host,
            credential,
            body.len()
        );
        stream
            .write_all(request.as_bytes())
            .and_then(|_| stream.write_all(&body))
            .map_err(|_| ProviderError::Unavailable)?;
        let mut raw = Vec::new();
        stream
            .take_remaining(&mut raw)
            .map_err(|_| ProviderError::Unavailable)?;
        let header_end = raw
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or(ProviderError::InvalidEnvelope)?;
        let head = String::from_utf8_lossy(&raw[..header_end]).into_owned();
        let status: u16 = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|code| code.parse().ok())
            .ok_or(ProviderError::InvalidEnvelope)?;
        if (300..400).contains(&status) {
            // Refused, never followed: the credential was written to exactly
            // one origin, and it stays bound to that origin's answer.
            return Err(ProviderError::LocalPolicyRequired);
        }
        if status != 200 {
            return Err(Self::classify(status));
        }
        Ok(String::from_utf8_lossy(&raw[header_end + 4..]).into_owned())
    }

    /// Extracts each `data:` event's JSON payload from a server-sent-event
    /// body. Lines other than `data:` (comments, event names, blank lines)
    /// are the format's furniture and are skipped; `[DONE]` ends the stream.
    fn sse_events(body: &str) -> Vec<Value> {
        let mut events = Vec::new();
        for line in body.lines() {
            let Some(payload) = line.strip_prefix("data:") else {
                continue;
            };
            let payload = payload.trim();
            if payload == "[DONE]" {
                break;
            }
            if let Ok(event) = serde_json::from_str(payload) {
                events.push(event);
            }
        }
        events
    }
}

/// `Read::read_to_end` under another name, so the import list above stays
/// honest about which reads this adapter performs.
trait TakeRemaining {
    fn take_remaining(&mut self, out: &mut Vec<u8>) -> std::io::Result<()>;
}

impl TakeRemaining for std::net::TcpStream {
    fn take_remaining(&mut self, out: &mut Vec<u8>) -> std::io::Result<()> {
        self.read_to_end(out).map(|_| ())
    }
}

impl InferenceTransport for OpenAiTransport {
    fn request(
        &mut self,
        request: &ProviderRequest,
        model: &ModelDescriptor,
    ) -> Result<ProviderResponse, ProviderError> {
        // Before any byte: the catalog answers for the model's bounds and
        // capabilities, the contract answers for the envelope.
        self.authorize(request)?;
        request.validate(model)?;
        let encoded = wire::encode_request(request);
        let body = self.exchange(&encoded)?;
        if request.streaming {
            let mut assembler = StreamAssembler::default();
            for event in Self::sse_events(&body) {
                assembler.feed(&event)?;
            }
            // A stream that ended without its finish reason is an interrupted
            // generation, and the adapter refuses it rather than assembling
            // the pieces into a completion nobody finished.
            if !assembler.finished() {
                return Err(ProviderError::Unavailable);
            }
            assembler.assemble(request, model)
        } else {
            let parsed: serde_json::Value =
                serde_json::from_str(&body).map_err(|_| ProviderError::InvalidEnvelope)?;
            wire::decode_response(&parsed, request, model)
        }
    }
}
