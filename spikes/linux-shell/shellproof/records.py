"""What a run records about itself, and the shape each version of each record carries.

``session_record`` is what a session was, so the figures carry their stack and its
compromises, and ``read_record`` is the one place a record is read: a record is held to
the shape the version it declares names rather than dated against prose. ``RECORD_SHAPES``
names the fields of every version of every record a run writes from its own observation —
the session, the cleanup record and the three process-tree records — and of the rows those
records carry. ``record_version`` is the version a writer declares; the entry points write
it beside the figures. ``hardware_of`` reads the machine the run happened on, ``host_system``
names the operating system the way a platform does, and ``sha256_of``/``repo_path`` say how
a record cites the bytes it kept.
"""
import hashlib
import os
from pathlib import Path
import subprocess

from shellproof.contract import REPO, read_document
from shellproof.session import SESSION_DISPLAY, renderer_of

# The operating systems a platform can name. A platform a result records is a claim
# about a machine, and this is the fact about the machine the driver can hold it to
# beside the display server ``session`` provides, so a platform naming another
# operating system than the one this process runs on is refused rather than recorded.
OPERATING_SYSTEMS = {'Linux': 'Linux', 'Darwin': 'macOS', 'macOS': 'macOS', 'Windows': 'Windows'}

# Paths this driver records and does not keep, dotted where they are nested. A
# reader following the provenance has to land on something that still exists, so
# each of these is recorded beside what outlives it — the binary beside the hash
# that identifies it, the build log beside the committed copy of the same bytes —
# and the one with nothing to reach, the session's private runtime directory, says
# in place that the driver removes it. ``test_run_proof.py`` holds both
# directions: a declared path says what replaces it, and a path the record cannot
# read back has to be declared.
UNKEPT_PATHS = ('runtime_dir', 'binary', 'build.log')

# The shape of every record a run writes from its own observation, one entry per kind
# and version: the fields that version carries. A kind is the record's own name — the file
# it is written as — so a reader finds the entry for the record it holds. Versions are per
# kind. Version 0 is the shape the writer wrote immediately before that record declared a
# version, which is what the committed result artifact holds, and is read rather than
# refused because a frozen artifact cannot be re-recorded to gain a field. Each writer
# declares ``record_version(kind)`` and a case holds the keys it writes to that entry, so a
# field added or removed without a version change fails there; ``read_record`` holds every
# record to the entry its declared version names, so the artifact and this table cannot
# drift.
#
# The dossier a run publishes is not a kind here, because its shape is the crate's rather
# than this driver's: ``Results`` in ``crates/symbiote-architecture/src/spike.rs`` reads it
# with ``deny_unknown_fields`` and refuses a version it does not read. The records this
# table declares are the ones nothing else reads, which is why their shape needs stating
# here: it has drifted in this tree before without anything declaring it — the X11 evidence
# under ``docs/proofs/evidence/linux-shell/`` holds eight fields in one process-tree record
# and twelve in another, against the eleven the writer wrote before this table, and two
# cleanup fields against the three it writes now. Those records predate the versioned
# writer and are cited evidence rather than read by it.
RECORD_SHAPES = {
    'session': {
        0: ('session', 'runtime_dir', 'runtime_dir_note', 'wayland_display', 'display_unset',
            'egl_vendor_icd_pinned', 'egl_icd_reason', 'compositor_log_bytes', 'compositor_log_note',
            'binary', 'binary_sha256', 'binary_note', 'rustc', 'cargo', 'hardware', 'build'),
        1: ('schema_version', 'display', 'system', 'session', 'runtime_dir', 'runtime_dir_note',
            'wayland_display', 'display_unset', 'egl_vendor_icd_pinned', 'egl_icd_reason',
            'compositor_log_bytes', 'compositor_log_note', 'binary', 'binary_sha256', 'binary_note',
            'rustc', 'cargo', 'hardware', 'build'),
    },
    'cleanup': {
        0: ('run_1_after_self_exit', 'run_2_after_cancel_request', 'compositor_terminated_by'),
        1: ('schema_version', 'run_1_after_self_exit', 'run_2_after_cancel_request',
            'compositor_terminated_by'),
    },
    'process-tree': {
        0: ('platform', 'session', 'app_exit', 'elapsed_seconds', 'markers',
            'preview_self_reported_denials', 'max_sum_rss_kib', 'max_sum_pss_kib', 'rss_caveat',
            'observed_survivors_before_cleanup', 'samples'),
        1: ('schema_version', 'platform', 'session', 'app_exit', 'elapsed_seconds', 'markers',
            'preview_self_reported_denials', 'max_sum_rss_kib', 'max_sum_pss_kib', 'rss_caveat',
            'observed_survivors_before_cleanup', 'samples'),
    },
    'process-tree-1': {
        0: ('session', 'run', 'app_exit', 'elapsed_seconds', 'markers', 'trace', 'peak_sample',
            'samples', 'cleaned_after_self_exit'),
        1: ('schema_version', 'session', 'run', 'app_exit', 'elapsed_seconds', 'markers', 'trace',
            'peak_sample', 'samples', 'cleaned_after_self_exit'),
    },
    'process-tree-2': {
        0: ('session', 'run', 'app_exit', 'elapsed_seconds', 'markers', 'cancellation',
            'peak_sample', 'samples'),
        1: ('schema_version', 'session', 'run', 'app_exit', 'elapsed_seconds', 'markers',
            'cancellation', 'peak_sample', 'samples'),
    },
}

