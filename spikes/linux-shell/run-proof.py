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
    try:
        output = subprocess.run(['eglinfo'], capture_output=True, text=True, env=env,
                                timeout=60).stdout
    except (OSError, subprocess.SubprocessError):
        return 'renderer not reported'
    for line in output.splitlines():
        if line.strip().startswith('OpenGL core profile renderer:'):
            return line.split(':', 1)[1].strip()
    return 'renderer not reported'


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


def session_record(session, env, binary, runtime):
    """What this session was, so the figures carry their stack and its compromises."""
    def tool(*command):
        try:
            return subprocess.run(command, capture_output=True, text=True,
                                  timeout=30).stdout.strip().splitlines()[0]
        except (OSError, IndexError, subprocess.SubprocessError):
            return 'not reported'
    cpu = next((line.split(':', 1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines()
                if line.startswith('model name')), 'unknown cpu')
    memory = next((line for line in Path('/proc/meminfo').read_text().splitlines()
                   if line.startswith('MemTotal')), 'MemTotal: unknown')
    return {
        'session': session,
        'runtime_dir': str(runtime),
        'wayland_display': env.get('WAYLAND_DISPLAY'),
        'display_unset': 'DISPLAY' not in env,
        'egl_vendor_icd_pinned': env.get('__EGL_VENDOR_LIBRARY_FILENAMES'),
        'egl_icd_reason': 'the NVIDIA/glvnd path aborts the client against the virtual backend with wp_linux_drm_syncobj_surface_v1 explicit-sync errors, so the session runs on Mesa EGL and says so here',
        'binary': str(binary.relative_to(REPO)),
        'binary_sha256': sha256_of(binary),
        'rustc': tool('rustc', '--version'),
        'cargo': tool('cargo', '--version'),
        'cpu': cpu,
        'memory': memory,
    }


def publish(paths, publish_dir):
    """Copy the artifacts a run cites into the tree, hashing what was written.

    The directory is named from the repository root, because the paths a result
    artifact cites have to be repository-relative for the ledger to hash them.
    """
    published = []
    publish_dir.mkdir(parents=True, exist_ok=True)
    for path in paths:
        if not path.exists():
            continue
        target = publish_dir / path.name
        shutil.copy2(path, target)
        published.append({'artifact': str(target.relative_to(REPO)), 'sha256': sha256_of(target)})
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
    if not memory:
        return figures, peak
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


def run_wayland(args, artifacts, binary, commit):
    compositor, env, log, runtime, session = start_wayland(artifacts)
    log.close()
    runs, published = [], []
    try:
        # Run 1: the app's own lifetime, with its protocol log captured, so the
        # readiness and first-frame figures come from the client's own clock.
        code, elapsed, samples, seen, marks = observe_app(
            env, artifacts / 'app-1.log', binary, args.seconds, 0, trace=True)
        cleaned = cleanup(seen, [])
        stats = trace_stats(artifacts / 'app-1.log')
        figures, peak = figures_of(samples, None, stats, None, memory=False)
        (artifacts / 'process-tree-1.json').write_text(json.dumps(
            {'session': session, 'run': 1, 'app_exit': code, 'elapsed_seconds': elapsed,
             'markers': marks, 'trace': stats, 'peak_sample': peak, 'samples': samples,
             'cleaned_after_self_exit': cleaned}, indent=2) + '\n')
        runs.append({'run': 1, 'code': code, 'elapsed': elapsed, 'marks': marks,
                     'figures': figures, 'peak': peak, 'cancellation': cleaned})

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
        runs.append({'run': 2, 'code': code, 'elapsed': elapsed, 'marks': marks,
                     'figures': figures, 'peak': peak, 'cancellation': cleaned})

        (artifacts / 'session.json').write_text(json.dumps(
            session_record(session, env, binary, runtime), indent=2) + '\n')
        (artifacts / 'cleanup.json').write_text(json.dumps(
            {'run_1_after_self_exit': runs[0]['cancellation'],
             'run_2_after_cancel_request': runs[1]['cancellation'],
             'compositor_terminated_by': 'the driver, on the compositor process group it started'}, indent=2) + '\n')
        sources = [artifacts / 'app-1.log', artifacts / 'app-2.log',
                   artifacts / 'process-tree-1.json', artifacts / 'process-tree-2.json',
                   artifacts / 'cleanup.json', artifacts / 'session.json', artifacts / 'kwin.log']
        if args.build_log:
            sources.append(Path(args.build_log))
        published = publish(sources, REPO / args.publish) if args.publish else []
        if args.build_seconds is not None and published:
            build_entry = next((item for item in published
                                if item['artifact'].endswith(Path(args.build_log).name)), None)
            runs.append({
                'run': 3, 'code': 0, 'elapsed': args.build_seconds, 'marks': {},
                'figures': [{'measurement': 'clean_locked_build_seconds',
                             'observed': args.build_seconds}],
                'peak': {'sum_pss_kib': 0}, 'cancellation': None,
                'artifacts': [build_entry] if build_entry else [],
                'session': 'clean locked release build with a warm toolchain, same host',
            })
        print(json.dumps([{key: value for key, value in row.items() if key != 'peak'}
                          for row in runs], indent=2))
        print(artifacts)
        if args.publish:
            if args.results:
                write_results(args, runs, published, artifacts, session, commit)
    finally:
        shutil.rmtree(runtime, ignore_errors=True)
        if compositor.poll() is None:
            try:
                os.killpg(compositor.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            compositor.wait(timeout=CANCEL_GRACE_SECONDS)


def write_results(args, runs, published, artifacts, session, commit):
    """The result artifact this contract points at, from what these runs observed."""
    by_name = {item['artifact'].split('/')[-1]: item for item in published}
    entries = []
    for row in runs:
        cited = []
        if row['run'] == 1:
            cited = [by_name.get('app-1.log'), by_name.get('process-tree-1.json'),
                     by_name.get('cleanup.json'), by_name.get('session.json'),
                     by_name.get('kwin.log')]
        elif row['run'] == 2:
            cited = [by_name.get('app-2.log'), by_name.get('process-tree-2.json'),
                     by_name.get('cleanup.json'), by_name.get('session.json'),
                     by_name.get('kwin.log')]
        else:
            cited = row.get('artifacts', [])
        failures = [args.stop_condition] + [
            f'{name}: {reason}' for name, reason in (item.split('=', 1) for item in args.unobservable)
        ] + args.limitation
        entries.append({
            'platform': args.platform,
            'version': row.get('session', session),
            'hardware': args.hardware,
            'commit': commit,
            'exercised': args.exercised,
            'outcome': 'stop_condition_triggered',
            'stop_condition': args.stop_condition,
            'failures': failures,
            'observations': row['figures'],
            'artifacts': [item for item in cited if item],
        })
    (artifacts / 'results-draft.json').write_text(json.dumps(entries, indent=2) + '\n')
    (REPO / args.results).write_text(json.dumps({
        'schema_version': 1,
        'contract': args.contract,
        'contract_sha256': args.contract_sha256,
        'untested_platforms': args.untested,
        'runs': entries,
    }, indent=2) + '\n')
    print(f'wrote {args.results}')


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
    parser.add_argument('--hardware', default=None, help='the machine the run happened on')
    parser.add_argument('--platform', default='Linux Wayland on the reference compositor')
    parser.add_argument('--untested', nargs='*', default=[],
                        help='contract platforms this run set left alone')
    parser.add_argument('--publish', default=None,
                        help='repository-relative directory to publish the cited artifacts into')
    parser.add_argument('--results', default=None,
                        help='repository-relative result artifact to write')
    parser.add_argument('--contract', default='#38/desktop-shell-representative-workload')
    parser.add_argument('--contract-sha256', default=None)
    parser.add_argument('--stop-condition', default=None)
    parser.add_argument('--unobservable', nargs='*', default=[],
                        help='measurement=reason pairs this run reports unknown rather than met')
    parser.add_argument('--limitation', nargs='*', default=[],
                        help='what this instrument cannot show, recorded with every run')
    parser.add_argument('--exercised', nargs='*', default=[])
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
    artifacts = ROOT / 'artifacts' / time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())
    artifacts.mkdir(parents=True)
    commit = args.commit or subprocess.run(['git', '-C', str(REPO), 'rev-parse', 'HEAD'],
                                           capture_output=True, text=True).stdout.strip()
    if args.session == 'xvfb':
        run_xvfb(args, artifacts, binary)
    else:
        run_wayland(args, artifacts, binary, commit)


if __name__ == '__main__':
    main()
