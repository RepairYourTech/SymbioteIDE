//! Workforce Runtime Contract projection surfaces and the runtime handshake (#184).
//!
//! The domain compiles a [`WorkforceRuntimeContract`]: the canonical facts a
//! dispatch runs under. A runtime never reads that contract. An adapter has to
//! place each fact somewhere the runtime actually reads it — a protocol file, a
//! role brief, a task brief, a sandbox the Host enforces, an MCP registration, a
//! profile home, an exit signal — and harnesses disagree about all of those
//! places. This module names those places as [`ContractSurface`] values without
//! assuming any harness's vocabulary, records what the contract demands of each
//! one against what the runtime's own descriptor declares, and reconciles what
//! the runtime reports it took.
//!
//! [#187](https://github.com/RepairYourTech/SymbioteIDE/issues/187) owns
//! projecting canonical configuration into a runtime's config home. This module
//! owns the surface a contract is *carried* on and the outcome of carrying it:
//! which of a runtime's own declared carriers realizes each canonical fact, and
//! what a surface that has no carrier reports instead.
//!
//! ## What a projection is, and what it is not
//!
//! A projection is a report, not a gate. Launch eligibility stays with
//! [`qualify_dispatch`](crate::qualify_dispatch) and the Host's own transaction,
//! which own freshness, evidence and the enforcement minimum. Projection is
//! total: every surface in [`ContractSurface::ALL`] appears in
//! [`ContractProjection::surfaces`], so a fact that no runtime carries is
//! *named* as missing rather than omitted. It is infallible and holds no clock,
//! because it decides nothing — it states, per surface, what the contract
//! demands and what the descriptor holds, using exactly the vocabulary the
//! enforcement claims already use ([`EnforcementStrength`] and
//! [`realizes`](crate::realizes)).
//!
//! A projection carries *references* to the canonical facts — the Workforce
//! Protocol, the Role Operating Contract, the Task Contract, the role, binding,
//! task, root and stream identities — and never their content. An adapter that
//! restated the protocol or the task brief inside a projection would be a second
//! copy of the contract, and the projection would then be a second place the
//! contract can be wrong.
//!
//! ## Desired, projected, observed
//!
//! The three states stay apart. The *demand* is the contract's own: a surface
//! demands the controls the contract claims over it and the capabilities its
//! binding names. The *projection* is what the runtime's descriptor declares for
//! those carriers, at the strength it declares. The *handshake* is what the
//! runtime reports back after launch, and
//! [`ContractProjection::reconcile`] compares the last two in one direction
//! only:
//!
//! - A runtime cannot report a strength above what the projection delivered
//!   (`AdapterError::ContractMismatch`). Reporting cannot mint enforcement the
//!   Host never established, and an incomparable strength — observed where
//!   emulated was delivered — is a mismatch rather than a degradation.
//! - A runtime cannot report a carrier the projection found missing, unsupported
//!   or unknown, and cannot report a carrier on a surface that does not demand
//!   it. A report widens nothing.
//! - A report *below* the projection is recorded as [`Agreement::Weaker`] with
//!   both strengths retained, and a surface the runtime said nothing about is
//!   [`Agreement::Unreported`]. Silence is not agreement, and a degradation is
//!   never smoothed into a delivery: routing, permissions, UI and certification
//!   read [`HandshakeOutcome::is_exact`] and the per-carrier states to see what
//!   the runtime actually took.
//!
//! Neither half of this module starts, stops or completes anything. No type here
//! can mark canonical work complete: a worker's completion is a request event
//! ([`crate::events`]), the verification surface carries the *signal* under the
//! contract's own `CompletionAuthority` claim, and the canonical transition
//! stays with the domain.

use crate::{AdapterError, Capability, MECHANISM_MAX_BYTES, RuntimeDescriptor, Support, realizes};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;