# The shape of the rows the records above carry rather than write as files of their own:
# one sample of the process tree — the measurement the contract's method names, taken every
# 100 ms — and the cleanup record one run's cancellation leaves, which ``cleanup.json`` and
# both Wayland process-tree records embed. They declare no version of their own, because the
# record carrying them does; a case holds a run's own written rows to these fields, so a
# key added to a sample or to the cleanup record is a shape change like any other.
SAMPLE_FIELDS = ('seconds', 'members', 'sum_rss_kib', 'sum_pss_kib', 'pss_by_class_kib')
CANCELLATION_FIELDS = ('survivors_after_cancel_request', 'listening_ports_after_cancel_request',
                       'killed_observed_pids', 'remaining_observed_pids', 'unreaped_observed_pids')


def record_version(kind):
    """The version a writer declares for *kind*: the newest shape this table gives it."""
    return max(RECORD_SHAPES[kind])


def sha256_of(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


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


def host_system():
    """The operating system this process runs on, named the way a platform names it."""
    reported = os.uname().sysname
    return OPERATING_SYSTEMS.get(reported, reported)


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


def session_record(session, binary, compositor_log_bytes, build):
    """What this session was, so the figures carry their stack and its compromises.

    Two of the paths it records are ones the run does not keep: the session's
    private runtime directory, which the driver removes when the run ends, and the
    binary, which was built outside the tree and goes with the target directory it
    built into. Each says in place what a reader can still reach instead.

    The session is read here rather than passed field by field, so what a session
    was has one owner: ``Session``. A session with no private runtime directory is
    refused by name rather than recorded with a hole where its provenance should be:
    an Xvfb session has none, and this record is only written for the session kind
    that does.

    ``display`` and ``system`` are what the platform a result records is held to: the
    display server this session provides and the operating system it ran on, recorded
    beside the figures so a reader checks the label rather than trusting it. They are
    part of what ``RECORD_SHAPES['session']`` says version 1 added to the shape written
    before it.
    """
    if session.runtime is None:
        raise SystemExit(f'refusing to record {session.name!r}: it has no private runtime '
                         'directory, so it is not the session this record describes')
    env = session.env
    def tool(*command):
        try:
            return subprocess.run(command, capture_output=True, text=True,
                                  timeout=30).stdout.strip().splitlines()[0]
        except (OSError, IndexError, subprocess.SubprocessError):
            return 'not reported'
    return {
        'schema_version': record_version('session'),
        'session': session.name,
        'display': SESSION_DISPLAY[session.kind],
        'system': host_system(),
        'runtime_dir': repo_path(session.runtime),
        'runtime_dir_note': 'the private XDG_RUNTIME_DIR this session started in, removed when the run ends; no figure is read from it',
        'wayland_display': env.get('WAYLAND_DISPLAY'),
        'display_unset': 'DISPLAY' not in env,
        'egl_vendor_icd_pinned': env.get('__EGL_VENDOR_LIBRARY_FILENAMES'),
        'egl_icd_reason': 'the NVIDIA/glvnd path aborts the client against the virtual backend with wp_linux_drm_syncobj_surface_v1 explicit-sync errors, so the session runs on Mesa EGL and says so here',
        'compositor_log_bytes': compositor_log_bytes,
        'compositor_log_note': 'the compositor writes nothing to the stream this run captures, so the display log is empty and is not cited as evidence; the session it started is recorded here instead',
        'binary': repo_path(binary),
        'binary_sha256': sha256_of(binary),
        'binary_note': 'built into a fresh target directory outside this tree to measure a clean locked build, and not kept; binary_sha256 is the identity a later pass checks',
        'rustc': tool('rustc', '--version'),
        'cargo': tool('cargo', '--version'),
        'hardware': hardware_of(env),
        'build': build,
    }


def read_record(path, kind):
    """The *kind* record at *path*, held to the shape the version it declares names.

    A reader does not date the record's fields against prose: ``RECORD_SHAPES`` is the
    shape each version of each kind carries, and a record whose fields are not the ones its
    version names is refused — a key added or removed without a version change is a
    different shape, and saying so is what keeps the artifact and the table from drifting.
    A version this reader does not know is refused by name rather than guessed at, and a
    version that is not an integer of one of those versions is no version at all: ``1.0``
    is not ``1`` here, whatever a dict lookup would say. A record that declares no version
    is version 0, the shape written before this record carried a version — the committed
    run's records — and is read for the same reason a frozen artifact is not re-recorded to
    gain a field.
    """
    shapes = RECORD_SHAPES.get(kind)
    if shapes is None:
        raise SystemExit(f'{kind!r} is not a record this driver writes, so there is no shape '
                         f'to read it against: RECORD_SHAPES declares {sorted(RECORD_SHAPES)}')
    record = read_document(path, f'the {kind} record')
    if not isinstance(record, dict):
        raise SystemExit(f'the {kind} record {path} holds a {type(record).__name__}, not a record')
    declared = record.get('schema_version', 0)
    shape = shapes.get(declared) if type(declared) is int else None
    if shape is None:
        raise SystemExit(f'the {kind} record {path} declares schema version {declared!r}, and this '
                         f'reader reads {sorted(shapes)}: it refuses to read a shape it '
                         'does not know')
    if set(shape) - set(record):
        raise SystemExit(f'the {kind} record {path} declares version {declared} and does not carry '
                         f'{sorted(set(shape) - set(record))}, which that shape names')
    if set(record) - set(shape):
        raise SystemExit(f'the {kind} record {path} declares version {declared} and carries '
                         f'{sorted(set(record) - set(shape))}, which that shape does not name: a '
                         'shape change belongs in RECORD_SHAPES')
    return record
