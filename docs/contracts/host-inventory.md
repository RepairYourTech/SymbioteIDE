# Host inventory foundation (#193)

The authenticated local Host exposes `get_host_pulse` in protocol v1.5. A pulse
contains a private installation identity, an observation identity, observation
and expiry times, probe provenance, resource facts and capability observations.
It is telemetry, not canonical completion evidence or a reservation/launch permit.
The existing resource-discovery `Fact` contract preserves explicit unknowns.

## Actual observations and limitations

The passive Linux probe uses fixed bounded reads of `/proc/meminfo`,
`/proc/stat`, `/proc/self/status` and the process's own cgroup under the unified
(cgroup v2) hierarchy: `/proc/self/cgroup` names the cgroup, `memory.max`,
`memory.current` and `cpu.max` say what the kernel actually enforces on this
process, and `Cpus_allowed_list` says which CPUs it will schedule this process
on. It reports logical CPUs, physical memory, kernel MemAvailable, the enforced
memory ceiling, the effective memory availability under it and the effective CPU
in millicores when those records validate. OS and architecture describe the
compiled target. Failures affect their own facts and retain static reasons. No
subprocess, container engine, network, account or provider configuration is
accessed.

Physical totals are not schedulable capacity. An enforced cgroup ceiling is:
the effective availability is the headroom under it (`memory.max` minus
`memory.current`), never more than the machine's own observed availability. A
ceiling the kernel reports as `max` imposes no bound, so the cgroup's effective
availability is the machine's observed availability and no ceiling is claimed;
the effective memory *limit* stays unknown with no failure recorded, because
there is no enforced limit to report. Effective CPU is the CPUs this process may
actually run on: those `Cpus_allowed_list` names — which already reflect any
cgroup cpuset and any process affinity — restricted to the online CPUs
`/proc/stat` names, then capped by any enforced `cpu.max` quota, in millicores.
The machine's CPU total is never substituted for that bound: when any of the
three sources cannot be read the fact is unknown with that source's static
reason, because a quota that cannot be seen may still bind. Being allowed no
online CPU is a measurement of zero, not a failure, and a quota the cgroup does
not enforce leaves the allowed set as the bound rather than an unknown. A host
whose hierarchy cannot be read (no unified line, missing or malformed files, a
platform without cgroups) reports the affected facts unknown with the static
reason (`unsupported_hierarchy`, `unreadable`, `malformed`), so an unobserved
fact is never replaced by the machine's totals. Qualification
requiring capacity uses the effective fields and reports an unobserved one as a
missing observation, not as a failed Host. GPU/VRAM, disk, Docker, toolchains,
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

The crate tests malformed, duplicate, missing and overflowing kernel and cgroup
fields, path traversal in `/proc/self/cgroup`, ceilings, charges and quotas, and
CPU lists and masks (repeated, overlapping, descending, unparsable and larger
than one observation will expand); stale/mismatched observations; unknown
capacity; and disabled telemetry. Daemon tests exercise authenticated protocol
behavior, caching, identity persistence, restart and telemetry disablement.
Private identity tests include FIFO/symlink rejection so malformed state fails
rather than hanging at startup.

`cargo build -p symbiote-host --bins` builds the daemon and CLI. With the daemon
running, send `fixtures/host-inventory/read.json` through
`symbiote --state-dir PRIVATE_DIRECTORY host-pulse` to inspect a real pulse.
Upgrade CLI and daemon together for protocol v1.20. This batch adds no database
migration. Older binaries ignore `host-id`; preserve it when rolling back.

Broad #193 remains open: atomic resource reservations/releases, pressure policies,
signed Fabric responses, additional capability probes, static override
validation, cgroup v1/hybrid hierarchies and streaming telemetry are not
complete. This is Linux integration evidence; Windows/macOS probes and transport
remain pending. No UI is added, so accessibility acceptance stays with the workbench.
Neither this inventory nor a structurally valid Team enables agent execution.
