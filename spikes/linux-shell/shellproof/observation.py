"""The host and the process tree as they ran, and what was left of them.

Every process this run can see, the memory each member of the app's own tree holds,
the component it is attributed to, the listening ports its members own, and the
app's own protocol log are read here. Nothing in this module decides anything about
what it observed: ``observe_app`` is one app lifetime, sampled, and ``cleanup``
kills only the identities this run observed and reports what remained. ``EXITED_STATES``
is the one distinction that matters to both: an entry that has exited but was never
reaped is not a survivor.
"""
import os
from pathlib import Path
import signal
import subprocess
import time

from shellproof.session import CANCEL_GRACE_SECONDS

SAMPLE_SECONDS = 0.1

# The states a ``/proc`` entry can have after the process it described has exited
# and before its parent reaps it. Such an entry runs nothing and owns no socket,
# so it is not a survivor; it is still not nothing, and the cleanup record names
# it instead of letting a survivor count either absorb it or lose it.
EXITED_STATES = 'ZX'


def processes():
    """Every process this run can see: parentage, identity, name and state, one read each.

    Only ``/proc/<pid>/stat`` is read per process, because the 100 ms sampling the
    contract's method asks for cannot afford the rest; the memory fields a member
    needs are read for the tree's own members, and by nobody else. The state comes
    from the same line, and is what separates a process that is running from a
    ``/proc`` entry that outlived one.
    """
    result = {}
    for path in Path('/proc').glob('[0-9]*/stat'):
        try:
            raw = path.read_text()
            name = raw[raw.index('(') + 1:raw.rindex(')')]
            fields = raw[raw.rindex(')') + 2:].split()
            result[int(path.parent.name)] = {
                'pid': int(path.parent.name),
                'name': name,
                'ppid': int(fields[1]),
                'start_ticks': int(fields[19]),
                'state': fields[0],
            }
        except (OSError, ValueError, IndexError):
            continue
    return result


def member_details(row):
    """The memory and identity fields of one tree member."""
    directory = Path('/proc') / str(row['pid'])
    try:
        status = dict(line.split(':', 1) for line in (directory / 'status').read_text().splitlines())
    except OSError:
        status = {}
    row = row | {
        'rss_kib': int(status.get('VmRSS', '0 kB').split()[0]),
        'pss_kib': pss_kib(directory / 'smaps_rollup'),
    }
    try:
        row['cmd'] = (directory / 'cmdline').read_bytes().decode(errors='replace').split('\0')[0]
    except OSError:
        row['cmd'] = ''
    return row


def pss_kib(smaps_rollup):
    """One member's proportional set size, or None where the kernel withholds it."""
    try:
        for line in smaps_rollup.read_text().splitlines():
            if line.startswith('Pss:'):
                return int(line.split()[1])
    except OSError:
        pass
    return None


def descendants(table, root_pid):
    """Every process descended from the app, by parentage rather than by name."""
    members = {root_pid}
    while True:
        expanded = members | {pid for pid, row in table.items() if row['ppid'] in members}
        if expanded == members:
            return members
        members = expanded


def classify(row):
    """The component a tree member belongs to, from the process itself.

    Attribution is by process identity: the app binary is the shell, the WebKit
    helpers are renderers, and the app's own Python children are the terminals. A
    member matching none of those is unattributed, and the unattributed share of
    tree PSS is reported rather than absorbed into a class.
    """
    command = Path(row['cmd'] or row['name']).name
    if command.startswith('symbiote-linux-shell-proof'):
        return 'shell'
    if row['name'].startswith('WebKit') or command.startswith('WebKit'):
        return 'webkit'
    if command.startswith('python'):
        return 'terminal'
    return 'unattributed'


def listening_ports(pids):
    """The listening TCP ports these processes own, by socket inode."""
    inodes = {}
    for relative in ('/proc/net/tcp', '/proc/net/tcp6'):
        try:
            lines = Path(relative).read_text().splitlines()[1:]
        except OSError:
            continue
        for line in lines:
            parts = line.split()
            if len(parts) > 9 and parts[3] == '0A':
                inodes[parts[9]] = int(parts[1].rsplit(':', 1)[1], 16)
    ports = set()
    for pid in pids:
        try:
            entries = list(Path(f'/proc/{pid}/fd').iterdir())
        except OSError:
            continue
        for entry in entries:
            try:
                target = os.readlink(entry)
            except OSError:
                continue
            if target.startswith('socket:[') and target[8:-1] in inodes:
                ports.add(inodes[target[8:-1]])
    return sorted(ports)


def sample(table, members, started):
    rows = [member_details(table[pid]) for pid in sorted(members) if pid in table]
    by_class = {}
    for row in rows:
        name = classify(row)
        by_class[name] = by_class.get(name, 0) + (row['pss_kib'] or 0)
    return {
        'seconds': round(time.monotonic() - started, 3),
        'members': len(rows),
        'sum_rss_kib': sum(row['rss_kib'] for row in rows),
        'sum_pss_kib': sum(row['pss_kib'] or 0 for row in rows),
        'pss_by_class_kib': by_class,
    }


