# Inference provider contracts — #464 foundation

`symbiote_runtime_sdk::provider` separates a model-turn adapter from the agent-loop `AgentRuntimeAdapter`. `InferenceProviderAdapter` uses the existing domain `InferenceProviderAdapterId`; runtime adapters use `AgentRuntimeAdapterId`. Neither trait is a control plane. This module performs no network requests, tool execution, credential reads, billing operations or canonical Task completion. It contains no provider implementation and establishes no live model compatibility.

## Identity, versions and capabilities

`ModelDescriptor` references canonical `ModelId` and `ProviderConnectionId`, token context/output limits and explicit reasoning/tool/image/streaming capabilities. Requests name the same identities; responses must preserve them and the request correlation. Capabilities are supplied facts, not guessed from a model name. Unsupported effort/capability requests fail explicitly. The reasoning vocabulary is none/minimal/low/medium/high/x_high/max/ultra; every explicit value requires model advertisement, including none. An omitted effort makes no explicit selection. These names do not assert any provider's wire mapping or support; additional mappings require verified adapter behavior rather than silent substitution.

`PROVIDER_CONTRACT_VERSION = 1` versions these provider envelopes independently from canonical domain schemas, Host RPC and native agent/external harness state. Unknown versions fail. There is no invented migration from a historical provider schema and no automatic rewrite of stored native or external session state.

## Authentication and billing

`validate_binding` requires consistent RuntimeProfile/provider/model/credential/entitlement identities and an unexpired entitlement. An expiry equal to the current time is expired. Resulting `ProviderBinding` is read-only metadata and intentionally cannot be deserialized as authority; it retains the endpoint reference and only a credential *identity*, never its vault key or secret value. Recheck expiry at request execution, not only when loading configuration.

Supported mappings are deliberately explicit:

| Authentication | Billing | Runtime | Credential input |
| --- | --- | --- | --- |
| API credential | Metered API | Native or external | Matching credential reference required |
| Harness managed | Harness subscription | External only | Must be absent |
| Local unauthenticated | Local | Native or external | Must be absent; trusted local policy required |

Native bindings reject harness-managed authentication and harness subscriptions. External subscription bindings reject passed native credentials, preventing this boundary from silently copying a native credential reference into harness-managed work. Unsupported combinations fail rather than switching billing methods. A symbolic credential ID remains in the existing domain RuntimeProfile even for no-secret mappings; it is not resolved or leased in those cases.

`ProviderPolicy` is a non-wire Host input mapping verified-local provider IDs to exact endpoint references. Changing an endpoint invalidates that local-policy match. Endpoint locality, entitlement verification evidence authenticity, credential ownership/access and the actual runtime loop owner remain Host responsibilities; a typed ID or a localhost-looking string does not prove them. An all-native local binding needs neither a secret nor an installed third-party CLI. This is a configuration fixture, not startup/process proof.

## Bounded model-turn envelopes

`ProviderRequest` accepts at most 128 messages, 32 parts per message and 64 uniquely named tools. Images reference Host-resolved artifact IDs, not arbitrary remote URLs. Model tool schemas remain declarations and do not grant permission. JSON schema/arguments are bounded to depth 32 and 4,096 nodes; each complete serialized envelope is capped at 256 KiB. Parsing checks byte size first and rejects unknown fields. Programmatic validation counts serialization bytes without creating a second unbounded output buffer.

The request's input-token budget is a Context Broker upper-bound estimate, distinct from measured usage. Input budget plus reserved maximum output must fit the declared context window using checked arithmetic. Maximum output must be positive and fit the model's output limit. This module does not implement tokenization or prove that an estimator is accurate.

`ProviderResponse` contains text, correlated tool-call envelopes, finish reason and explicit `TokenUsage`. Tool-call IDs must be unique, tool names must have been declared, and arguments must be JSON objects. The tool engine must still validate arguments against the tool's actual schema and enforce Host permissions before execution. A provider tool call is a request, never an authorized effect. Cancelled/truncated responses cannot carry actionable tool-call lists under this initial completed-envelope contract.

Usage is `Known` or `Unknown` with a reason (`not_reported`, `interrupted`, `unsupported`). Unknown is never zero. Known cached tokens are a subset of input and reasoning tokens are a subset of output, so totals do not double-count them. Overflow, invalid subsets, context overruns and reported input/output beyond their respective request budgets fail explicitly. Known input above `input_token_budget` returns `InputLimit` even when the model context still has room; equality is allowed. No monetary cost is guessed and no entitlement accounting is performed.

The async trait returns a completed response envelope. A streaming-capability request may be implemented by consuming a provider stream into that bounded result; incremental stream transport, cancellation wiring and partial-response recovery are not implemented here and remain pending. Adapter implementations must validate descriptors/requests/bindings/responses and match their own adapter ID before use. No fake adapter is counted as provider integration evidence.

## Verification and pending scope

Tests cover mixed native/external identity round trips, native rejection of subscription/harness mappings, required API references, external no-copy behavior, exact local policy with no CLI/secret, changed endpoint rejection, entitlement expiry, request/model limits, unsupported capabilities, tool-call validation, strict/oversized envelopes, deep JSON, usage subsets/overflow/unknowns and response correlation. They do not authenticate an account or call a model.

Pending #464 acceptance: actual native runtime integration, verified provider adapters/endpoints, credential-store leasing and ownership enforcement, endpoint/model evidence freshness, full provider authentication/billing compatibility, adapter validation enforcement at live invocation, stream/cancellation/recovery tests, mixed real-runtime executions and denied direct-Host-state writes. Security/privacy applies to every live credential/endpoint/tool boundary. Accessibility is not applicable to this headless contract module; UI reporting remains client work. Contract checks are portable, but actual platform/provider behavior is untested. No state migration, billing change or production rollout occurs in this module.
