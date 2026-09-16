"""Where a run happens, and the one thing that closes it.

``Session`` owns the private session a run happens in whoever started it: ``close()``
is the only thing that kills its process group, removes whatever private runtime
directory it needed, and closes its log, so neither entry point can leave a session
behind. ``open_session`` picks the session an invocation asked for, and the two
starters below describe what each one is. ``SESSION_DISPLAY`` is the display server
each of them provides, which is the fact a result's platform claim is held to.
"""
from dataclasses import dataclass
import os
from pathlib import Path
import shutil
import signal
import subprocess
import time

SOCKET_NAME = 'symbiote-proof'
MESA_EGL_ICD = '/usr/share/glvnd/egl_vendor.d/50_mesa.json'
CANCEL_GRACE_SECONDS = 5
_RENDERERS = {}

# The display server each session this driver starts provides. A platform a result
# records is a claim about a machine, and this is one of the two facts about the
# session the driver can hold it to; the other is the operating system this process
# runs on, in ``records``, and ``session_record`` carries both beside the platform so
# a reader can check the rest. Which of the contract's platforms such a session stands
# for stays the operator's judgement — a virtual compositor standing for the reference
# compositor is one the documents state — because nothing here can know a machine it
# did not run on.
SESSION_DISPLAY = {'xvfb': 'X11', 'wayland': 'Wayland'}


@dataclass
class Session:
    """The private session a run happens in, whoever started it.

    One owner of the sequence both entry points share: a session is started where
    it is described below, and ``close()`` is the only thing that kills its process
    group, removes whatever private runtime directory it needed, and closes its
    log — so neither entry point can leave a session behind.
    """

    kind: str
    name: str
    process: subprocess.Popen
    env: dict
    log: object
    runtime: Path | None = None

    def close(self):
        if self.runtime is not None:
            shutil.rmtree(self.runtime, ignore_errors=True)
        if self.process.poll() is None:
            try:
                os.killpg(self.process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            self.process.wait(timeout=CANCEL_GRACE_SECONDS)
        self.log.close()


def open_session(args, artifacts):
    """The private session this invocation runs in: Xvfb X11, or kwin Wayland."""
    if args.session == 'xvfb':
        return start_xvfb(artifacts)
    return start_wayland(artifacts)


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
    return Session(kind='xvfb', name=f'Xvfb {display}', process=server, env=env, log=log)


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
    return Session(kind='wayland', name=f'{version} --virtual 1440x960, {renderer_of(env)}',
                   process=compositor, env=env, log=log, runtime=runtime)


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
