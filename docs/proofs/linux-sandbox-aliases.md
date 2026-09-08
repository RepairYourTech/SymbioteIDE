# Linux sandbox alias and parent-death probes

Local evidence: bubblewrap 0.12.0, `/usr/bin/bwrap`, Linux development host. These are bounded private temporary-directory probes, not Ubuntu CI evidence or complete sandbox acceptance. No host policy was changed.

## Exact namespace configuration

```text
bwrap --unshare-all --unshare-user --disable-userns --assert-userns-disabled
  --die-with-parent --new-session --cap-drop ALL --clearenv
  --ro-bind /usr /usr
  --symlink usr/bin /bin --symlink usr/lib /lib --symlink usr/lib /lib64
  --proc /proc --dev /dev --tmpfs /tmp
  --bind PRIVATE_WORKTREE /work
  /usr/bin/python3 -c FIXTURE
```

The `/lib` symlinks reflect this host. Ubuntu library layouts and namespace/AppArmor permission must be verified on its actual CI runner; do not infer support from this result or relax failures into unconfined execution.

`--unshare-all --disable-userns` without explicit `--unshare-user` failed with `--disable-userns requires --unshare-user`. With the explicit flag, sandboxed `unshare -Ur /usr/bin/true` failed with `No space left on device`, exit 1. The installed bwrap manual describes disabling further user namespaces through a nested namespace and a namespace-local limit. Dropping capabilities alone does not replace that restriction.

## Worktree aliases bypass the mount boundary

Reproducible private fixture logic:

1. Create a temporary parent directory, an empty `work` child directory, and an `outside` regular file containing `original`.
2. `os.link(outside, work / "alias")` creates a hardlink into the worktree.
3. Bind a host Python `AF_UNIX` stream listener to `work / "host.sock"`.
4. Launch the configuration above, with a five-second subprocess timeout, executing:

```python
import socket, pathlib
s = socket.socket(socket.AF_UNIX)
s.connect('/work/host.sock')
s.send(b'probe')
pathlib.Path('/work/alias').write_text('changed')
```

5. Host accepts the connection, reads `probe`, and checks `outside` contents. Close all sockets and remove only the private fixture.

Observed output:

```text
sandbox_exit 0
host_pathname_socket_reached True
outside_hardlink_modified True
```

Network namespaces do not isolate a pathname Unix socket exposed through a filesystem bind. A hardlink names the same inode, so a writable mount can modify content outside the worktree. A read-only bind prevents this write but still exposes the inode's content: confidentiality is not established merely by making the alias read-only. The read-only case is a filesystem consequence, not a separately executed probe here.

Reject sockets, special files and external hardlink aliases before binding. A scan is insufficient if another host process can concurrently insert or replace entries after validation. The launcher must require trusted, nonconcurrent provisioning or use an independently protected staged tree that breaks hardlinks and excludes sockets/special files. Directory symlinks, nested mounts, inherited file descriptors and later host writes also need explicit control. Ordinary Git worktree `.git` links to outside administration state require a separate restricted design; do not expose the entire source home to repair Git access.

## Parent death with a setsid descendant

A private Python supervisor launched bwrap with the exact flags above. Inside it, a shell launched `setsid python3`, whose child wrote a heartbeat to `/work/heartbeat` every 30 ms. The outer probe sampled the host `/proc` descendant tree and recorded each PID's start time, killed only its own supervisor, then compared heartbeat content and process states after 300 ms and a further 200 ms. Any still-live matching owned PIDs would have been cleaned up with SIGKILL; none required cleanup.

```text
setsid_heartbeat_observed True
heartbeat_stopped True
observed_live_descendants_after_parent_death 0
```

This proves cleanup for this observed descendant tree, including a new session, on this host. It relies on the PID namespace and bwrap parent-death behavior, not a process-group-only kill claim. It does not establish CPU/memory/disk quotas, protection against kernel vulnerabilities, arbitrary descendant escape prevention, or cross-platform support.