/// A place a Workforce Runtime Contract fact must be carried for the runtime to
/// act under it. The names describe the *place*, not a harness: an adapter maps
/// its own protocol file, brief, sandbox or registration onto these, and a
/// runtime that has no place for one has to say so rather than appear to have it.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ContractSurface {
    /// The stable Workforce Protocol the dispatch was compiled under.
    WorkforceProtocol,
    /// The routed Role's operating contract.
    RoleOperatingContract,
    /// The Task Contract and the context handed to the worker with it.
    TaskContract,
    /// The skills the binding requires the worker to hold.
    Skills,
    /// The tools and MCP registrations the binding requires, and nothing else.
    ToolsAndMcp,
    /// Native permissions and sandboxing: the filesystem, process and network
    /// confinement the contract claims over this execution.
    PermissionsAndSandbox,
    /// Hooks and event delivery the runtime can be driven through.
    HooksAndEvents,
    /// Plugins and extensions the runtime loads.
    PluginsAndExtensions,
    /// The environment the worker runs in: its own profile's home and
    /// configuration rather than the Host's ambient ones.
    Environment,
    /// References to secrets the execution is authorized to use. The projection
    /// names the references; no secret value enters it.
    SecretReferences,
    /// Verification and exit signals the worker produces. Carries signals only:
    /// the contract's `CompletionAuthority` claim is what keeps a worker's own
    /// report from being completion.
    VerificationSignals,
    /// Cleanup and teardown after the execution, under the contract's
    /// `Cancellation` claim.
    Cleanup,
}

impl ContractSurface {
    /// Every surface, in the order this module states them. A projection covers
    /// all of them; the list is the completeness the census of a projection is
    /// checked against.
    pub const ALL: [ContractSurface; 12] = [
        ContractSurface::WorkforceProtocol,
        ContractSurface::RoleOperatingContract,
        ContractSurface::TaskContract,
        ContractSurface::Skills,
        ContractSurface::ToolsAndMcp,
        ContractSurface::PermissionsAndSandbox,
        ContractSurface::HooksAndEvents,
        ContractSurface::PluginsAndExtensions,
        ContractSurface::Environment,
        ContractSurface::SecretReferences,
        ContractSurface::VerificationSignals,
        ContractSurface::Cleanup,
    ];

    /// The controls whose claims this surface carries. The table assigns each
    /// control to exactly one surface, so a control can be reported missing
    /// where it matters instead of being delivered on a surface nobody checks.
    pub fn controls(self) -> &'static [Control] {
        match self {
            ContractSurface::PermissionsAndSandbox => {
                &[Control::Filesystem, Control::Process, Control::Network]
            }
            ContractSurface::SecretReferences => &[Control::Credentials],
            ContractSurface::VerificationSignals => &[Control::CompletionAuthority],
            ContractSurface::Cleanup => &[Control::Cancellation],
            _ => &[],
        }
    }

    /// The capabilities a runtime declares when it carries this surface itself.
    /// A capability here is a declaration of a place, not of enforcement: the
    /// strength of a control is claimed only on the preventive surfaces.
    pub fn capabilities(self) -> &'static [Capability] {
        match self {
            ContractSurface::WorkforceProtocol
            | ContractSurface::RoleOperatingContract
            | ContractSurface::TaskContract => &[Capability::Instructions],
            ContractSurface::Skills => &[Capability::Skills],
            ContractSurface::ToolsAndMcp => &[Capability::Tools],
            ContractSurface::HooksAndEvents => &[Capability::Hooks],
            ContractSurface::PluginsAndExtensions => &[Capability::Extensions],
            ContractSurface::Environment => &[Capability::ProfileIsolation],
            _ => &[],
        }
    }
}

/// What the descriptor holds for one control the contract claims over a surface.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreventiveDelivery {
    /// The runtime's own declared control realizes the contract's claim.
    Delivered {
        strength: EnforcementStrength,
        mechanism: String,
    },
    /// The runtime carries the control, but weaker than the contract claims —
    /// an observed or emulated mechanism where the contract claims prevention.
    /// Both strengths are retained: the degradation is a fact routing,
    /// permissions, UI and certification read, not a delivery to assume.
    Degraded {
        claimed: EnforcementStrength,
        declared: EnforcementStrength,
        mechanism: String,
    },
    /// The descriptor does not list this control at all. Nothing carries it.
    Missing {},
}

