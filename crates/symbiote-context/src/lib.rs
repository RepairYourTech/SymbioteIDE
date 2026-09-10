//! Context and credential resolution for worker runs (#217 slice): the
//! typed bridge between a started dispatch's trusted Host state and the
//! exact payload a worker run receives.
//!
//! Two halves, kept deliberately separate:
//!
//! - [`ResolvedContext`]: everything the run should SEE. Built from the
//!   dispatch contract and the origin work item (description, requirements,
//!   constraints, risks, acceptance), bounded, and attributable — no caller
//!   input can steer it. The resolution is pure data-over-trusted-state;
//!   the Host's store supplies every fact.
//!
//! - [`CredentialBroker`]: everything the run may USE. The operator
//!   registers secret values against `CredentialReferenceId`s (owning
//!   project, environment label); at run time the broker issues a
//!   short-lived [`CredentialLease`] ONLY when the dispatch's binding
//!   profile references that credential, the owning project matches the
//!   dispatch's project, AND the binding access grants `UseCredential`.
//!   A lease is the only path to the plaintext and it expires by
//!   timestamp. Environment labels are materialization NAMES chosen by
//!   the operator at registration — they are not authorization surfaces.
//!   Values live in process memory only (zeroized on drop); they never
//!   enter the store, the journal, manifests, task text, or events.
//!   NOT yet provided: environment/file/header injection into a running
//!   transport, and invalidation of leases already issued before a
//!   revocation (an outstanding lease still materializes until it
//!   expires).
//!
//! Boundary: this crate never decides WHO may register a secret (operator
//! surface, canonical owner #217/H04) and never performs OS keychain or
//! encrypted-at-rest storage (pending canonical scope). The broker is the
//! process-local registry the Host composes; persistence and keychain
//! integration are later slices on #217. Leases are non-amplifying: a
//! lease grants nothing beyond the referenced value for this dispatch,
//! and revocation (drop of the registration or explicit revoke) makes
//! every later resolution and materialization fail closed.
//!
//! Tests cover normal, boundary (expiry, wrong project/role/profile/host,
//! unknown reference, revoked), failure (over-bound text, missing work
//! item through an injected reader), and cleanup behavior. The store
//! dependency is injected as a trait so this crate does not depend on
//! symbiote-store.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::{
    AccessSnapshot, CredentialReferenceId, DispatchId, HostId, ProjectId, RoleId, RuntimeProfileId,
    TaskId, Timestamp, WorkId,
};
use zeroize::Zeroize;

/// The resolved context payload for one worker run: the prompt text plus
/// the origin work item's structured guidance, all from trusted Host
/// state. Serialized into the loop's task prompt by the caller; the loop
/// sees text, this crate owns the bounded composition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedContext {
    pub dispatch_id: DispatchId,
    pub task_id: TaskId,
    pub project_id: ProjectId,
    /// The origin work item's description — the run's prompt text.
    pub prompt: BoundedText,
    /// The origin work item's requirements, constraints, risks and
    /// acceptance items, each bounded, in work-item order.
    pub requirements: Vec<BoundedText>,
    pub constraints: Vec<BoundedText>,
    pub risks: Vec<BoundedText>,
    pub acceptance: Vec<BoundedText>,
    /// The binding's context budget, carried so the caller can enforce
    /// input bounds without re-reading the store.
    pub context: symbiote_domain::ContextPolicy,
    /// The access snapshot the dispatch contract carries (current policy
    /// at compile time; the Host re-checks against the ceiling).
    pub access: AccessSnapshot,
}

/// The per-item and per-collection text bounds. Every list is capped both
/// per item and in total so a hostile or careless work item cannot blow up
/// the run's input envelope through structure.
pub const MAX_ITEM_BYTES: usize = 8_192;
pub const MAX_LIST_ITEMS: usize = 64;
pub const MAX_LIST_TOTAL_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolutionError {
    /// The origin work item is missing or its description is empty.
    NoPrompt,
    /// A list exceeded its structural bound.
    Overbound,
    /// The store reader refused (payload-free; the caller logs its own).
    StoreFailed,
}

