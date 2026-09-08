# Reserved Change Stream worktree locations

Canonical owner: [#211](https://github.com/RepairYourTech/SymbioteIDE/issues/211), depending on #189 and respecting the #190 repository abstraction. `symbiote-worktrees` implements the location layer of Change Stream mutation isolation: deterministic collision-resistant naming plus filesystem reservation and re-verification of the exact location the [Linux sandbox](../security/linux-sandbox.md) requires its caller to reserve. This is not full #211 acceptance: no Git is executed, no repository worktree is created, no checkout or synchronization happens, and cleanup with evidence remains pending. Git/repository identity stays with #190; this crate never second-guesses it.

## Deterministic naming

`Derived::derive(DeriveInputs)` is a pure function of canonical Project/Root/Change Stream identities plus a trusted policy seed (1–64 characters of `[A-Za-z0-9_-]`, supplied by Host state, never client JSON). The same recorded identities and seed always derive the same names:

- `worktree_id`: `st-` plus 16 hash-hex characters. This is a bounded identity string, never a path; it is suitable for the canonical `WorktreeId` reference the store already treats as globally unique.
- `branch`: `symbiote/<project>/<stream>/<12-hex>` — a Git-ref-safe branch in a reserved namespace. Every component passes check-ref-format rules (no leading dot, no `.lock`, no `..`, no `@{`, safe charset), and branch length is bounded to 256 bytes.

Different seeds or identities derive different names. `derived(seed, ids)` feeds the store's existing worktree/branch uniqueness enforcement at Task creation; the seed choice is durable policy, because changing it changes every subsequent derived name.

## Reservation premises

`reserve(derived, base, created_at)` creates `<base>/<project>/<worktree_id>/` with an exclusive marker binding the derived identity and creation time. Premises match the sandbox launcher exactly:

- The base directory is absolute, bounded, symlink-free, and outside system trees (`/usr`, `/etc`, `/dev`, `/proc`, `/sys`, `/run`, `/boot`, `/var`). A missing base is created with mode 0700; an existing base must be euid-owned (root-owned components like `/tmp` are accepted with the sticky-bit discipline, mirroring the sandbox walk rule).
- The project directory and the reserved worktree are created with mode 0700 and verified euid-owned.
- The marker is written `O_EXCL` with mode 0600 and fsynced: concurrent reservations of the same identity have exactly one winner (`AlreadyReserved`), and a failed marker write removes the directory it created.

The marker lives in the private project directory, never inside the worktree, so later Git materialization keeps a clean tree. `verify(reservation)` re-derives nothing new and creates nothing: it re-walks base/project/worktree without following symlinks, checks ownership and modes, reads the marker, rejects identity mismatch (including a swapped stream identity), and rejects any content in the reserved directory — foreign use fails loudly instead of being absorbed. This is the provisioner-side premise the launcher relies on; the launcher still revalidates independently at launch.

## Release and content discipline

`release(reservation, policy)` has exactly two outcomes:

- `Retain` removes only the marker; the directory is kept for recovery/inspection and becomes an abandoned location, never removable work.
- `DeleteIfEmpty` removes marker and directory only when the directory holds no entries. Any content — including materialized Git state or uncommitted files — fails with `NotEmpty` and touches nothing. Unmerged or uncommitted work is never deleted recursively; that requires separate evidence and authorized confirmation under the Change Stream contract.

`list(base, project)` enumerates a project's markers; integrity is not re-verified by `list` — callers verify per reservation.

## Evidence and remaining acceptance

Covered by unit tests: determinism/collision resistance, seed/ref-safety rejection, reserve-verify-release round trip (both release policies, mode checks), single-winner reservation, tamper detection (foreign content, swapped marker identity, missing base not recreated), refusal to delete nonempty work, unsafe base rejection (system trees, relative paths, writable base, symlink components), and eight concurrent reservations without collision.

Pending #211 acceptance, tracked in the issue: actual `git worktree add` materialization against #190's repository abstraction, base-commit validation before dispatch, read-only vs sole-mutation vs parallel-mutation policy, no-worktree verified read-only tasks, synchronization/rebase/restack preparation, integration state, and evidence-confirmed cleanup. Store integration for recorded stream identity is exercised through the existing `create_task` worktree/branch uniqueness tests.

Applicability: security/privacy — the crate enforces the private-parent premise the sandbox documents and never follows symlinks; it grants no authority by itself and holds no credentials. Accessibility is not applicable to a filesystem provisioner library. Performance is bounded by path/component limits, not benchmarked. Linux-only today (`#![cfg(target_os = "linux")]`), matching the sandbox; Windows/macOS placement is separately pending with the platform work.
