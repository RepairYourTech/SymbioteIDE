# Resource consent

Owners #174/#191; runtime enforcement integration #218. `symbiote-trust` models a resource snapshot and the local owner's expiring consent. Snapshots bind Project, Role, runtime profile, Host, resource reference, exact byte fingerprint, roots, permissions and policy revision. SHA-256 retains no original bytes; it does not authenticate a publisher or prove a dependency closure. Metadata identifiers must not contain secret values.

`authorize_load` validates the consent and exact observed snapshot, Host identity, validity interval, revocation and current policy. Requested roots and permissions must remain within current policy at the same revision. A changed fingerprint or context cannot reuse the consent. This is a pure check over caller-supplied records; it neither authenticates those records nor launches a resource. A real Host loader must obtain consent from the trusted store, recompute the loaded content and close the check/use race.

## Host operations and journal

Protocol v1.1 adds `record_resource_consent {snapshot, expires_at}`, `get_resource_consent {project_id, consent_id}` and `revoke_resource_consent {project_id, consent_id}`. Recording and revoking require the authenticated local owner; Project register/create/read grants do not authorize them. The Host derives the approving/revoking user from its principal and issuance/revocation time from its clock. The record command ID becomes the consent ID. No wire user, actor, issuance time or revoked state is accepted.

Storage validates Project/Role/root relationships and commits current consent, journal event and command receipt atomically. Profile and Host registry membership are not yet backed by persistent registries; their identities are bound metadata, not discovery evidence. Replays reuse the original timestamp and receipt; a changed payload or actor under the same command ID conflicts. Revocation is permanent for that consent; a new decision needs a new ID. Reads bind the requested Project to stored ownership. Expired or revoked consent never supplies permission to load.

The journal stores `resource_consent_recorded` and `resource_consent_revoked`, with the revoking user in the latter. Payloads carry metadata only. Existing journal replay and integrity reconstruction include these events, and a failed journal append rolls back the consent mutation.

## Upgrade, verification and limits

SQLite schema v2 adds the consent table transactionally to valid v1 stores. Older binaries refuse the newer schema; there is no downgrade. Upgrade the CLI and daemon together: v1.1 explicitly rejects v1.0 envelopes instead of sending new event/capability variants to old strict clients. Stop the daemon and preserve a consistent database backup before upgrading valuable existing state; restore the complete prior database with the prior binary if rollback is required. Automated backup/export remains pending under #43.

Run `cargo test -p symbiote-trust -p symbiote-store -p symbiote-protocol -p symbiote-host --locked`. Tests cover altered snapshots, scope/grant/revision changes, expiry/revocation, server-compiled authority, exact retries, cross-Project denial, transaction rollback, migration and daemon death/restart. They do not certify OS containment, publisher signing, active-session revocation or resource loading. The current Host has no executable resource activation endpoint.

See [the threat register](../security/trust-boundaries.md) for the same-UID worker risk and remaining release requirements. Pure contracts are platform-neutral; actual daemon evidence is Linux-only. Approval UI accessibility, Windows/macOS peer identity and the complete native/Codex task demonstration remain pending.