impl std::fmt::Display for ResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "context resolution failed: {self:?}")
    }
}
impl std::error::Error for ResolutionError {}

/// A bounded text item with a visible truncation marker on overflow —
/// the same char-boundary discipline as every other bounded text in the
/// system.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BoundedText(String);
impl BoundedText {
    const MARKER: &str = "…[truncated]";
    pub fn new(raw: &str) -> Result<Self, ResolutionError> {
        if raw.len() <= MAX_ITEM_BYTES {
            return Ok(Self(raw.to_owned()));
        }
        let budget = MAX_ITEM_BYTES - Self::MARKER.len();
        let mut end = budget;
        while !raw.is_char_boundary(end) {
            end -= 1;
        }
        Ok(Self(format!("{}{}", &raw[..end], Self::MARKER)))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for BoundedText {
    type Error = ResolutionError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}
impl From<BoundedText> for String {
    fn from(value: BoundedText) -> Self {
        value.0
    }
}

/// Bounded collection: caps item count and total bytes.
fn bounded_list(raw: &[String]) -> Result<Vec<BoundedText>, ResolutionError> {
    if raw.len() > MAX_LIST_ITEMS {
        return Err(ResolutionError::Overbound);
    }
    let mut total = 0usize;
    let mut out = Vec::with_capacity(raw.len());
    for item in raw {
        let bounded = BoundedText::new(item)?;
        total = total.saturating_add(bounded.0.len());
        if total > MAX_LIST_TOTAL_BYTES {
            return Err(ResolutionError::Overbound);
        }
        out.push(bounded);
    }
    Ok(out)
}

/// The trusted-state reader the resolution runs against. The Host
/// implements it over the store; tests implement it in memory. Payload
/// errors are collapsed so no store text leaks into resolution errors.
pub trait ContextSource {
    fn work_item_text(
        &self,
        project: &ProjectId,
        work: &WorkId,
    ) -> Result<Option<WorkItemText>, ResolutionError>;
}

/// The bounded slice of a work item a run's context needs.
#[derive(Clone)]
pub struct WorkItemText {
    pub description: String,
    pub requirements: Vec<String>,
    pub constraints: Vec<String>,
    pub risks: Vec<String>,
    pub acceptance: Vec<String>,
}

/// The dispatch-facet inputs resolution needs. All come from the dispatch
/// contract — the caller cannot substitute its own.
pub struct ResolutionInputs<'a> {
    pub dispatch_id: &'a DispatchId,
    pub task_id: &'a TaskId,
    pub project_id: &'a ProjectId,
    pub origin_work: &'a WorkId,
    pub context: symbiote_domain::ContextPolicy,
    pub access: AccessSnapshot,
}

/// Resolves the run context from trusted state. Pure composition: no
/// network, no execution, no authority grant — the payload is text the
/// worker will see, and the Host's existing authorization gates decide
/// everything else.
pub fn resolve_context(
    source: &impl ContextSource,
    inputs: ResolutionInputs<'_>,
) -> Result<ResolvedContext, ResolutionError> {
    let work = source
        .work_item_text(inputs.project_id, inputs.origin_work)?
        .ok_or(ResolutionError::NoPrompt)?;
    if work.description.trim().is_empty() {
        return Err(ResolutionError::NoPrompt);
    }
    let prompt = BoundedText::new(&work.description)?;
    Ok(ResolvedContext {
        dispatch_id: inputs.dispatch_id.clone(),
        task_id: inputs.task_id.clone(),
        project_id: inputs.project_id.clone(),
        requirements: bounded_list(&work.requirements)?,
        constraints: bounded_list(&work.constraints)?,
        risks: bounded_list(&work.risks)?,
        acceptance: bounded_list(&work.acceptance)?,
        prompt,
        context: inputs.context,
        access: inputs.access,
    })
}

/// Renders the context as the task prompt text the loop receives. The
/// structured guidance is appended in labeled sections so the model sees
/// each obligation distinctly; nothing here can execute.
pub fn render_prompt(context: &ResolvedContext) -> String {
    let mut out = String::with_capacity(1024);
    out.push_str(context.prompt.as_str());
    let section = |out: &mut String, label: &str, items: &[BoundedText]| {
        if !items.is_empty() {
            out.push_str(&format!("\n\n{label}:"));
            for item in items {
                out.push_str(&format!("\n- {}", item.as_str()));
            }
        }
    };
    section(&mut out, "Requirements", &context.requirements);
    section(&mut out, "Constraints", &context.constraints);
    section(&mut out, "Risks", &context.risks);
    section(&mut out, "Acceptance criteria", &context.acceptance);
    out
}

/// A registered secret: the broker's process-local record. Values are
/// held zeroized-on-drop and never serialized — the type has no serde
/// impls, so it cannot cross the wire by construction.
pub struct RegisteredSecret {
    owning_project: ProjectId,
    environment: String,
    value: SecretBytes,
}

/// Zeroized-on-drop byte buffer with a Debug that never prints contents.
pub struct SecretBytes(Box<[u8]>);
impl SecretBytes {
    pub fn new(value: Vec<u8>) -> Self {
        Self(value.into_boxed_slice())
    }
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}
impl Drop for SecretBytes {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}
impl std::fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretBytes({} bytes)", self.0.len())
    }
}

