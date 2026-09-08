# Host inventory foundation (#193)

The authenticated local Host exposes `get_host_pulse` in protocol v1.5. A pulse
contains a private installation identity, an observation identity, observation
and expiry times, probe provenance, resource facts and capability observations.
It is telemetry, not canonical completion evidence or a reservation/launch permit.
The existing resource-discovery `Fact` contract preserves explicit unknowns.

## Actual observations and limitations

The passive Linux probe uses fixed bounded reads of `/proc/meminfo` and
`/proc/stat`. It reports logical CPUs, physical memory and kernel MemAvailable
when those records validate. OS and architecture describe the compiled target.
Failures affect their own facts and retain static reasons. No subprocess,
container engine, network, account or provider configuration is accessed.

Physical totals are not schedulable capacity. Effective CPU and memory limits
remain unknown because cgroup/affinity limits, reservations and enforceable
pressure policies are not integrated. Qualification requiring capacity uses the
effective fields and rejects unknowns. GPU/VRAM, disk, Docker, toolchains,
signing, models, inference servers and Project services remain unverified.
Installed, authenticated and healthy capability claims stay distinct; one cannot
substitute for another. Discovery observations must be attached through trusted
Host collection before qualification, not accepted from arbitrary client claims.

## Identity, privacy and sampling

`host-id` is a random private identity stored beside `control.sqlite3`. Creation
uses a temporary file, file sync, atomic rename and directory sync under the Host
lock. Reads reject symlinks, special files, malformed content and incorrect
ownership/mode. The identifier survives daemon restart without reading a machine
ID, hostname or account name. Preserve this file with same-Host backups; cloned
state on a different Host requires explicit future enrollment, not identity reuse.
This does not implement Fabric signing or remote Host enrollment.

Only the authenticated local Host owner may read the pulse. Project permissions
alone do not grant access to machine resources. Repeated requests within one
second return the same cached observation, including its original expiry; reads
never extend freshness. Samples expire after five seconds. Polling is bounded
sampling, not a background telemetry subscription. Restart discards cached facts
and creates new observation IDs.

Start with `symbioted --state-dir PRIVATE_DIRECTORY --no-telemetry` to disable
resource sampling. The endpoint then reports disabled telemetry and unknown
resource facts, not fabricated capacity or a refreshed previous observation.
The switch is explicit and has no global environment/configuration side effects.

## Verification and remaining acceptance

The crate tests malformed, duplicate, missing and overflowing kernel fields;
stale/mismatched observations; unknown capacity; and disabled telemetry. Daemon
tests exercise authenticated protocol behavior, caching, identity persistence,
restart and telemetry disablement. Private identity tests include FIFO/symlink
rejection so malformed state fails rather than hanging at startup.

`cargo build -p symbiote-host --bins` builds the daemon and CLI. With the daemon
running, send `fixtures/host-inventory/read.json` through
`symbiote --state-dir PRIVATE_DIRECTORY request` to inspect a real pulse.
Upgrade CLI and daemon together for protocol v1.5. This batch adds no database
migration. Older binaries ignore `host-id`; preserve it when rolling back.

Broad #193 remains open: atomic resource reservations/releases, pressure policies,
effective container limits, signed Fabric responses, additional capability
probes, static override validation and streaming telemetry are not complete.
This is Linux integration evidence; Windows/macOS probes and transport remain
pending. No UI is added, so accessibility acceptance stays with the workbench.
Neither this inventory nor a structurally valid Team enables agent execution.