impl PreventiveDelivery {
    /// The strength actually realized on the runtime, or `None` when nothing
    /// carries the control. A degraded control is realized at its weaker
    /// strength rather than at the contract's claim.
    pub fn realized(&self) -> Option<EnforcementStrength> {
        match self {
            PreventiveDelivery::Delivered { strength, .. } => Some(*strength),
            PreventiveDelivery::Degraded { declared, .. } => Some(*declared),
            PreventiveDelivery::Missing {} => None,
        }
    }
}

/// What the descriptor holds for one capability the contract demands on a
/// surface. The three refusals stay distinct: a runtime that declares a
/// capability unsupported, one that has not established it, and one that never
/// mentioned it are three different facts, and none of them is support.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CarrierDelivery {
    /// The descriptor declares the capability supported.
    Declared {},
    /// The descriptor declares it unsupported.
    Unsupported {},
    /// The descriptor has not established it.
    Unknown {},
    /// The descriptor does not mention it.
    Absent {},
}

/// One surface: the carriers this contract demands on it, with what the runtime
/// declares for each. A surface lists what the contract demands and never the
/// runtime's whole advertised feature set, so an undeclared feature stays a
/// descriptor fact rather than becoming an apparent requirement.
///
/// The surface is a field rather than a map key on purpose: a map keyed by the
/// surface would carry the vocabulary in the running SDK and publish a schema
/// whose keys are untyped strings, so a third-party adapter reading the schema
/// could not learn which surfaces exist. As a field, the vocabulary is in the
/// published schema and the entries keep their declared order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurfaceProjection {
    /// The place this projection is about.
    pub surface: ContractSurface,
    /// One entry for every control the contract claims over this surface, and
    /// none for a control it does not claim.
    pub preventive: BTreeMap<Control, PreventiveDelivery>,
    /// One entry for every capability this contract's own facts require here.
    pub carried: BTreeMap<Capability, CarrierDelivery>,
}

/// The canonical facts a projection references, by identity and revision only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContractReferences {
    /// The stable Workforce Protocol the binding pinned, with its revision.
    pub protocol: VersionedProtocol,
    /// The routed Role's operating contract, with its revision.
    pub role_contract: VersionedRoleContract,
    /// The Task Contract handed to the worker, with its revision.
    pub task_contract: VersionedTaskContract,
    /// The Role this dispatch staffed.
    pub role: RoleId,
    /// The binding that authorized it.
    pub binding: BindingId,
    /// The Task it is executing.
    pub task: TaskId,
    /// The Root it is confined to.
    pub root: RootId,
    /// The Change Stream it is allowed to mutate.
    pub stream: ChangeStreamId,
}

/// A Workforce Runtime Contract as a set of surfaces, projected onto the
/// runtime's own declared carriers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContractProjection {
    /// The dispatch this projection is of. A handshake for any other dispatch
    /// is refused rather than reconciled.
    pub dispatch: DispatchId,
    /// The runtime the projection was built against, so a projection names the
    /// exact adapter, installation, profile, model, host, versions and platform
    /// it was read from. Freshness stays qualification's fact: this is the
    /// descriptor's identity, not a claim about when it was observed.
    pub adapter: AgentRuntimeAdapterId,
    /// The installed harness the projection was read from, absent for a native
    /// runtime.
    pub installation: Option<InstallationId>,
    /// The runtime profile — account, configuration home and model identity —
    /// at the revision the projection was read at.
    pub profile: RuntimeProfileId,
    pub profile_revision: Revision,
    /// The selected model.
    pub model: ModelId,
    /// The Host the runtime runs on.
    pub host: HostId,
    /// Native or external: the kind of runtime the surfaces were read from.
    pub runtime: RuntimeKind,
    /// The adapter's own version, and the upstream product version it drives.
    pub adapter_version: String,
    pub upstream_version: String,
    /// The platform the descriptor was observed on.
    pub platform: String,
    /// The canonical facts, by identity and revision rather than by content.
    pub references: ContractReferences,
    /// Every surface in [`ContractSurface::ALL`], in that order, with no entry
    /// omitted and no surface named twice.
    pub surfaces: Vec<SurfaceProjection>,
}