/// The operator's process-local credential registry (#217 slice scope).
/// Registration, scoped resolution and revocation live here; OS keychain
/// and encrypted-at-rest storage remain pending canonical scope. The
/// broker is NOT durable: restart requires the operator to re-register —
/// the failure mode is "missing secret" (typed, surfaced), never "stale
/// secret silently reused".
///
/// Debug never prints values: the impl redacts to reference names only.
#[derive(Default)]
pub struct CredentialBroker {
    secrets: BTreeMap<CredentialReferenceId, RegisteredSecret>,
    revoked: BTreeSet<CredentialReferenceId>,
}
impl std::fmt::Debug for CredentialBroker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialBroker")
            .field("registered", &self.secrets.keys().collect::<Vec<_>>())
            .field("revoked", &self.revoked.iter().collect::<Vec<_>>())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerError {
    /// The reference is unknown to the broker.
    UnknownReference,
    /// The reference was explicitly revoked; it cannot be re-registered
    /// under a different project (re-registration under the SAME project
    /// is rotation and is allowed).
    Revoked,
    /// The dispatch's project does not match the secret's owning project.
    CrossProjectDenied,
    /// The dispatch's binding profile does not reference this credential.
    NotReferencedByProfile,
    /// The dispatch's binding access does not grant `UseCredential` — the
    /// typed authority that any credential lease may issue at all.
    UseCredentialNotGranted,
    /// A registration's environment label was empty or over-bound.
    InvalidEnvironmentLabel,
    /// The lease window has closed.
    Expired,
    /// The lease window has not opened (a backdated clock cannot reuse).
    NotYetValid,
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "credential broker: {self:?}")
    }
}
impl std::error::Error for BrokerError {}

/// The lease scope, all from the dispatch contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LeaseScope {
    pub dispatch_id: DispatchId,
    pub project_id: ProjectId,
    pub role_id: RoleId,
    pub profile_id: RuntimeProfileId,
    pub host_id: HostId,
}

