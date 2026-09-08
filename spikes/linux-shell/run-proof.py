#!/usr/bin/env python3
"""Run only this experimental app in a disposable X11 server; no user display access."""
import json
import argparse
import os
from pathlib import Path
import shutil
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parent


def processes():
    result = {}
    for path in Path('/proc').glob('[0-9]*/status'):
        try:
            fields = dict(line.split(':', 1) for line in path.read_text().splitlines())
            pid = int(path.parent.name)
            stat = (path.parent / 'stat').read_text().rsplit(')', 1)[1].split()
            result[pid] = {'pid': pid, 'ppid': int(fields['PPid']), 'name': fields['Name'].strip(), 'start_ticks': int(stat[19]), 'rss_kib': int(fields.get('VmRSS', '0 kB').split()[0])}
        except (OSError, ValueError, KeyError):
            continue
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--interact', action='store_true', help='Capture input/project interactions in this private Xvfb; screenshots require inspection')
    args = parser.parse_args()
    if args.interact and (not shutil.which('xdotool') or not shutil.which('import')):
        raise SystemExit('Interaction capture requires installed xdotool and ImageMagick; no packages are installed automatically')
    if os.environ.get('SYMBIOTE_PROOF_AUTHORIZED') != '1':
        raise SystemExit('Execution gate: root must establish #170/#173 prerequisites and set SYMBIOTE_PROOF_AUTHORIZED=1.')
    binary = ROOT / 'src-tauri/target/debug/symbiote-linux-shell-proof'
    if not binary.exists():
        raise SystemExit('Build first: npm ci --ignore-scripts; npm run build; cargo build --locked -j 4 --manifest-path src-tauri/Cargo.toml')
    artifacts = ROOT / 'artifacts' / time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())
    artifacts.mkdir(parents=True)
    read_fd, write_fd = os.pipe()
    xvfb_log = (artifacts / 'xvfb.log').open('w')
    server = subprocess.Popen(['Xvfb', '-displayfd', str(write_fd), '-screen', '0', '1440x960x24', '-nolisten', 'tcp'], pass_fds=(write_fd,), stdout=xvfb_log, stderr=subprocess.STDOUT)
    os.close(write_fd)
    app = None
    seen = {}
    samples = []
    try:
        with os.fdopen(read_fd) as pipe:
            display = ':' + pipe.readline().strip()
        if display == ':':
            raise RuntimeError('Xvfb failed to allocate display')
        env = os.environ | {'DISPLAY': display, 'GDK_BACKEND': 'x11', 'SHELL_PROOF_SECONDS': '35' if args.interact else '20'}
        env.pop('WAYLAND_DISPLAY', None)
        # Keep WebKit sandbox enabled. A failure is evidence, not grounds to disable it.
        env.pop('WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS', None)
        with (artifacts / 'app.log').open('w') as log:
            started = time.monotonic()
            app = subprocess.Popen([str(binary)], env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            screenshot = False
            interactions = []
            while app.poll() is None and time.monotonic() - started < 45:
                table = processes()
                children = {app.pid}
                while True:
                    expanded = children | {pid for pid, p in table.items() if p['ppid'] in children}
                    if expanded == children:
                        break
                    children = expanded
                seen.update({pid: table[pid]['start_ticks'] for pid in children if pid in table})
                rows = [table[pid] for pid in sorted(children) if pid in table]
                samples.append({'seconds': round(time.monotonic() - started, 3), 'processes': rows, 'sum_rss_kib': sum(row['rss_kib'] for row in rows), 'xvfb_instrumentation_rss_kib': table.get(server.pid, {}).get('rss_kib', 0)})
                if args.interact and not interactions and time.monotonic() - started > 7:
                    windows = subprocess.check_output(['xdotool', 'search', '--onlyvisible', '--pid', str(app.pid), '--name', '^Symbiote experimental Linux shell$'], env=env, text=True, timeout=5).splitlines()
                    if len(windows) != 1:
                        raise RuntimeError(f'Expected exactly one proof window, got {len(windows)}')
                    window = windows[0]
                    def action(*arguments):
                        subprocess.run(['xdotool', *arguments], env=env, timeout=5, check=True)
                    def field(x, y, value):
                        action('mousemove', '--window', window, str(x), str(y), 'click', '1')
                        action('key', 'ctrl+a')
                        action('type', '--clearmodifiers', '--delay', '12', value)
                        time.sleep(.2)
                    def capture(name):
                        subprocess.run(['import', '-window', window, str(artifacts / name)], env=env, timeout=5, check=True)
                        interactions.append({'screenshot': name, 'result': 'commands delivered; inspect rendered values, not automatic assertion'})
                    # Coordinates come from the inspected 1400x900 fixed-layout
                    # proof screenshot. DISPLAY is this run's private Xvfb only.
                    field(960, 435, 'preview-focus-proof')
                    field(1170, 577, 'lead-focus-proof')
                    field(315, 598, 'alpha-project-proof')
                    capture('interaction-alpha.png')
                    action('mousemove', '--window', window, '1340', '40', 'click', '1')
                    action('key', 'End', 'Return')
                    time.sleep(.3)
                    field(315, 598, 'beta-project-proof')
                    capture('interaction-beta.png')
                    action('mousemove', '--window', window, '1340', '40', 'click', '1')
                    action('key', 'Home', 'Return')
                    time.sleep(.3)
                    capture('interaction-alpha-restored.png')
                    action('mousemove', '--window', window, '225', '671', 'click', '1')
                    action('mousemove', '--window', window, '505', '671', 'click', '1')
                    time.sleep(.5)
                    capture('interaction-ptys-stopped.png')
                if not screenshot and time.monotonic() - started > 8 and shutil.which('import'):
                    subprocess.run(['import', '-window', 'root', str(artifacts / 'x11.png')], env=env, timeout=5, check=True)
                    screenshot = True
                time.sleep(.5)
            if app.poll() is None:
                os.killpg(app.pid, signal.SIGTERM)
                app.wait(timeout=5)
            time.sleep(.5)
            live = processes()
            survivors = [live[pid] for pid in sorted(seen) if pid in live and live[pid]['start_ticks'] == seen[pid]]
            text = (artifacts / 'app.log').read_text()
            # These reports are untrusted Preview text, not authenticated denial evidence.
            probes_denied = all(f'PROOF_PREVIEW_REPORT {command}: denied' in text for command in ['snapshot', 'stop_ptys'])
            summary = {'platform': 'Linux Xvfb X11 only', 'display_dimensions': '1440x960', 'window_dimensions': '1400x900', 'app_exit': app.returncode, 'elapsed_seconds': round(time.monotonic() - started, 3), 'preview_probes_denied': probes_denied, 'max_sum_rss_kib': max((sample['sum_rss_kib'] for sample in samples), default=0), 'rss_caveat': 'RSS sum double-counts shared pages; not PSS or complete product measurement; Xvfb is instrumentation outside app tree', 'observed_survivors_before_cleanup': survivors, 'screenshot': screenshot, 'interaction_captures': interactions, 'samples': samples}
            summary['preview_self_reported_denials'] = summary.pop('preview_probes_denied')
            (artifacts / 'process-tree.json').write_text(json.dumps(summary, indent=2) + '\n')
            print(json.dumps({k: v for k, v in summary.items() if k != 'samples'}))
            print(artifacts)
            if app.returncode or survivors or not probes_denied or 'PROOF_READY' not in text:
                raise SystemExit('Proof failure: inspect artifacts')
    finally:
        if app is not None and app.poll() is None:
            os.killpg(app.pid, signal.SIGKILL)
            app.wait()
        # A crashed/exited parent may leave PTYs in separate sessions. Match observed
        # PID birth time before cleanup so unrelated reused process IDs are untouched.
        live = processes()
        cleaned = []
        for pid, start_ticks in seen.items():
            if pid in live and live[pid]['start_ticks'] == start_ticks:
                try:
                    os.kill(pid, signal.SIGKILL)
                    cleaned.append(pid)
                except ProcessLookupError:
                    pass
        time.sleep(.1)
        after = processes()
        remaining = [pid for pid, ticks in seen.items() if pid in after and after[pid]['start_ticks'] == ticks]
        (artifacts / 'cleanup.json').write_text(json.dumps({'killed_observed_pids': cleaned, 'remaining_observed_pids': remaining}, indent=2) + '\n')
        server.terminate()
        server.wait(timeout=5)
        xvfb_log.close()


if __name__ == '__main__':
    main()