impl ContractProjection {
    /// Read a compiled dispatch onto a runtime's declared carriers. Pure and
    /// infallible: the contract's validity and the runtime's eligibility are
    /// [`qualify_dispatch`](crate::qualify_dispatch)'s facts, and a projection
    /// that gated would be a second owner of them.
    pub fn project(dispatch: &Dispatch, descriptor: &RuntimeDescriptor) -> Self {
        let contract = dispatch.contract();
        let binding = contract.binding();
        let enforcement = contract.enforcement();
        let mut surfaces = Vec::new();
        for surface in ContractSurface::ALL {
            let mut preventive = BTreeMap::new();
            for control in surface.controls() {
                if let Some(claim) = enforcement.get(control) {
                    preventive.insert(
                        control.clone(),
                        match descriptor.controls.get(control) {
                            Some(support) if realizes(claim.strength, support.strength) => {
                                PreventiveDelivery::Delivered {
                                    strength: support.strength,
                                    mechanism: support.mechanism.clone(),
                                }
                            }
                            Some(support) => PreventiveDelivery::Degraded {
                                claimed: claim.strength,
                                declared: support.strength,
                                mechanism: support.mechanism.clone(),
                            },
                            None => PreventiveDelivery::Missing {},
                        },
                    );
                }
            }
            let mut carried = BTreeMap::new();
            for capability in surface.capabilities() {
                if !demanded(capability.clone(), binding) {
                    continue;
                }
                carried.insert(
                    capability.clone(),
                    match descriptor.capabilities.get(capability) {
                        Some(Support::Supported { .. }) => CarrierDelivery::Declared {},
                        Some(Support::Unsupported) => CarrierDelivery::Unsupported {},
                        Some(Support::Unknown) => CarrierDelivery::Unknown {},
                        None => CarrierDelivery::Absent {},
                    },
                );
            }
            surfaces.push(SurfaceProjection {
                surface,
                preventive,
                carried,
            });
        }
        Self {
            dispatch: dispatch.id().clone(),
            adapter: descriptor.adapter_id.clone(),
            installation: descriptor.installation.clone(),
            profile: descriptor.profile_id.clone(),
            profile_revision: descriptor.profile_revision,
            model: descriptor.model_id.clone(),
            host: descriptor.host_id.clone(),
            runtime: descriptor.runtime,
            adapter_version: descriptor.adapter_version.clone(),
            upstream_version: descriptor.upstream_version.clone(),
            platform: descriptor.platform.clone(),
            references: ContractReferences {
                protocol: binding.protocol.clone(),
                role_contract: contract.role().operating_contract.clone(),
                task_contract: contract.task_contract().clone(),
                role: contract.role().id.clone(),
                binding: binding.id.clone(),
                task: contract.task_id().clone(),
                root: contract.root_id().clone(),
                stream: contract.stream_id().clone(),
            },
            surfaces,
        }
    }

    /// One surface's carriers, or `None` for a surface this projection does not
    /// cover — which no projection of [`ContractSurface::ALL`] can be.
    pub fn surface(&self, surface: ContractSurface) -> Option<&SurfaceProjection> {
        self.surfaces.iter().find(|entry| entry.surface == surface)
    }

    /// The surfaces the runtime carries nothing for: a control the descriptor
    /// does not list at all, or a demanded capability it declared unsupported,
    /// unknown or not at all. Empty is the only state in which every demanded
    /// carrier is at least declared — and a declared carrier is a declaration,
    /// not a delivery: the handshake is where the runtime says what it took.
    ///
    /// A control the runtime carries *weakly* is deliberately not withheld: it
    /// is degraded, and `preventive` names both strengths, so a caller reading
    /// this set alone cannot mistake a degradation for either delivery or
    /// nothing at all.
    pub fn withheld(&self) -> BTreeSet<ContractSurface> {
        let mut withheld = BTreeSet::new();
        for entry in &self.surfaces {
            if entry
                .preventive
                .values()
                .any(|delivery| delivery.realized().is_none())
                || entry
                    .carried
                    .values()
                    .any(|delivery| !matches!(delivery, CarrierDelivery::Declared {}))
            {
                withheld.insert(entry.surface);
            }
        }
        withheld
    }