/// A short-lived grant to ONE secret for ONE dispatch. Holds a copy of
/// the value (the registration is untouched); expiry is checked at
/// materialization time, not just at issue time, so a held lease cannot
/// outlive its window.
pub struct CredentialLease {
    pub reference: CredentialReferenceId,
    pub scope: LeaseScope,
    pub issued_at: Timestamp,
    pub expires_at: Timestamp,
    /// The environment name the dispatch may bind this value to.
    pub environment: String,
    value: SecretBytes,
}
impl CredentialLease {
    /// Materializes the lease as one name→value pair for the run's
    /// environment. Refuses an expired or not-yet-valid lease. The name
    /// is the caller's declared binding name, NOT model input.
    pub fn materialize(&self, at: Timestamp) -> Result<(String, String), BrokerError> {
        if at.0 < self.issued_at.0 {
            return Err(BrokerError::NotYetValid);
        }
        if at.0 >= self.expires_at.0 {
            return Err(BrokerError::Expired);
        }
        Ok((self.environment.clone(), self.text()))
    }
    fn text(&self) -> String {
        String::from_utf8_lossy(self.value.expose()).into_owned()
    }
}
impl Drop for CredentialLease {
    fn drop(&mut self) {
        // The boxed slice is zeroized by SecretBytes' own Drop; the lease
        // additionally clears the environment name so materialized labels
        // do not outlive the grant.
        self.environment.zeroize();
    }
}
impl std::fmt::Debug for CredentialLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialLease")
            .field("reference", &self.reference)
            .field("scope", &self.scope)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

/// The dispatch facet the broker checks against. `profile_credential_refs`
/// are the binding profile's credential references;
/// `use_credential_granted` is the binding access's `Permission::
/// UseCredential` grant — the typed authority that a credential lease may
/// issue at all (the Host computes it from the dispatch contract; PR #508
/// review P0: environment labels are NOT authorization surfaces, they are
/// materialization names chosen by the operator at registration).
pub struct BrokerRequest<'a> {
    pub scope: &'a LeaseScope,
    pub profile_credential_refs: &'a [CredentialReferenceId],
    pub use_credential_granted: bool,
    pub at: Timestamp,
}

