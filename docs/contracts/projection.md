# Inactive configuration projection

Owner: [#187](https://github.com/RepairYourTech/SymbioteIDE/issues/187), consuming #184/#179/#214 foundations. `symbiote-projection` prepares explicit TOML mappings from resolved environment intent and publishes a new inactive profile generation. It does not discover native homes, import credentials, activate resources, reload a harness or change canonical task state.

## Preparation and ownership

`prepare_toml` requires an environment with no global blockers and exact eligible resource declarations for each pack mapping. A mapping for an older resource version or different payload is rejected even if its ID matches. Every required resource must have a nonempty mapping; missing formats cannot silently disappear. Pack declarations remain trusted adapter inputs whose real semantics require a compatibility dossier. These data checks are not Host authorization.

The TOML reconciler accepts explicitly managed primitive string, Boolean and integer leaf keys, with expected baseline and desired values. Baseline and current text are separate. Current managed values must equal the baseline or already equal the desired value; other edits conflict. Unknown keys and unrelated human edits remain unchanged. Replacements preserve key/value comments. A deletion that would discard a comment is refused. Duplicate/overlapping paths, malformed TOML, unmanaged keys, arrays, inline tables and other unsupported managed value shapes are refused. Untouched unsupported shapes are preserved as opaque TOML content.

Changes are ordered deterministically. String inputs and formatted output are bounded before unbounded candidate construction, including escaping expansion. Plans and semantic diffs expose private content only through explicit fields; their `Debug` implementations redact it. Callers must never supply credential stores or inline secrets as projection input. No native source file is read automatically.

## Publication and recovery

`generation::publish` requires an existing private directory owned by the current OS user, then creates a new generation without replacing an existing one. Every path component is opened relative to a directory descriptor with no symlink following. Public filenames are single restricted components; files use 0600 and generation directories 0700. The initial implementation allows at most 16 files, 1 MiB per file and 4 MiB total. Nested directory trees are not supported yet.

Each file is synchronized, followed by a bounded manifest and a final readiness marker; directories are synchronized before returning. The marker binds the manifest, which records exact filenames, byte counts and SHA-256 hashes. `verify` checks the exact entry set, permissions, readiness and content, rejecting symlinks, hardlinks, FIFOs, unexpected files and tampering. Verification is a point-in-time observation, not protection against a later write or a malicious actor running as the same OS user. Hashes detect drift; they are not signatures or permission grants.

Errors preserve the generation for inspection. A missing marker remains unusable; an error after marker creation can leave a complete generation with uncertain durability, so reconcile with `verify` rather than assuming nothing was written. Retrying the same generation name never overwrites it. A later human edit is preserved and causes verification failure. There is no automatic cleanup or in-place rollback: existing profiles remain untouched, and a failed inactive candidate is never selected by this library. Future activation must persist the chosen generation, authorize the contract and install Host controls before executable configuration can load.

## Reproduction and remaining acceptance

Run `cargo test -p symbiote-projection --locked` for format, preparation and real-filesystem fixtures. Tests cover concurrent publishers, modified managed keys, unmanaged edits, drift, partial directory states and private path handling. Partial-state fixtures construct incomplete publications; they do not certify arbitrary process-kill or power-loss boundaries.

For a runnable fixture, create a private scratch directory and run:

```sh
mkdir -m 700 /tmp/symbiote-projection-demo
cargo run -p symbiote-projection --example projection_demo -- \
  /tmp/symbiote-projection-demo first-generation
```

The demo publishes and verifies fixture TOML. It performs no inference and reads no real runtime home. Repeating the generation name refuses replacement.

Real Codex/native/other pack mappings, per-version golden homes and precedence, credential-safe native import, JSON/JSONC/YAML/Markdown/other patchers, directory trees, native uptake, durable activation/rollback, trust enforcement and cross-platform runtime evidence remain pending. #187 remains open. Linux is the tested filesystem platform; there is no UI accessibility surface in this library. No paid inference, credential changes or existing-native-file migration occurs.
