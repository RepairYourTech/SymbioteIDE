#!/usr/bin/env python3
"""Run only this experimental app in a disposable session; no user display access.

Two sessions are supported, and both are private to the run:

* ``--session xvfb`` (default) creates its own Xvfb X11 server, as documented in
  ``docs/proofs/linux-shell.md``.
* ``--session wayland`` creates its own ``kwin_wayland --virtual`` compositor in a
  private ``XDG_RUNTIME_DIR`` and runs the app with no ``DISPLAY`` at all. The
  session is accelerated on this host through Mesa's EGL ICD because the
  NVIDIA/glvnd path aborts the client with ``wp_linux_drm_syncobj_surface_v1``
  explicit-sync errors against the virtual backend; that is recorded with the run
  rather than worked around in the app.

Both sessions sample the app's own process tree every 100 ms: RSS, PSS (per
member and summed), the component each member is attributed to, and — around
cancellation — the listening ports its members own. The Wayland session writes a
run dossier and, when asked, the result artifact its contract points at. Nothing
here touches the user's session, display or packages.

The result artifact is written from what the run observed, and it is checked
before the run reports success:

* the platform it names must be one the committed contract applies to, its stop
  condition must be one the contract declares, and every predeclared measurement
  must be either observed or named with the reason the instrument could not see
  it — a measurement that is simply absent is refused rather than left silent;
* an obligation is attested only where the fixture's own log carries the marker
  for it (see ``ATTESTED_BY``), so ``exercised`` cannot be typed;
* the run itself refuses to record a revision while the tree differs from it, and
  the artifact it writes is then read back by the ledger's own map, which must
  report no refusals;
* a run merges into the dossier it was pointed at rather than replacing it: a
  settlement needs one artifact holding every platform the contract applies to, and
  this driver runs one platform at a time, so another platform's runs are kept
  exactly as they stand and the untested list is derived from the runs the dossier
  actually holds; and the clean build it cites has to have logged the revision the
  run records, because a log that names only the commands cannot be tied to the
  tree the figures came from.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]
SOCKET_NAME = 'symbiote-proof'
MESA_EGL_ICD = '/usr/share/glvnd/egl_vendor.d/50_mesa.json'
SAMPLE_SECONDS = 0.1
CANCEL_GRACE_SECONDS = 5
CONTRACTS_PATH = 'docs/architecture/spike-contracts.json'
_RENDERERS = {}

# The contract obligations this fixture's own log can attest, the marker the
# fixture emits for each, and how many of them one run has to show. An
# attestation is read from the app's log and never from the command line, so an
# obligation whose marker is missing stays unattested: four concurrent agent
# streams are absent below because this fixture starts four *synthetic* streams,
# not agent streams, and no marker of its own says otherwise.
ATTESTED_BY = (
    ('#38: three active terminal tabs with bounded scrollback in real PTYs',
     'PROOF_PTY_START', 3),
    ("#38: a live integrated Preview of the run's own output",
     'PROOF_READY preview_origin=', 1),
    ('#38: Preview origins, localhost included, that inherit neither workbench nor Host authority '
     'and reach the app only through validated bridge operations',
     'PROOF_PREVIEW_REPORT', 2),
)


def attested(log_text):
    """The contract obligations this log attests, and nothing it does not show."""
    return [obligation for obligation, marker, least in ATTESTED_BY
            if log_text.count(marker) >= least]


def processes():
    """Every process this run can see: parentage, identity and name, one read each.

    Only ``/proc/<pid>/stat`` is read per process, because the 100 ms sampling the
    contract's method asks for cannot afford the rest; the memory fields a member
    needs are read for the tree's own members, and by nobody else.
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


