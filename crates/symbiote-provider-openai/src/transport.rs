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
    MAX_ENVELOPE_BYTES, ModelDescriptor, ProviderError, ProviderRequest, ProviderResponse,
};

/// What one provider exchange may hold on the wire: the contract's envelope
/// bound plus the HTTP framing around it — headers, chunk delimiters, the SSE
/// line furniture. The read stops at this bound while the bytes are still on
/// the socket.
const MAX_RESPONSE_BYTES: usize = MAX_ENVELOPE_BYTES + 64 * 1024;

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
        // A remote endpoint is refused, on purpose: this transport has no
        // TLS yet, and a bearer credential written to a socket that leaves
        // the machine is a credential published to the network path. The
        // endpoints this adapter serves are the machine's own — a local
        // provider process or a loopback tunnel the operator runs — and
        // remote enablement is the TLS-bearing increment behind AUTH-01,
        // not a flag here.
        let loopback = host == "localhost"
            || host == "127.0.0.1"
            || host == "::1"
            || host.ends_with(".localhost");
        let refused = host.is_empty()
            || !loopback
            || !path.starts_with('/')
            || path.contains("://")
            || path
                .bytes()
                .any(|byte| byte == b'\r' || byte == b'\n' || byte == b' ');
        if refused {
            return Err(ProviderError::LocalPolicyRequired);
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
        // A catalog the reader cannot hold is refused at construction, not
        // discovered mid-run: the transport's every refusal rests on this
        // document being one its reader speaks.
        catalog.validate()?;
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
        // The credential rides one header line, so a credential carrying a
        // line break or a space is a second header nobody configured — or a
        // whole second request. Refused, not sanitized: a credential that
        // needs mangling to be sent is a credential this endpoint must not
        // receive.
        if credential.is_empty()
            || credential
                .bytes()
                .any(|byte| byte <= b' ' || byte >= b'\x7f')
        {
            return Err(ProviderError::CredentialRequired);
        }
        let body = serde_json::to_vec(request_body).map_err(|_| ProviderError::InvalidEnvelope)?;
        // One wall clock for the whole exchange: connect, writes and reads
        // each get the time that is LEFT, so a peer that dribbles a byte
        // inside every socket timeout cannot hold the run forever.
        let started = std::time::Instant::now();
        let remaining = || {
            self.deadline
                .checked_sub(started.elapsed())
                .ok_or(ProviderError::Unavailable)
        };
        let address = std::net::ToSocketAddrs::to_socket_addrs(&(
            self.endpoint.host.as_str(),
            self.endpoint.port,
        ))
        .map_err(|_| ProviderError::Unavailable)?
        .next()
        .ok_or(ProviderError::Unavailable)?;
        let mut stream = std::net::TcpStream::connect_timeout(&address, remaining()?)
            .map_err(|_| ProviderError::Unavailable)?;
        stream
            .set_read_timeout(Some(remaining()?))
            .map_err(|_| ProviderError::Unavailable)?;
        stream
            .set_write_timeout(Some(remaining()?))
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
        // The response is read under the published bound — headers, framing
        // and body together. A provider answer larger than what this adapter
        // can accept is refused while it is still bytes on the socket, not
        // after it has been allocated and parsed.
        let mut raw = Vec::new();
        let mut scratch = [0u8; 8192];
        loop {
            match stream.read(&mut scratch) {
                Ok(0) => break,
                Ok(read) => {
                    if raw.len() + read > MAX_RESPONSE_BYTES {
                        return Err(ProviderError::EnvelopeTooLarge);
                    }
                    raw.extend_from_slice(&scratch[..read]);
                }
                Err(_) => return Err(ProviderError::Unavailable),
            }
            remaining()?;
        }
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
        // The body is what the transfer framing says it is: chunked framing
        // carries its own sizes and delimiters, and handing those bytes to
        // the JSON or event parser is how a body silently loses pieces.
        let raw_body = &raw[header_end + 4..];
        let chunked = head.lines().skip(1).any(|line| {
            line.split_once(':').is_some_and(|(name, value)| {
                name.trim().eq_ignore_ascii_case("transfer-encoding")
                    && value.trim().eq_ignore_ascii_case("chunked")
            })
        });
        if chunked {
            decode_chunked(raw_body)
        } else {
            Ok(String::from_utf8_lossy(raw_body).into_owned())
        }
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

fn decode_chunked(raw: &[u8]) -> Result<String, ProviderError> {
    let mut body = Vec::new();
    let mut cursor = 0;
    loop {
        let line_end = raw[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or(ProviderError::InvalidEnvelope)?
            + cursor;
        let size_text = String::from_utf8_lossy(&raw[cursor..line_end]);
        let size = usize::from_str_radix(size_text.trim().split(';').next().unwrap_or(""), 16)
            .map_err(|_| ProviderError::InvalidEnvelope)?;
        cursor = line_end + 2;
        if size == 0 {
            return Ok(String::from_utf8_lossy(&body).into_owned());
        }
        if cursor + size + 2 > raw.len() {
            return Err(ProviderError::InvalidEnvelope);
        }
        body.extend_from_slice(&raw[cursor..cursor + size]);
        cursor += size;
        // The chunk's own CRLF delimiter, exactly where the size said.
        if raw.get(cursor) != Some(&b'\r') || raw.get(cursor + 1) != Some(&b'\n') {
            return Err(ProviderError::InvalidEnvelope);
        }
        cursor += 2;
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
        let encoded = wire::encode_request(request)?;
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