    /// Reconcile what the runtime reported it took with what was projected.
    ///
    /// The report may only narrow. Anything it claims beyond the projection —
    /// a stronger control, a carrier the projection found missing, unsupported
    /// or unknown, a carrier on a surface that does not demand it, a blank or
    /// oversized mechanism — is refused with the typed error rather than
    /// recorded as an observation. What narrows is returned as an outcome that
    /// names itself: [`Agreement::Weaker`] retains both strengths,
    /// [`Agreement::Unreported`] is what silence yields, and
    /// [`HandshakeOutcome::is_exact`] is true only when every demanded carrier
    /// came back at exactly what was projected.
    pub fn reconcile(
        &self,
        session: &Session,
        report: &RuntimeHandshake,
    ) -> Result<HandshakeOutcome, AdapterError> {
        if report.dispatch != self.dispatch {
            return Err(AdapterError::ContractMismatch);
        }
        if session.dispatch_id != self.dispatch || session.kind != self.runtime {
            return Err(AdapterError::SessionMismatch);
        }
        let mut named = BTreeSet::new();
        if report
            .surfaces
            .iter()
            .any(|entry| !named.insert(entry.surface))
        {
            return Err(AdapterError::ContractMismatch);
        }
        let mut surfaces = Vec::new();
        for projection in &self.surfaces {
            let reported = report
                .surfaces
                .iter()
                .find(|entry| entry.surface == projection.surface);
            if let Some(entry) = reported {
                // A report widens nothing: a carrier this surface does not
                // demand cannot appear in one, whatever the runtime says.
                if entry
                    .preventive
                    .keys()
                    .any(|control| !projection.preventive.contains_key(control))
                    || entry
                        .carried
                        .iter()
                        .any(|capability| !projection.carried.contains_key(capability))
                {
                    return Err(AdapterError::ContractMismatch);
                }
                if entry.mechanism.trim().is_empty()
                    || entry.mechanism.len() > MECHANISM_MAX_BYTES
                    || entry.mechanism.chars().any(char::is_control)
                {
                    return Err(AdapterError::InvalidInput);
                }
            }
            let mut preventive = BTreeMap::new();
            for (control, delivered) in &projection.preventive {
                let stated = reported
                    .and_then(|entry| entry.preventive.get(control))
                    .copied();
                let agreement = match (delivered.realized(), stated) {
                    (_, None) => Agreement::Unreported,
                    (None, Some(_)) => return Err(AdapterError::ContractMismatch),
                    (Some(realized), Some(stated)) => {
                        if !realizes(stated, realized) {
                            return Err(AdapterError::ContractMismatch);
                        }
                        if stated == realized {
                            Agreement::Exact
                        } else {
                            Agreement::Weaker
                        }
                    }
                };
                preventive.insert(
                    control.clone(),
                    ControlState {
                        delivered: delivered.clone(),
                        reported: stated,
                        agreement,
                    },
                );
            }
            let mut carried = BTreeMap::new();
            for (capability, delivered) in &projection.carried {
                let stated = reported.is_some_and(|entry| entry.carried.contains(capability));
                if stated && !matches!(delivered, CarrierDelivery::Declared {}) {
                    return Err(AdapterError::ContractMismatch);
                }
                carried.insert(
                    capability.clone(),
                    CarrierState {
                        delivered: *delivered,
                        reported: stated,
                        agreement: if stated {
                            Agreement::Exact
                        } else {
                            Agreement::Unreported
                        },
                    },
                );
            }
            surfaces.push(SurfaceState {
                surface: projection.surface,
                reported_mechanism: reported.map(|entry| entry.mechanism.clone()),
                preventive,
                carried,
            });
        }
        Ok(HandshakeOutcome {
            session: session.id.clone(),
            surfaces,
        })
    }
}

/// Whether a capability on a surface is demanded by the contract's own facts.
/// The reference surfaces, hooks, extensions and the worker's own environment
/// are always demanded; skills and tools are demanded when the binding names
/// them. A capability that no surface demands would be a table entry nothing
/// reads, which the module's cases refuse.
pub fn demanded(capability: Capability, binding: &WorkforceBinding) -> bool {
    match capability {
        Capability::Instructions
        | Capability::Hooks
        | Capability::Extensions
        | Capability::ProfileIsolation => true,
        Capability::Skills => !binding.required_skills.is_empty(),
        Capability::Tools => !binding.required_tools.is_empty(),
        _ => false,
    }
}