def start_xvfb(artifacts):
    log = (artifacts / 'xvfb.log').open('w')
    read_fd, write_fd = os.pipe()
    server = subprocess.Popen(
        ['Xvfb', '-displayfd', str(write_fd), '-screen', '0', '1440x960x24', '-nolisten', 'tcp'],
        pass_fds=(write_fd,), stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
    os.close(write_fd)
    with os.fdopen(read_fd) as pipe:
        display = ':' + pipe.readline().strip()
    if display == ':':
        raise RuntimeError('Xvfb failed to allocate display')
    env = os.environ | {'DISPLAY': display, 'GDK_BACKEND': 'x11'}
    env.pop('WAYLAND_DISPLAY', None)
    return server, env, log, f'Xvfb {display}'


def start_wayland(artifacts):
    runtime = artifacts / 'runtime'
    runtime.mkdir(mode=0o700, exist_ok=True)
    log = (artifacts / 'kwin.log').open('w')
    env = os.environ | {
        'XDG_RUNTIME_DIR': str(runtime),
        'WAYLAND_DISPLAY': SOCKET_NAME,
        '__EGL_VENDOR_LIBRARIES': MESA_EGL_ICD,
        '__EGL_VENDOR_LIBRARY_FILENAMES': MESA_EGL_ICD,
    }
    env.pop('DISPLAY', None)
    env.pop('GDK_BACKEND', None)
    socket_path = runtime / SOCKET_NAME
    if socket_path.exists():
        socket_path.unlink()
    compositor = subprocess.Popen(
        ['dbus-run-session', '--', 'kwin_wayland', '--virtual', '--socket', SOCKET_NAME,
         '--width', '1440', '--height', '960', '--no-lockscreen'],
        env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
    for _ in range(120):
        if socket_path.exists():
            break
        if compositor.poll() is not None:
            raise RuntimeError('the compositor exited before it listened')
        time.sleep(.5)
    else:
        raise RuntimeError('the compositor never created its socket')
    version = subprocess.run(['kwin_wayland', '--version'], capture_output=True, text=True,
                             env=env).stdout.strip()
    return compositor, env, log, runtime, f'{version} --virtual 1440x960, {renderer_of(env)}'


def renderer_of(env):
    """The GPU EGL renderer inside this session, so the figures carry their stack."""
    key = env.get('XDG_RUNTIME_DIR', '')
    if key in _RENDERERS:
        return _RENDERERS[key]
    try:
        output = subprocess.run(['eglinfo'], capture_output=True, text=True, env=env,
                                timeout=60).stdout
    except (OSError, subprocess.SubprocessError):
        output = ''
    _RENDERERS[key] = next((line.split(':', 1)[1].strip() for line in output.splitlines()
                            if line.strip().startswith('OpenGL core profile renderer:')),
                           'renderer not reported')
    return _RENDERERS[key]


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


def survivors_of(seen, table):
    return sorted(pid for pid, ticks in seen.items()
                  if pid in table and table[pid]['start_ticks'] == ticks)


def cleanup(seen, sessions):
    """Kill only the identities this run observed, then report what remains."""
    live = processes()
    after_request = survivors_of(seen, live)
    ports_after_request = listening_ports(after_request)
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


def sha256_of(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def hardware_of(env):
    """The reference configuration this run happened on, read from the host.

    A run's figures carry the machine they were taken on, and a typed sentence
    cannot be re-checked; every part of this one is read here, and the renderer
    is the one the session's own EGL stack reports. ``session.json`` commits it
    beside the run that cites the record.
    """
    cpu = next((line.split(':', 1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines()
                if line.startswith('model name')), 'unknown CPU')
    kib = next((line.split(':', 1)[1].strip().split()[0]
                for line in Path('/proc/meminfo').read_text().splitlines()
                if line.startswith('MemTotal')), '0')
    storage = 'NVMe storage' if any(path.name.startswith('nvme')
                                    for path in Path('/sys/block').glob('*')) else 'unreported storage'
    return (f'{cpu} ({os.cpu_count()} logical cores), {round(int(kib) / 1048576)} GiB RAM, '
            f'{renderer_of(env)}, {storage}')


def repo_path(path):
    """A path as the artifact cites it, or as given when it is outside the tree.

    The binary a run exercises may have been built from a clean target directory
    outside this tree, which is how a clean locked build is measured; its path is
    then recorded as it is, with its hash, rather than made up relative to a root
    it is not under.
    """
    try:
        return str(Path(path).resolve().relative_to(REPO))
    except ValueError:
        return str(Path(path).resolve())


def session_record(session, env, binary, runtime, compositor_log_bytes, build):
    """What this session was, so the figures carry their stack and its compromises."""
    def tool(*command):
        try:
            return subprocess.run(command, capture_output=True, text=True,
                                  timeout=30).stdout.strip().splitlines()[0]
        except (OSError, IndexError, subprocess.SubprocessError):
            return 'not reported'
    return {
        'session': session,
        'runtime_dir': str(runtime),
        'wayland_display': env.get('WAYLAND_DISPLAY'),
        'display_unset': 'DISPLAY' not in env,
        'egl_vendor_icd_pinned': env.get('__EGL_VENDOR_LIBRARY_FILENAMES'),
        'egl_icd_reason': 'the NVIDIA/glvnd path aborts the client against the virtual backend with wp_linux_drm_syncobj_surface_v1 explicit-sync errors, so the session runs on Mesa EGL and says so here',
        'compositor_log_bytes': compositor_log_bytes,
        'compositor_log_note': 'the compositor writes nothing to the stream this run captures, so the display log is empty and is not cited as evidence; the session it started is recorded here instead',
        'binary': repo_path(binary),
        'binary_sha256': sha256_of(binary),
        'rustc': tool('rustc', '--version'),
        'cargo': tool('cargo', '--version'),
        'hardware': hardware_of(env),
        'build': build,
    }


def contract_of(path, identity):
    """The contract a run measured, from the document the repository commits."""
    document = json.loads(path.read_text())
    for contract in document['contracts']:
        if contract['id'] == identity:
            return contract
    raise SystemExit(f'no committed contract named {identity!r} in {path}')


def uncommitted(text):
    """Tracked files that differ from HEAD, from ``git status --porcelain``.

    Untracked files are the run's own new evidence and are left alone; a modified
    tracked file means the revision a run would record is not the tree it ran, and
    the run refuses to publish rather than record a revision it cannot stand on.
    """
    return [line for line in text.splitlines() if line[:2] not in ('??', '')]


def platform_key(value):
    """One platform, as a name is compared: trimmed and case-insensitive.

    The contract, the artifact and the ledger all name platforms as strings and the
    ledger compares them this way, so a run that spelled its own platform
    differently from the contract would otherwise be counted as a second platform.
    """
    return value.strip().lower()


def publish_slug(platform):
    """The directory one platform's artifacts live in inside the publish root.

    Every run cites its own app, process-tree, session, cleanup and build records,
    whose file names do not differ between platforms; publishing each platform into
    its own directory is what keeps a second platform's run from overwriting the
    first one's evidence as easily as it replaces its run entries.
    """
    words = ''.join(character if character.isalnum() else ' ' for character in platform.lower()).split()
    return '-'.join(words) or 'unnamed'


def logged_revision(text):
    """The revision a build recipe logged, from a line that is nothing but a hash.

    ``set -x`` echoes the recipe, so a build that runs ``git rev-parse HEAD``
    before it compiles writes the revision it read on a line of its own.
    """
    for line in text.splitlines():
        token = line.strip()
        if 7 <= len(token) <= 40 and all(character in '0123456789abcdef' for character in token):
            return token
    return None


def revision_problems(commit, head, status):
    """Why a run may not record this revision, if it may not.

    Nothing in the tree can re-derive a build's revision from a build log, so the
    revision a run records has to be the tree it ran in — clean, at HEAD — and the
    recipe it cites has to have logged the same revision. Anything else is a claim
    about a tree nobody can find.
    """
    problems = []
    changed = uncommitted(status)
    if changed:
        problems.append('the tree differs from HEAD in:\n    ' + '\n    '.join(changed))
    if commit and head and commit != head:
        problems.append(f'the run would record revision {commit} and this tree is {head}')
    return problems


def untested_platforms(contract, runs):
    """The contract's platforms this run set holds no run for.

    Read from the runs the dossier actually carries, never from the platform this
    invocation was pointed at: a platform with recorded runs is measured, and one
    another invocation already recorded is not declared untested by this one.
    """
    measured = {platform_key(run['platform']) for run in runs}
    return [platform for platform in contract['applicable_platforms']
            if platform_key(platform) not in measured]


def merged_runs(path, contract, fingerprint, platform, entries):
    """This run's entries, merged into the dossier the artifact already holds.

    A settlement needs one artifact carrying the runs of every platform its
    contract applies to, and this driver runs one platform at a time, so a run
    merges rather than replaces: every other platform's runs stay exactly as they
    stand — same figures, same citations, still the ones the ledger re-hashes — and
    this platform's own entries are replaced by what this invocation observed. A
    dossier of another contract, or one measured against other thresholds, is
    refused rather than merged into, because the figures it holds were not measured
    against this contract.
    """
    if not path.exists():
        return entries
    document = json.loads(path.read_text())
    if document.get('contract') != contract['id']:
        raise SystemExit(f'refusing to merge: {path} holds {document.get("contract")!r}, '
                         f'not {contract["id"]!r}')
    if document.get('contract_sha256') != fingerprint:
        raise SystemExit(f'refusing to merge: {path} was measured against contract '
                         f'{document.get("contract_sha256")}, and this run answers {fingerprint}')
    held = [run for run in document.get('runs', [])
            if platform_key(run.get('platform', '')) != platform_key(platform)]
    return held + entries


def ledger_refusals():
    """What the ledger's own map says about the tree, by running it.

    The artifact is written from the run's observations, and this is how the run
    finds out whether the ledger accepts it: the map reads the committed contract,
    the fingerprint the result records and every artifact it cites, and reports a
    refusal for each one that does not stand.
    """
    completed = subprocess.run(['cargo', 'run', '-q', '-p', 'symbiote-architecture',
                                '--example', 'decisions'], cwd=REPO, capture_output=True, text=True)
    refusals = [line for line in completed.stdout.splitlines() if line.startswith('refused ')]
    if completed.returncode != 0 and not refusals:
        raise SystemExit(f'the ledger could not read the tree: {completed.stderr.strip()}')
    return refusals


def publish(paths, publish_dir):
    """Copy the artifacts a run cites into the tree, hashing what was written.

    The directory is named from the repository root, because the paths a result
    artifact cites have to be repository-relative for the ledger to hash them. An
    artifact that recorded nothing is not published: citing an empty file would
    claim evidence where there is none.
    """
    published = []
    publish_dir.mkdir(parents=True, exist_ok=True)
    for path in paths:
        if not path.exists() or path.stat().st_size == 0:
            continue
        target = publish_dir / path.name
        shutil.copy2(path, target)
        published.append({'artifact': repo_path(target), 'sha256': sha256_of(target)})
    return published


def figures_of(samples, cleaned, stats, build_seconds, memory=True):
    """The predeclared measurements this run's instrument observed, and no others.

    A traced run reports the timing figure only: its client logs every protocol
    message, so its memory figures are not the ones the contract predeclares.
    """
    peak = max(samples, key=lambda row: row['sum_pss_kib'], default={})
    pss = peak.get('sum_pss_kib', 0)
    unattributed = peak.get('pss_by_class_kib', {}).get('unattributed', 0)
    figures = []
    if stats and stats['first_frame_seconds'] is not None:
        figures.append({'measurement': 'cold_start_to_first_frame_seconds',
                        'observed': stats['first_frame_seconds']})
    if memory:
        figures.append({'measurement': 'workload_process_tree_pss_mib',
                        'observed': round(pss / 1024, 3)})
        figures.append({'measurement': 'unattributed_process_tree_memory_percent',
                        'observed': round(100 * unattributed / max(pss, 1), 3)})
        if cleaned is not None:
            figures.append({'measurement': 'orphaned_processes_after_cancel',
                            'observed': float(cleaned['survivors_after_cancel_request'])})
            figures.append({'measurement': 'orphaned_listening_ports_after_cancel',
                            'observed': float(len(cleaned['listening_ports_after_cancel_request']))})
    if build_seconds is not None:
        figures.append({'measurement': 'clean_locked_build_seconds', 'observed': build_seconds})
    return figures, peak


def run_xvfb(args, artifacts, binary):
    server, env, log, session = start_xvfb(artifacts)
    try:
        code, elapsed, samples, seen, marks = observe_app(
            env, artifacts / 'app.log', binary,
            35 if args.interact else args.seconds, 0, trace=False)
        text = (artifacts / 'app.log').read_text()
        probes_denied = all(f'PROOF_PREVIEW_REPORT {command}: denied' in text
                            for command in ['snapshot', 'stop_ptys'])
        summary = {
            'platform': 'Linux Xvfb X11 only',
            'session': session,
            'app_exit': code,
            'elapsed_seconds': elapsed,
            'markers': marks,
            'preview_self_reported_denials': probes_denied,
            'max_sum_rss_kib': max((row['sum_rss_kib'] for row in samples), default=0),
            'max_sum_pss_kib': max((row['sum_pss_kib'] for row in samples), default=0),
            'rss_caveat': 'RSS sum double-counts shared pages; PSS is reported beside it; Xvfb is instrumentation outside the app tree',
            'observed_survivors_before_cleanup': survivors_of(seen, processes()),
            'samples': samples,
        }
        (artifacts / 'process-tree.json').write_text(json.dumps(summary, indent=2) + '\n')
        print(json.dumps({key: value for key, value in summary.items() if key != 'samples'}))
        print(artifacts)
        if code or not probes_denied or 'PROOF_READY' not in text:
            raise SystemExit('Proof failure: inspect artifacts')
    finally:
        print(json.dumps(cleanup(seen, [server])))
        log.close()


def build_record(args, revision):
    """The clean locked build these figures belong to, as the log recorded it.

    The log is captured with ``set -x`` and has to name the revision it built, on a
    line of its own — ``git rev-parse HEAD`` in the same recipe — because a log
    that names only the commands cannot be tied to the revision the run records.
    """
    log = Path(args.build_log)
    text = log.read_text()
    logged = logged_revision(text)
    if logged is None:
        raise SystemExit(f'refusing to publish: {log} records no revision it was built from, so '
                         'nothing ties the figures to a tree; log `git rev-parse HEAD` in the '
                         'same recipe')
    if not (revision.startswith(logged) or logged.startswith(revision)):
        raise SystemExit(f'refusing to publish: {log} was built from {logged} and the run '
                         f'records {revision}')
    record = {'seconds': args.build_seconds, 'revision': logged, 'log': repo_path(log),
              'log_sha256': sha256_of(log),
              'commands': [line.strip() for line in text.splitlines() if line.startswith('+ ')],
              'note': 'built from a fresh target directory outside this tree, logged with `set -x` so the log names the commands it timed'}
    if args.publish:
        # The build log is written outside the tree, where nothing durable holds
        # it: the copy the result cites is the record a later pass can re-hash.
        record['published_log'] = str(Path(args.publish) / publish_slug(args.platform) / log.name)
    return record


def unknown_of(args):
    """The measurements this instrument cannot see, and why, in declared order."""
    unknown = []
    for item in args.unobservable:
        name, _, reason = item.partition('=')
        unknown.append((name.strip(), reason.strip() or 'no reason recorded'))
    return unknown


def peak_note(samples, peak):
    """Where the memory figure's peak fell, so a reader is not left to guess.

    A peak early in a run is a startup peak, not a steady-state one, and the
    figure alone cannot say which; the sample it came from is data, so the run
    records it rather than describing it.
    """
    window = f"{samples[0]['seconds']} s to {samples[-1]['seconds']} s" if samples else 'no window'
    return (f"workload_process_tree_pss_mib: the peak of {len(samples)} samples fell "
            f"{peak.get('seconds')} s into the sampled window ({window}), so where it fell is "
            'recorded with the figure rather than left to the reader')


def result_problems(args, contract, runs):
    """Everything that would make this result claim more than the run shows.

    Checked before anything is published: a platform the contract does not apply
    to, a stop condition it does not declare, a measurement it predeclares that is
    neither observed nor named unknown, an unknown or an observation the contract
    never declared, and an obligation answered here with no marker in the run's
    own log.
    """
    problems = []
    if args.platform not in contract['applicable_platforms']:
        problems.append(f'{args.platform!r} is not a platform this contract applies to: '
                        + ', '.join(contract['applicable_platforms']))
    if args.stop_condition not in contract['stop_conditions']:
        problems.append(f'{args.stop_condition!r} is not a stop condition this contract declares')
    predeclared = [measurement['name'] for measurement in contract['measurements']]
    observed = {figure['measurement'] for row in runs for figure in row['figures']}
    unknown = dict(unknown_of(args))
    for name in sorted(set(unknown) - set(predeclared)):
        problems.append(f'{name} is named unobservable and the contract does not predeclare it')
    for name in sorted(observed - set(predeclared)):
        problems.append(f'{name} is observed and the contract does not predeclare it')
    for name in predeclared:
        if name not in observed and name not in unknown:
            problems.append(f'{name} is predeclared and neither observed nor named unobservable, '
                            'so this result would leave it silent')
    for row in runs:
        if not row['exercised']:
            problems.append('a run attests no obligation: its log carries no marker this fixture '
                            'emits for one')
    return problems


def result_runs(args, contract, runs, published, session, hardware, commit):
    """The result artifact's run entries, citing the artifacts that were published."""
    by_name = {item['artifact'].split('/')[-1]: item for item in published}
    unknown = unknown_of(args)
    failures = ([args.stop_condition] + [f'{name}: {reason}' for name, reason in unknown]
                + args.limitation + [note for row in runs for note in row.get('notes', [])])
    return [{
        'platform': args.platform,
        'version': session,
        'hardware': hardware,
        'commit': commit,
        'exercised': row['exercised'],
        'outcome': 'stop_condition_triggered',
        'stop_condition': args.stop_condition,
        'failures': failures,
        'observations': row['figures'],
        'artifacts': [by_name[name] for name in row['cites'] if name in by_name],
    } for row in runs]


def run_wayland(args, artifacts, binary, commit):
    contract = contract_of(REPO / args.contracts, args.contract)
    compositor, env, log, runtime, session = start_wayland(artifacts)
    log.close()
    runs = []
    try:
        # Run 1: the app's own lifetime, with its protocol log captured, so the
        # first-frame figure comes from the client's own clock. The clean locked
        # build of this candidate happened before it, on the same commit and
        # hardware, so run 1 records and cites it.
        code, elapsed, samples, seen, marks = observe_app(
            env, artifacts / 'app-1.log', binary, args.seconds, 0, trace=True)
        cleaned = cleanup(seen, [])
        stats = trace_stats(artifacts / 'app-1.log')
        figures, peak = figures_of(samples, None, stats, args.build_seconds, memory=False)
        (artifacts / 'process-tree-1.json').write_text(json.dumps(
            {'session': session, 'run': 1, 'app_exit': code, 'elapsed_seconds': elapsed,
             'markers': marks, 'trace': stats, 'peak_sample': peak, 'samples': samples,
             'cleaned_after_self_exit': cleaned}, indent=2) + '\n')
        runs.append({'run': 1, 'code': code, 'elapsed': elapsed, 'marks': marks, 'figures': figures,
                     'peak': peak, 'cancellation': cleaned,
                     'exercised': attested((artifacts / 'app-1.log').read_text()),
                     'cites': ['app-1.log', 'process-tree-1.json', 'session.json', 'cleanup.json',
                               'kwin.log'] + ([Path(args.build_log).name] if args.build_log else [])})

        # Run 2: the same workload, cancelled from under the driver, so the
        # cancellation figures are about a live tree rather than a reaped one.
        code, elapsed, samples, seen, marks = observe_app(
            env, artifacts / 'app-2.log', binary, 300, args.cancel_after, trace=False)
        cleaned = cleanup(seen, [])
        figures, peak = figures_of(samples, cleaned, None, None)
        (artifacts / 'process-tree-2.json').write_text(json.dumps(
            {'session': session, 'run': 2, 'app_exit': code, 'elapsed_seconds': elapsed,
             'markers': marks, 'cancellation': cleaned, 'peak_sample': peak,
             'samples': samples}, indent=2) + '\n')
        runs.append({'run': 2, 'code': code, 'elapsed': elapsed, 'marks': marks, 'figures': figures,
                     'peak': peak, 'cancellation': cleaned,
                     'exercised': attested((artifacts / 'app-2.log').read_text()),
                     'notes': [peak_note(samples, peak)],
                     'cites': ['app-2.log', 'process-tree-2.json', 'session.json', 'cleanup.json',
                               'kwin.log']})

        (artifacts / 'session.json').write_text(json.dumps(
            session_record(session, env, binary, runtime,
                           (artifacts / 'kwin.log').stat().st_size,
                           build_record(args, commit) if args.build_log else None), indent=2) + '\n')
        (artifacts / 'cleanup.json').write_text(json.dumps(
            {'run_1_after_self_exit': runs[0]['cancellation'],
             'run_2_after_cancel_request': runs[1]['cancellation'],
             'compositor_terminated_by': 'the driver, on the compositor process group it started'}, indent=2) + '\n')
        print(json.dumps([{key: value for key, value in row.items() if key != 'peak'} for row in runs],
                         indent=2))
        print(artifacts)
        if not args.results:
            return
        problems = result_problems(args, contract, runs)
        if problems:
            raise SystemExit('refusing to publish a result the run does not show:\n  '
                             + '\n  '.join(problems))
        sources = [artifacts / name for name in ('app-1.log', 'app-2.log', 'process-tree-1.json',
                                                 'process-tree-2.json', 'cleanup.json', 'session.json',
                                                 'kwin.log')]
        if args.build_log:
            sources.append(Path(args.build_log))
        published = publish(sources, REPO / args.publish / publish_slug(args.platform))
        entries = result_runs(args, contract, runs, published, session, hardware_of(env), commit)
        (artifacts / 'results-draft.json').write_text(json.dumps(entries, indent=2) + '\n')
        merged = merged_runs(REPO / args.results, contract, args.contract_sha256, args.platform, entries)
        (REPO / args.results).write_text(json.dumps({
            'schema_version': 1,
            'contract': contract['id'],
            'contract_sha256': args.contract_sha256,
            'untested_platforms': untested_platforms(contract, merged),
            'runs': merged,
        }, indent=2) + '\n')
        print(f'wrote {args.results}')
        refusals = ledger_refusals()
        if refusals:
            raise SystemExit('the ledger refuses the run this driver just published:\n  '
                             + '\n  '.join(refusals))
        print('the ledger reports no refusals for the tree this run wrote')
    finally:
        shutil.rmtree(runtime, ignore_errors=True)
        if compositor.poll() is None:
            try:
                os.killpg(compositor.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            compositor.wait(timeout=CANCEL_GRACE_SECONDS)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--session', choices=['xvfb', 'wayland'], default='xvfb',
                        help='the disposable session to run in (default: xvfb)')
    parser.add_argument('--interact', action='store_true',
                        help='X11 only: drive input and capture screenshots for inspection')
    parser.add_argument('--seconds', type=int, default=20,
                        help='seconds run 1 lets the app run before it exits on its own')
    parser.add_argument('--cancel-after', type=int, default=25,
                        help='Wayland: seconds into run 2 to request cancellation')
    parser.add_argument('--binary', default=None, help='the built app to run')
    parser.add_argument('--build-seconds', type=float, default=None,
                        help='a clean locked build of this candidate, measured separately')
    parser.add_argument('--build-log', default=None, help='where that build wrote its log')
    parser.add_argument('--commit', default=None, help='the revision the binary was built from')
    parser.add_argument('--platform', default='Linux Wayland on the reference compositor',
                        help='the contract platform this run exercised; the rest are derived as untested')
    parser.add_argument('--publish', default=None,
                        help='repository-relative root to publish the cited artifacts under, in a '
                             'directory per platform, so a second platform cannot overwrite the first')
    parser.add_argument('--results', default=None,
                        help='repository-relative result artifact to write')
    parser.add_argument('--contracts', default=CONTRACTS_PATH,
                        help='the committed contract document this run was measured against')
    parser.add_argument('--contract', default='#38/desktop-shell-representative-workload')
    parser.add_argument('--contract-sha256', default=None,
                        help='the fingerprint the contract document reaches; the ledger is what holds it')
    parser.add_argument('--stop-condition', default=None)
    parser.add_argument('--unobservable', nargs='*', default=[],
                        help='measurement=reason pairs this run reports unknown rather than met')
    parser.add_argument('--limitation', nargs='*', default=[],
                        help='what this instrument cannot show, recorded with every run')
    args = parser.parse_args()

    if args.interact and (not shutil.which('xdotool') or not shutil.which('import')):
        raise SystemExit('Interaction capture requires installed xdotool and ImageMagick; no packages are installed automatically')
    if os.environ.get('SYMBIOTE_PROOF_AUTHORIZED') != '1':
        raise SystemExit('Execution gate: root must establish #170/#173 prerequisites and set SYMBIOTE_PROOF_AUTHORIZED=1.')
    binary = Path(args.binary) if args.binary else next(
        (candidate for candidate in (ROOT / 'src-tauri/target/release/symbiote-linux-shell-proof',
                                     ROOT / 'src-tauri/target/debug/symbiote-linux-shell-proof')
         if candidate.exists()), None)
    if binary is None or not binary.exists():
        raise SystemExit('Build first: npm ci --ignore-scripts; npm run build; cargo build --locked -j 4 --manifest-path src-tauri/Cargo.toml')
    if args.results and args.publish is None:
        raise SystemExit('--results needs --publish: a result cites the artifacts it stands on')
    head = subprocess.run(['git', '-C', str(REPO), 'rev-parse', 'HEAD'],
                          capture_output=True, text=True).stdout.strip()
    commit = args.commit or head
    if args.results:
        status = subprocess.run(['git', '-C', str(REPO), 'status', '--porcelain'],
                                capture_output=True, text=True).stdout
        problems = revision_problems(commit, head, status)
        if problems:
            raise SystemExit('refusing to publish a result: a run records the revision it was '
                             'built from, and:\n  ' + '\n  '.join(problems))
    artifacts = ROOT / 'artifacts' / time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())
    artifacts.mkdir(parents=True)
    if args.session == 'xvfb':
        run_xvfb(args, artifacts, binary)
    else:
        run_wayland(args, artifacts, binary, commit)


if __name__ == '__main__':
    main()