def observe_app(env, log_path, binary, seconds, cancel_after, trace):
    """One app lifetime, sampled every 100 ms. Returns what the run observed."""
    app_env = env | {'SHELL_PROOF_SECONDS': str(seconds)}
    if trace:
        app_env['WAYLAND_DEBUG'] = '1'
    else:
        app_env.pop('WAYLAND_DEBUG', None)
    started = time.monotonic()
    seen, samples, marks = {}, [], {}
    offset, tail = 0, ''
    with log_path.open('w') as log:
        app = subprocess.Popen([str(binary)], env=app_env, stdout=log,
                               stderr=subprocess.STDOUT, start_new_session=True)
        try:
            while app.poll() is None and time.monotonic() - started < seconds + 60:
                tick = time.monotonic()
                table = processes()
                members = descendants(table, app.pid)
                seen.update({pid: table[pid]['start_ticks'] for pid in members if pid in table})
                samples.append(sample(table, members, started))
                # Only what the app appended since the last tick is searched, so a
                # long protocol log does not slow the sampling the method names.
                with log_path.open() as open_log:
                    open_log.seek(offset)
                    chunk = open_log.read()
                    offset += len(chunk.encode())
                text, tail = tail + chunk, chunk[-64:]
                for marker in ('PROOF_READY', 'PROOF_EXIT'):
                    if marker not in marks and marker in text:
                        marks[marker] = round(time.monotonic() - started, 3)
                if cancel_after and time.monotonic() - started > cancel_after:
                    marks['CANCEL_REQUESTED'] = round(time.monotonic() - started, 3)
                    os.killpg(app.pid, signal.SIGTERM)
                    time.sleep(CANCEL_GRACE_SECONDS)
                    break
                time.sleep(max(0.0, SAMPLE_SECONDS - (time.monotonic() - tick)))
            if app.poll() is None:
                os.killpg(app.pid, signal.SIGTERM)
                try:
                    app.wait(timeout=CANCEL_GRACE_SECONDS)
                except subprocess.TimeoutExpired:
                    os.killpg(app.pid, signal.SIGKILL)
                    app.wait(timeout=CANCEL_GRACE_SECONDS)
        finally:
            pass
    text = log_path.read_text()
    for marker in ('PROOF_READY', 'PROOF_EXIT'):
        if marker not in marks and marker in text:
            marks[marker] = round(time.monotonic() - started, 3)
    return app.returncode, round(time.monotonic() - started, 3), samples, seen, marks


def observed_in(seen, table):
    """The identities this run observed that ``table`` still lists, and their state.

    Identity is the pid *and* the birth ticks: a pid the kernel has reused is not
    the process this run saw, so a reused pid is absent here rather than counted.
    """
    return {pid: table[pid]['state'] for pid, ticks in seen.items()
            if pid in table and table[pid]['start_ticks'] == ticks}


def survivors_of(seen, table):
    """The processes this run observed that are still running, by identity.

    ``0 surviving processes`` has to mean nothing is left running, so an entry that
    has exited but was never reaped is excluded rather than counted.
    """
    return sorted(pid for pid, state in observed_in(seen, table).items()
                  if state not in EXITED_STATES)


def unreaped_of(seen, table):
    """The identities this run observed whose ``/proc`` entry outlived them."""
    return sorted(pid for pid, state in observed_in(seen, table).items()
                  if state in EXITED_STATES)


def cleanup(seen, sessions):
    """Kill only the identities this run observed, then report what remains.

    ``survivors_after_cancel_request`` counts processes still running when the
    cancel request was made, ``unreaped_observed_pids`` names the ones the kernel
    still listed after they exited, and ``remaining_observed_pids`` counts what is
    running after this function's own kill.
    """
    live = processes()
    after_request = survivors_of(seen, live)
    ports_after_request = listening_ports(after_request)
    unreaped = unreaped_of(seen, live)
    killed = []
    for pid in after_request:
        try:
            os.kill(pid, signal.SIGKILL)
            killed.append(pid)
        except ProcessLookupError:
            pass
    for process in sessions:
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        try:
            process.wait(timeout=CANCEL_GRACE_SECONDS)
        except subprocess.TimeoutExpired:
            pass
    time.sleep(.2)
    remaining = survivors_of(seen, processes())
    return {
        'survivors_after_cancel_request': len(after_request),
        'listening_ports_after_cancel_request': ports_after_request,
        'killed_observed_pids': killed,
        'remaining_observed_pids': remaining,
        'unreaped_observed_pids': unreaped,
    }


def trace_stats(trace_path):
    """What the client's own protocol log says about frames and first paint.

    libwayland timestamps every message, so the first ``wl_surface#N.commit()``
    after the app attaches a buffer is the first frame the compositor was handed,
    measured from the client's first request. The log keeps going for the whole
    run, so the number of commits and the last one it made say whether the client
    was drawing throughout or painted once and stopped.
    """
    stamp, first, committed, attached = None, None, 0, set()
    last = None
    for line in trace_path.read_text().splitlines():
        if not line.startswith('[') or ']' not in line:
            continue
        at = line[:line.index(']')].strip('[] ')
        stamp = stamp or at
        surface = next((token.split('.')[0] for token in line.replace('->', ' ').split()
                        if token.startswith(('wl_surface#', 'wl_surface@'))), None)
        if surface is None:
            continue
        if '.attach(' in line:
            attached.add(surface)
        elif '.commit()' in line and surface in attached:
            committed += 1
            last = at
            if first is None:
                first = at
    return {
        'first_frame_seconds': None if stamp is None or first is None else round(seconds_between(stamp, first), 3),
        'commits': committed,
        'last_commit_seconds': None if stamp is None or last is None else round(seconds_between(stamp, last), 3),
    }


def seconds_between(before, after):
    def parts(value):
        clock, _, micros = value.partition('.')
        hours, minutes, seconds = (int(part) for part in clock.split(':'))
        return hours * 3600 + minutes * 60 + seconds + int(micros.ljust(6, '0')[:6]) / 1e6
    return parts(after) - parts(before)
