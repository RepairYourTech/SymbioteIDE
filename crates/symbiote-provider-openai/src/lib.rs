//! The OpenAI-compatible inference adapter: the first `InferenceProviderAdapter`
//! behind the neutral provider contract (#471).
//!
//! `symbiote-runtime-sdk::provider` defines the neutral request/response contract —
//! `ProviderRequest`, `ProviderResponse`, `TokenUsage`, `ReasoningEffort` — and
//! validates both directions of it. This crate is the adapter that speaks one
//! provider wire format on the far side of that contract, so provider shapes stay
//! out of orchestration and orchestration stays out of provider shapes. It carries
//! three things, each in its own module:
//!
//! - The versioned capability catalog (`catalog`): which models the operator's
//!   connection offers, with the context/output bounds and reasoning efforts each
//!   actually supports. A request that names an unknown model, an unsupported
//!   effort, an over-budget context or an over-budget output is refused BEFORE any
//!   byte reaches the network — invalid effort and oversized context are rejected
//!   before inference rather than silently coerced (#471's own acceptance line).
//! - The wire mapping (`wire`): the neutral request encoded as the provider's
//!   chat-completions JSON, and the provider's response — whole or streamed as
//!   server-sent events, with tool-call arguments arriving fragmented across
//!   chunks — decoded back into the neutral shapes. Usage preserves the
//!   provider's own semantics: cached tokens are a subset of input and reasoning
//!   tokens a subset of output, both recorded beside the totals rather than
//!   counted into them, and usage the provider never reported stays
//!   `TokenUsage::Unknown` rather than becoming a zero.
//! - The transport (`transport`): an HTTP/1.1 client for one explicitly
//!   configured endpoint, carrying the connection's credential to exactly that
//!   endpoint. Redirects are refused rather than followed, so a credential cannot
//!   be walked to another origin; hosts are exact-match allowlisted, so a suffix
//!   look-alike cannot pass; errors are classified into the neutral vocabulary —
//!   rate limits and provider faults as `Unavailable`, refused credentials as
//!   `CredentialRequired`, refused endpoints as `LocalPolicyRequired` — and a
//!   stream that ends before the provider finished produces an error, never a
//!   fabricated completion.
//!
//! Local endpoint support is protocol-tested here against an in-process server,
//! not assumed from an OpenAI-compatible label (#471's own requirement). No live
//! provider is contacted by this crate's tests; live enablement remains gated on
//! AUTH-01 evidence and the operator's explicit authorization.
pub mod catalog;
pub mod transport;
pub mod wire;

pub use catalog::{CATALOG_CONTRACT_VERSION, CatalogModel, ProviderCatalog};
pub use transport::{AllowedHosts, CredentialSource, Endpoint, FixedCredential, OpenAiTransport};
pub use wire::StreamAssembler;
