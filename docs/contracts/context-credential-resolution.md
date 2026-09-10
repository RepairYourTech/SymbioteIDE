# Context and credential resolution — dispatch runs (#217 slice)

`symbiote-context` is the typed bridge between a started dispatch's trusted
Host state and the exact payload a worker run receives. It resolves two
halves the run must never conflate:

- **Context (what the run sees).** `resolve_context` composes the origin
  work item's description, requirements, constraints, risks and acceptance
  — plus the dispatch contract's `ContextPolicy` budget and `AccessSnapshot`
  — into a `ResolvedContext`, and `render_prompt` renders it as the task
  prompt the loop receives. Every fact comes from the store through the
  `ContextSource` adapter (`symbiote-host::context_resolution`); caller
  input cannot steer it. All text is bounded per item (8 KiB with a
  visible truncation marker) and per list (64 items, 64 KiB total);
  structural overflow is a typed refusal (`Overbound`), never silent
  narrowing. The daemon's `run_started_dispatch` resolves this for BOTH
  runtimes — native and external receive the same resolved prompt.

- **Credentials (what the run may use).** `CredentialBroker` is the
  operator's process-local registry: secrets registered per
  `CredentialReferenceId` with an owning project and an environment label
  (the label is the materialization NAME, not an authorization surface).
  `resolve` issues a short-lived `CredentialLease` (5-minute window,
  expiry-checked at materialization with distinct `Expired`/`NotYetValid`
  identities) only when the dispatch's profile references the credential,
  the owning project matches the dispatch's project, AND the binding
  access grants `Permission::UseCredential`, OR an explicit elevation
  lease (#269) that the owner decided for THIS dispatch and whose window
  is still open at the run's clock — the domain's typed authority that a
  credential lease may issue at all. A native run that is refused with
  `use_credential_not_granted` journals a worker `request_elevation` as
  evidence of the ask and then returns the same typed refusal — the ask
  never satisfies this gate and never licenses a lease. Attestation honesty: when the basis
  is the binding grant, the dispatch compiler requires `Control::Credentials`
  host enforcement (the claim is in the contract); when the basis is an
  elevation, the contract carries no `Credentials` claim — the binding
  did not grant the permission at compile time — so the broker's own
  provisioning check is the enforcing gate and a future materialization
  slice must not key its attestation on the contract's claim alone. Distinct typed refusals: `UnknownReference`, `Revoked`,
  `CrossProjectDenied`, `NotReferencedByProfile`, `UseCredentialNotGranted`,
  `InvalidEnvironmentLabel`, `Expired`, `NotYetValid`. Values are zeroized
  on drop, never serialized (no serde on the material type), never appear
  in Debug, the store, the journal, manifests, task text, or events.
  Rotation scrubs the previous value at replacement time; the caller must
  hand `register` the only owning `Vec` of the value.

## Wiring

`WorkerTransports::with_credential_broker` attaches the operator's broker.
The native arm of `run_started_dispatch` resolves a lease for the profile's
`credential` reference — **with no broker configured, a native run refuses
with the typed `NoCredentialBroker` precondition** (a profile declares its
credential; proceeding without it would be a silent downgrade). External
harness runs receive no native leases: harness credentials remain
harness-owned and are never harvested into the native broker (the #217
boundary). Refusals surface as `credential lease refused: <reason>` /
`no credential broker configured` protocol errors — payload-free.

## Honest non-claims (pending #217 scope)

- The broker is **not durable**: values live in process memory only;
  restart requires operator re-registration, and the failure mode is the
  typed refusal, never a stale value. OS keychain integration and the
  encrypted-at-rest fallback are later slices.
- **Revocation is sticky per reference id**: a revoked reference can never
  be re-registered (rotation uses a NEW reference id). A lease ALREADY
  issued before revocation still materializes until its own expiry —
  active-process invalidation, lease cleanup of materialized files, and
  audit query surfaces are pending.
- **Lease materialization is not yet consumed by a loop transport**: no
  `materialize` call exists on the daemon path — the lease is issued,
  scope-checked, and dropped unmaterialized. Env/file/stdin/header
  injection according to adapter capability is the next slice. No live
  model turn exists — credential plumbing is exercised with fixture values
  only, and live verification remains gated on explicit user authorization
  for credentials and billing. The daemon-level positive lease path is
  proven at the `WorkerTransports` seam with the real fixture contract
  (the daemon's operator-provisioning surface for brokers does not exist
  yet, so no `Host::call` test drives it).
- Redaction across logs, terminals, artifacts, crash reports and export
  corpora, environment/resource-plan resolution (#176), and the full
  secret-descriptor model (versions, rotation windows, per-role/profile
  scopes, Host binding beyond the lease scope) remain with #217.
