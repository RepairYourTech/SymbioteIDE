# Configuration and portable Project intent v1

Canonical owners: [#176](https://github.com/RepairYourTech/SymbioteIDE/issues/176), then [#179](https://github.com/RepairYourTech/SymbioteIDE/issues/179). This is a prerequisite library slice, not completion of either issue. It consumes the #170 constitution and #36 runtime kind. There are no file writes, installed-provider checks, Host execution, credential reads, or client synchronization in this crate.

## Contract

`symbiote-config` exports strict serde contracts and JSON Schema generation through `cargo run -p symbiote-config --example config_schema`. `ProjectManifest::parse` accepts v1 partial declarations with explicit `schema_version` and stable `project_id`. Missing collections are empty; absent optional fields remain unspecified. Opening a declaration needs neither a harness installation nor provider credentials. No project identity is inferred from its display name.

Manifest intent includes portable relative roots, declared commands, Lead Role reference, per-Role runtime/profile/provider/entitlement references and independently assigned skills/MCPs/hooks/rules/permission policies, environment and harness requirements, orchestration/context/verification/documentation/Git/delivery policy references, graph profile and learning/telemetry/retention/delegation/goal/analytics preferences. References deliberately do not resolve or install anything. `RuntimeKind` comes from the domain crate; assignments reference Roles instead of duplicating their lifecycle.

Unknown core fields are rejected. Future namespaced extension payloads under `extensions` round trip unchanged and are never interpreted or executed. Sorted maps/sets give deterministic serialization. `managed` is a separate source/revision map keyed by field path; fields without entries remain user-authored. Importers must preserve user content and supply authentic provenance; this library cannot certify importer authorship.

Portable root syntax uses `/` separators and canonical relative components. A single `.` denotes the manifest directory. Empty/parent/dot interior components, control characters (including NUL), Windows-invalid characters, reserved device names (also with extensions), and trailing dots/spaces are rejected on every Host. Ordinary Unicode and interior spaces are allowed. This is a conservative declaration policy, not filesystem existence, symlink-containment, case-collision or OS path-length verification. Host-specific paths belong in local `MachineBindings` and require separate Host validation.

## Scope and storage

| Scope | Specificity | Storage classification | Selection |
| --- | --- | --- | --- |
| Application defaults | 1 | Version controlled | Caller-selected defaults |
| User preferences | 2 | User synchronized | Caller-selected authenticated user |
| Project policy preferences | 3 | Version controlled | Project identity |
| Workspace roots | 4 | Version controlled | Project and root |
| Repository metadata preferences | 5 | Version controlled | Project and repository |
| Role preferences | 6 | Version controlled | Project and Role |
| Explicit local override | 7 | Local only | Exact Project/root/repository/Role/controller context |
| Machine/Host bindings | Outside inheritance | Local only | Explicit local binding map |
| Secret bytes | Outside inheritance | Prohibited from config disk serialization | Non-serializable `SecretValue` |

Unspecified preferences inherit. More-specific supplied values override; unequal values at equal scope return a deterministic diagnostic instead of last-write-wins. Equal values retain every contributing source. Resolved output records ordered source identities, revisions and scope per field, including overridden inputs. Invalid scoped selectors and unknown versions fail. Security permissions and consent authorization are not ordinary overrideable preferences: the Host must enforce authoritative policy intersections before acting on resolved requests. `learning = true` alone is not consent evidence. Authorized-project analytics preference is not source/credential sharing authority.

`MachineBindings` holds Host identity, local root paths, credential *references*, and harness installation paths. These are outside `ProjectManifest`. There is no API-key/token/auth/session property in the portable core. Secret values have no serde or schema implementation and must never be copied into commands, reference strings, or extensions. Arbitrary authored strings and opaque extensions are **not** a secret detector; secure import/export scanning and credential-store integration remain required. `SecretValue` does not claim memory zeroization or a secure vault implementation.

`ControllerState::switch_project` changes only one controller's presentation identity. Its signature cannot cancel execution or change another controller. Serializing/restoring this state is a local contract test, not evidence of a real desktop restore or multi-controller Host operation. Rename affects `name`, not stable `project_id`.

## Merge, migration and recovery

`merge(base, left, right)` validates all inputs and rejects different Projects. Disjoint object-field edits combine; arrays/sets are atomic. Same-location unequal changes and delete-versus-edit produce sorted JSON-pointer conflicts containing base/left/right values. Conflicted candidate positions keep the base only for review. `finish()` refuses any conflict and validates a conflict-free candidate. Resolve conflicts by explicitly editing the inputs and previewing again; there is no automatic last writer.

`preview_migration` retains the original JSON. Only validated v1→v1 identity migration is registered. Older→v1, future→v1 and v1→other versions return unsupported previews; no history or lossy coercion is invented. Applying migrations, atomic file replacement, destructive-reconciliation preview, backup/rollback and real interrupted-write recovery await the persistence/import owners. Original inputs remain untouched by these pure APIs.

## Evidence and remaining acceptance

`cargo test -p symbiote-config` covers partial manifests without auth, both unsupported migration directions, unknown core/auth fields, portable-root rejection, extension determinism, managed-source separation, distinct Project preferences, controller-local switching/restore, local overrides/provenance, equal-scope conflicts, Role resources across runtime changes, disjoint/conflicting/delete-edit merges and storage classification.

Pending #176/#179 acceptance: actual client simultaneous-controller isolation and rename/restore; physical scope storage and authorization; real credentials/provider/billing validation; complete policy compilation and runtime resource projection; clone/open on a second Host; import/diff/repair/export commands; user-content-preserving native projection; migration application/recovery; exact-commit review and checks. No mocks are counted as these integrations. Runtime availability blocks dependent execution only, never parsing a Project declaration.

Applicability: security/privacy review is required for secret handling, permission enforcement and synchronization boundaries; this slice checks type and parse boundaries only. Accessibility is not applicable to a headless contract library, and remains required for future editors/diagnostics UI. Performance is unmeasured; this slice performs bounded-by-input in-memory operations without resource-budget claims. Paths are validated portably without OS I/O; actual Windows/macOS/Linux Host bindings remain separately required. Later schema revisions must add bidirectional migration fixtures or explicit unsupported/rollback evidence before changing persisted state.