/// What the runtime reports it took for one surface, after launch. A runtime
/// reports what it actually did: a carrier the projection did not deliver cannot
/// appear here, and neither can a strength above what was delivered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurfaceReport {
    pub surface: ContractSurface,
    /// The strengths the runtime says it took for the preventive carriers on
    /// this surface.
    pub preventive: BTreeMap<Control, EnforcementStrength>,
    /// The capabilities the runtime says it took on this surface.
    pub carried: BTreeSet<Capability>,
    /// How it took them, in the runtime's own terms. Bounded, nonempty and
    /// control-character free, so a report cannot smuggle payload.
    pub mechanism: String,
}

/// The runtime's own account of a session's surfaces. Deliberately no field for
/// canonical work: a handshake is an observation, and the events module owns the
/// completion request a worker sends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeHandshake {
    pub dispatch: DispatchId,
    /// One entry per surface the runtime has something to say about. A surface
    /// left out is reported as [`Agreement::Unreported`] rather than assumed,
    /// and naming one surface twice refuses: a report is one account of one
    /// dispatch's surfaces, not two accounts to choose between.
    pub surfaces: Vec<SurfaceReport>,
}

/// How a demanded carrier compares with what the runtime reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Agreement {
    /// Reported at exactly what was projected.
    Exact,
    /// Reported weaker than what was projected, with both strengths retained.
    Weaker,
    /// The runtime said nothing about this carrier.
    Unreported,
}

/// One demanded control's outcome.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ControlState {
    /// What the projection delivered for this control.
    pub delivered: PreventiveDelivery,
    /// The strength the runtime reported, or `None` when it said nothing.
    pub reported: Option<EnforcementStrength>,
    /// How the report compares with the delivery.
    pub agreement: Agreement,
}

/// One demanded capability's outcome.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CarrierState {
    /// What the descriptor declared for this capability.
    pub delivered: CarrierDelivery,
    /// Whether the runtime reported taking it.
    pub reported: bool,
    /// How the report compares with the declaration: a capability carries no
    /// strength, so it is either reported taken or unreported.
    pub agreement: Agreement,
}

/// One surface's outcome: what was projected for it and how the runtime's report
/// compares.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurfaceState {
    /// The place this outcome is about.
    pub surface: ContractSurface,
    /// The mechanism the runtime named, or `None` when it reported nothing for
    /// this surface at all.
    pub reported_mechanism: Option<String>,
    /// Every control this surface demanded, with how the report compares.
    pub preventive: BTreeMap<Control, ControlState>,
    /// Every capability this surface demanded, with how the report compares.
    pub carried: BTreeMap<Capability, CarrierState>,
}

/// A reconciliation: every surface, with the desired, projected and observed
/// states kept apart.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HandshakeOutcome {
    /// The session the runtime launched under the projected dispatch.
    pub session: SessionId,
    /// Every surface in [`ContractSurface::ALL`], in that order, whether or not
    /// the runtime mentioned it.
    pub surfaces: Vec<SurfaceState>,
}

impl HandshakeOutcome {
    /// One surface's outcome. The outcome covers every surface, so this is
    /// `None` only for a value that did not come from a reconciliation.
    pub fn surface(&self, surface: ContractSurface) -> Option<&SurfaceState> {
        self.surfaces.iter().find(|state| state.surface == surface)
    }

    /// True only when every demanded carrier — control and capability alike —
    /// was reported at exactly what was projected. A runtime that stayed silent
    /// about a carrier, took a weaker mechanism, or carries nothing for it,
    /// makes this false: there is no state in which silence counts as
    /// agreement.
    pub fn is_exact(&self) -> bool {
        self.surfaces.iter().all(|surface| {
            surface
                .preventive
                .values()
                .all(|state| state.agreement == Agreement::Exact)
                && surface
                    .carried
                    .values()
                    .all(|state| state.agreement == Agreement::Exact)
        })
    }
}