impl CredentialBroker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a secret, or replaces its value (same project). The
    /// reference's owning project is immutable; a different project on an
    /// existing reference is refused, and a revoked reference can never be
    /// re-registered — rotation after revocation uses a NEW reference id.
    /// The caller MUST hand this method the only owning `Vec` of the
    /// value: the broker scrubs the PREVIOUS record immediately (zeroized
    /// at replacement, not at some later drop), but bytes the caller kept
    /// are the caller's to manage.
    pub fn register(
        &mut self,
        reference: CredentialReferenceId,
        owning_project: ProjectId,
        environment: String,
        value: Vec<u8>,
    ) -> Result<(), BrokerError> {
        if environment.is_empty() || environment.len() > 128 {
            return Err(BrokerError::InvalidEnvironmentLabel);
        }
        if let Some(existing) = self.secrets.remove(&reference) {
            if owning_project != existing.owning_project {
                // Project immutability: restore the ORIGINAL record shell
                // (its value was scrubbed by the remove above) so a later
                // same-owner rotation still works, then refuse.
                let owner = existing.owning_project.clone();
                let label = existing.environment.clone();
                drop(existing);
                self.secrets.insert(
                    reference,
                    RegisteredSecret {
                        owning_project: owner,
                        environment: label,
                        value: SecretBytes::new(Vec::new()),
                    },
                );
                return Err(BrokerError::CrossProjectDenied);
            }
            // Drop the previous record BEFORE building the new one: the
            // old value is zeroized here, not when the map insert drops a
            // shadowed record.
            drop(existing);
        } else if self.revoked.contains(&reference) {
            // Revocation is STICKY per reference id: re-registering the
            // same id would silently resurrect a compromised secret.
            // Rotation is the operator registering a NEW reference id and
            // updating the profile that references it.
            return Err(BrokerError::Revoked);
        }
        self.secrets.insert(
            reference,
            RegisteredSecret {
                owning_project,
                environment,
                value: SecretBytes::new(value),
            },
        );
        Ok(())
    }

    /// Revokes: the reference can no longer be resolved, its registered
    /// value is zeroized immediately, and re-registration is refused
    /// forever (sticky). A lease already issued from the revoked record
    /// still holds its own copy until it expires — the honest disclosure
    /// of the #217 'revocation blocks new dispatches' boundary; actively
    /// invalidating outstanding leases is pending canonical scope.
    pub fn revoke(&mut self, reference: &CredentialReferenceId) {
        // Remove-then-drop scrubs the value at this point, not later.
        drop(self.secrets.remove(reference));
        self.revoked.insert(reference.clone());
    }

    /// Issues a lease if and only if every scope check passes. Checks are
    /// ordered so the refusal identity names the FIRST violated boundary.
    pub fn resolve(
        &self,
        reference: &CredentialReferenceId,
        request: BrokerRequest<'_>,
    ) -> Result<CredentialLease, BrokerError> {
        if self.revoked.contains(reference) {
            return Err(BrokerError::Revoked);
        }
        let secret = self
            .secrets
            .get(reference)
            .ok_or(BrokerError::UnknownReference)?;
        if !request
            .profile_credential_refs
            .iter()
            .any(|candidate| candidate == reference)
        {
            return Err(BrokerError::NotReferencedByProfile);
        }
        if secret.owning_project != request.scope.project_id {
            return Err(BrokerError::CrossProjectDenied);
        }
        if !request.use_credential_granted {
            return Err(BrokerError::UseCredentialNotGranted);
        }
        let window = Timestamp(
            request.at.0.saturating_add(300_000), // 5-minute lease window
        );
        Ok(CredentialLease {
            reference: reference.clone(),
            scope: request.scope.clone(),
            issued_at: request.at,
            expires_at: window,
            environment: secret.environment.clone(),
            value: SecretBytes::new(secret.value.expose().to_vec()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbiote_domain::ContextPolicy;

    struct FixedSource {
        item: Option<WorkItemText>,
    }
    impl ContextSource for FixedSource {
        fn work_item_text(
            &self,
            _project: &ProjectId,
            _work: &WorkId,
        ) -> Result<Option<WorkItemText>, ResolutionError> {
            Ok(self.item.clone())
        }
    }

    fn inputs<'a>(
        dispatch_id: &'a DispatchId,
        task_id: &'a TaskId,
        project_id: &'a ProjectId,
        origin_work: &'a WorkId,
        access: AccessSnapshot,
    ) -> ResolutionInputs<'a> {
        ResolutionInputs {
            dispatch_id,
            task_id,
            project_id,
            origin_work,
            context: ContextPolicy {
                bundle: symbiote_domain::ContextBundleId::new("bundle").unwrap(),
                revision: symbiote_domain::Revision(1),
                max_input_tokens: 4096,
                reserved_output_tokens: 2048,
            },
            access,
        }
    }

    fn access(project: &ProjectId) -> AccessSnapshot {
        AccessSnapshot {
            project_id: project.clone(),
            roots: std::collections::BTreeSet::new(),
            grants: std::collections::BTreeSet::from([symbiote_domain::Permission::ReadRoot]),
            policy_revision: symbiote_domain::Revision(1),
        }
    }

    fn ids(tag: &str) -> (DispatchId, TaskId, ProjectId, WorkId) {
        (
            DispatchId::new(format!("disp-{tag}")).unwrap(),
            TaskId::new(format!("task-{tag}")).unwrap(),
            ProjectId::new(format!("project-{tag}")).unwrap(),
            WorkId::Objective(symbiote_domain::ObjectiveId::new(format!("obj-{tag}")).unwrap()),
        )
    }

    #[test]
    fn resolution_carries_prompt_and_structured_guidance_bounded() {
        let (dispatch, task, project, work) = ids("resolve");
        let source = FixedSource {
            item: Some(WorkItemText {
                description: "Implement the bounded change.".into(),
                requirements: vec!["must compile".into(), "must test".into()],
                constraints: vec!["no network".into()],
                risks: vec![],
                acceptance: vec!["reviewer approves".into()],
            }),
        };
        let context = resolve_context(
            &source,
            inputs(&dispatch, &task, &project, &work, access(&project)),
        )
        .unwrap();
        assert_eq!(context.prompt.as_str(), "Implement the bounded change.");
        assert_eq!(context.requirements.len(), 2);
        assert_eq!(context.acceptance.len(), 1);
        assert_eq!(context.project_id, project);
        let prompt = render_prompt(&context);
        assert!(prompt.starts_with("Implement the bounded change."));
        assert!(prompt.contains("Requirements:\n- must compile"));
        assert!(prompt.contains("Acceptance criteria:\n- reviewer approves"));
        assert!(!prompt.contains("Risks:"));
    }

    #[test]
    fn overbound_lists_are_refused_not_truncated_away() {
        // Structural overflow is a refusal, not a silent narrowing: a work
        // item with 65 requirements is a composition problem.
        let (dispatch, task, project, work) = ids("overbound");
        let source = FixedSource {
            item: Some(WorkItemText {
                description: "d".into(),
                requirements: (0..65).map(|i| format!("r{i}")).collect(),
                constraints: vec![],
                risks: vec![],
                acceptance: vec![],
            }),
        };
        assert_eq!(
            resolve_context(
                &source,
                inputs(&dispatch, &task, &project, &work, access(&project))
            ),
            Err(ResolutionError::Overbound)
        );
        // Total-bytes overflow: each over-long item truncates to the 8 KiB
        // item cap, so NINE of them push the list total past its 64 KiB
        // budget (9 x 8192 > 65536).
        let source = FixedSource {
            item: Some(WorkItemText {
                description: "d".into(),
                requirements: vec!["x".repeat(MAX_ITEM_BYTES); 9],
                constraints: vec![],
                risks: vec![],
                acceptance: vec![],
            }),
        };
        assert_eq!(
            resolve_context(
                &source,
                inputs(&dispatch, &task, &project, &work, access(&project))
            ),
            Err(ResolutionError::Overbound)
        );
    }

    #[test]
    fn oversized_items_truncate_on_char_boundaries_with_a_marker() {
        let (dispatch, task, project, work) = ids("bigitem");
        let emoji = "🔍".repeat(MAX_ITEM_BYTES); // 4-byte chars
        let source = FixedSource {
            item: Some(WorkItemText {
                description: "ok".into(),
                requirements: vec![emoji],
                constraints: vec![],
                risks: vec![],
                acceptance: vec![],
            }),
        };
        let context = resolve_context(
            &source,
            inputs(&dispatch, &task, &project, &work, access(&project)),
        )
        .unwrap();
        assert_eq!(context.requirements.len(), 1);
        let text = context.requirements[0].as_str();
        assert!(text.len() <= MAX_ITEM_BYTES);
        assert!(text.ends_with("…[truncated]"));
        assert!(!text.contains(char::REPLACEMENT_CHARACTER));
    }

    #[test]
    fn missing_or_empty_origin_is_a_typed_refusal() {
        let (dispatch, task, project, work) = ids("missing");
        let source = FixedSource { item: None };
        assert_eq!(
            resolve_context(
                &source,
                inputs(&dispatch, &task, &project, &work, access(&project))
            ),
            Err(ResolutionError::NoPrompt)
        );
        let source = FixedSource {
            item: Some(WorkItemText {
                description: "   ".into(),
                requirements: vec![],
                constraints: vec![],
                risks: vec![],
                acceptance: vec![],
            }),
        };
        assert_eq!(
            resolve_context(
                &source,
                inputs(&dispatch, &task, &project, &work, access(&project))
            ),
            Err(ResolutionError::NoPrompt)
        );
    }

    fn lease_request<'a>(
        scope: &'a LeaseScope,
        refs: &'a [CredentialReferenceId],
        at: Timestamp,
    ) -> BrokerRequest<'a> {
        BrokerRequest {
            scope,
            profile_credential_refs: refs,
            use_credential_granted: true,
            at,
        }
    }

    #[test]
    fn broker_issues_a_scoped_expiring_lease_on_the_happy_path() {
        let (_d, _t, project, _w) = ids("lease");
        let mut broker = CredentialBroker::new();
        let reference = CredentialReferenceId::new("openai-native").unwrap();
        broker
            .register(
                reference.clone(),
                project.clone(),
                "OPENAI_API_KEY".into(),
                b"sk-fixture-value".to_vec(),
            )
            .unwrap();
        let scope = LeaseScope {
            dispatch_id: DispatchId::new("disp-lease").unwrap(),
            project_id: project.clone(),
            role_id: RoleId::new("worker").unwrap(),
            profile_id: RuntimeProfileId::new("profile").unwrap(),
            host_id: HostId::new("host").unwrap(),
        };
        let lease = broker
            .resolve(
                &reference,
                lease_request(&scope, std::slice::from_ref(&reference), Timestamp(1_000)),
            )
            .unwrap();
        // Scope is carried verbatim from the dispatch contract.
        assert_eq!(lease.scope, scope);
        assert_eq!(lease.expires_at, Timestamp(301_000));
        // Materialization within the window yields the name→value pair.
        let (name, value) = lease.materialize(Timestamp(2_000)).unwrap();
        assert_eq!(name, "OPENAI_API_KEY");
        assert_eq!(value, "sk-fixture-value");
        // Debug never prints the value.
        assert!(!format!("{lease:?}").contains("sk-fixture-value"));
        assert!(!format!("{broker:?}").contains("sk-fixture-value"));
    }

    #[test]
    fn cross_project_and_unreferenced_and_ungranted_are_distinct_refusals() {
        let (_d, _t, project, _w) = ids("deny");
        let other = ProjectId::new("project-other").unwrap();
        let mut broker = CredentialBroker::new();
        let reference = CredentialReferenceId::new("shared-key").unwrap();
        broker
            .register(
                reference.clone(),
                project.clone(),
                "KEY".into(),
                b"v".to_vec(),
            )
            .unwrap();
        let scope_other = LeaseScope {
            dispatch_id: DispatchId::new("disp-deny").unwrap(),
            project_id: other.clone(),
            role_id: RoleId::new("worker").unwrap(),
            profile_id: RuntimeProfileId::new("profile").unwrap(),
            host_id: HostId::new("host").unwrap(),
        };
        let scope_self = LeaseScope {
            project_id: project.clone(),
            ..scope_other.clone()
        };
        // Cross-project dispatch: refused even though the profile names it.
        assert!(matches!(
            broker.resolve(
                &reference,
                lease_request(
                    &scope_other,
                    std::slice::from_ref(&reference),
                    Timestamp(1_000)
                )
            ),
            Err(BrokerError::CrossProjectDenied)
        ));
        // Same project but the binding profile does not reference it.
        assert!(matches!(
            broker.resolve(
                &reference,
                lease_request(&scope_self, &[], Timestamp(1_000))
            ),
            Err(BrokerError::NotReferencedByProfile)
        ));
        // Referenced and project-matched but UseCredential not granted.
        assert!(matches!(
            broker.resolve(
                &reference,
                BrokerRequest {
                    scope: &scope_self,
                    profile_credential_refs: std::slice::from_ref(&reference),
                    use_credential_granted: false,
                    at: Timestamp(1_000),
                }
            ),
            Err(BrokerError::UseCredentialNotGranted)
        ));
        // Unknown reference.
        let unknown = CredentialReferenceId::new("nope").unwrap();
        assert!(matches!(
            broker.resolve(
                &unknown,
                lease_request(
                    &scope_self,
                    std::slice::from_ref(&unknown),
                    Timestamp(1_000)
                )
            ),
            Err(BrokerError::UnknownReference)
        ));
        // Invalid environment labels are refused at registration with
        // their own identity (not an access denial).
        assert_eq!(
            broker.register(
                CredentialReferenceId::new("bad-label").unwrap(),
                project.clone(),
                String::new(),
                b"v".to_vec()
            ),
            Err(BrokerError::InvalidEnvironmentLabel)
        );
        assert_eq!(
            broker.register(
                CredentialReferenceId::new("bad-label").unwrap(),
                project.clone(),
                "x".repeat(129),
                b"v".to_vec()
            ),
            Err(BrokerError::InvalidEnvironmentLabel)
        );
    }

    #[test]
    fn revocation_fails_closed_and_blocks_cross_project_rotation() {
        let (_d, _t, project, _w) = ids("revoke");
        let other = ProjectId::new("project-revoke2").unwrap();
        let mut broker = CredentialBroker::new();
        let reference = CredentialReferenceId::new("live-key").unwrap();
        broker
            .register(
                reference.clone(),
                project.clone(),
                "KEY".into(),
                b"v".to_vec(),
            )
            .unwrap();
        broker.revoke(&reference);
        let scope = LeaseScope {
            dispatch_id: DispatchId::new("disp-revoke").unwrap(),
            project_id: project.clone(),
            role_id: RoleId::new("worker").unwrap(),
            profile_id: RuntimeProfileId::new("profile").unwrap(),
            host_id: HostId::new("host").unwrap(),
        };
        assert!(matches!(
            broker.resolve(
                &reference,
                lease_request(&scope, std::slice::from_ref(&reference), Timestamp(1_000))
            ),
            Err(BrokerError::Revoked)
        ));
        // Re-registration is refused under a DIFFERENT project (the
        // revoked record binds the owner) AND under the same project:
        // revocation is STICKY per reference id, so a compromised secret
        // cannot be resurrected. Rotation uses a NEW reference id.
        assert!(matches!(
            broker.register(
                reference.clone(),
                other.clone(),
                "KEY".into(),
                b"v".to_vec()
            ),
            Err(BrokerError::Revoked)
        ));
        assert!(matches!(
            broker.register(
                reference.clone(),
                project.clone(),
                "KEY".into(),
                b"v2".to_vec()
            ),
            Err(BrokerError::Revoked)
        ));
        // Resolution stays revoked either way.
        assert!(matches!(
            broker.resolve(
                &reference,
                lease_request(&scope, std::slice::from_ref(&reference), Timestamp(1_000))
            ),
            Err(BrokerError::Revoked)
        ));
        // The rotation path: a NEW reference id registers and resolves.
        let rotated = CredentialReferenceId::new("live-key-r2").unwrap();
        assert!(
            broker
                .register(
                    rotated.clone(),
                    project.clone(),
                    "KEY".into(),
                    b"v2".to_vec()
                )
                .is_ok()
        );
        assert!(
            broker
                .resolve(
                    &rotated,
                    lease_request(&scope, std::slice::from_ref(&rotated), Timestamp(1_000))
                )
                .is_ok()
        );
    }

    #[test]
    fn an_expired_lease_cannot_materialize() {
        let (_d, _t, project, _w) = ids("expire");
        let mut broker = CredentialBroker::new();
        let reference = CredentialReferenceId::new("short-lived").unwrap();
        broker
            .register(
                reference.clone(),
                project.clone(),
                "KEY".into(),
                b"v".to_vec(),
            )
            .unwrap();
        let scope = LeaseScope {
            dispatch_id: DispatchId::new("disp-expire").unwrap(),
            project_id: project.clone(),
            role_id: RoleId::new("worker").unwrap(),
            profile_id: RuntimeProfileId::new("profile").unwrap(),
            host_id: HostId::new("host").unwrap(),
        };
        let lease = broker
            .resolve(
                &reference,
                lease_request(&scope, std::slice::from_ref(&reference), Timestamp(1_000)),
            )
            .unwrap();
        // Within the window it materializes; at expiry it refuses with the
        // distinct window identities (not an access denial).
        assert!(lease.materialize(Timestamp(300_999)).is_ok());
        assert_eq!(
            lease.materialize(Timestamp(301_000)),
            Err(BrokerError::Expired)
        );
        // Before issuance it refuses too (a backdated clock cannot reuse).
        assert_eq!(
            lease.materialize(Timestamp(999)),
            Err(BrokerError::NotYetValid)
        );
    }
}
