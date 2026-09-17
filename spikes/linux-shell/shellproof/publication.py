"""How this run's entries and the evidence they cite reach the tree.

``figures_of`` is the arithmetic that turns one measured lifetime into the
predeclared measurements it observed, ``publish`` copies and hashes the artifacts a
result stands on, ``merged_runs`` keeps another platform's runs exactly as they
stand, ``build_record`` is the clean locked build those figures belong to, and
``result_runs`` is the entries themselves. ``ledger_refusals`` is how the run finds
out whether the ledger accepts what it wrote.
"""
import shutil
import subprocess
from pathlib import Path

from shellproof.checks import unknown_of
from shellproof.contract import REPO, platform_key, read_document
from shellproof.observation import MARKS
from shellproof.records import repo_path, sha256_of


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
    document = read_document(path, 'the result dossier')
    if not isinstance(document, dict):
        raise SystemExit(f'refusing to merge: {path} holds a {type(document).__name__}, '
                         'not a dossier')
    if document.get('contract') != contract.id:
        raise SystemExit(f'refusing to merge: {path} holds {document.get("contract")!r}, '
                         f'not {contract.id!r}')
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


def figures_of(samples, cleaned, stats, build_seconds, memory=True, marks=None):
    """The predeclared measurements this run's instrument observed, and no others.

    A traced run reports the timing figure only: its client logs every protocol
    message, so its memory figures are not the ones the contract predeclares.
    Each name below is the computation that produces it — the contract predeclares
    the names, not the arithmetic — and ``result_problems`` refuses a name the
    contract does not predeclare before anything is written, so this copy cannot go
    stale silently. The marks the app's own log gave are read through ``observation``'s
    table, so the measurement a marker bears and the figure recorded for it cannot be
    stated in two places.
    """
    peak = max(samples, key=lambda row: row['sum_pss_kib'], default={})
    pss = peak.get('sum_pss_kib', 0)
    unattributed = peak.get('pss_by_class_kib', {}).get('unattributed', 0)
    figures = []
    if stats and stats['first_frame_seconds'] is not None:
        figures.append({'measurement': 'cold_start_to_first_frame_seconds',
                        'observed': stats['first_frame_seconds']})
    for marker, measurement in MARKS:
        if measurement and (marks or {}).get(marker) is not None:
            figures.append({'measurement': measurement, 'observed': marks[marker]})
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


def build_record(args, revision):
    """The clean locked build these figures belong to, as the log recorded it.

    The log is captured with ``set -x`` and has to name the revision it built, on a
    line of its own — ``git rev-parse HEAD`` in the same recipe — because a log
    that names only the commands cannot be tied to the revision the run records.

    A log given without ``--build-seconds`` is kept the same way and records
    ``seconds: null`` with no observation beside it, so that direction claims no
    figure and needs no rule; the figure without its log is the one that would.
    """
    log = Path(args.build_log)
    try:
        text = log.read_text()
    except OSError as error:
        raise SystemExit(f'refusing to publish: the build log {log} cannot be read: {error}')
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
        record['log_note'] = ('`log` is where the build wrote this outside the tree and goes with the '
                              'target directory it built into; `published_log` is the committed copy '
                              'of the same bytes, and is what log_sha256 re-hashes')
    else:
        record['log_note'] = ('`log` is outside this tree and this invocation asked for no published '
                              'copy, so the bytes it names are not kept')
    return record


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
        'stop_condition_note': ('the condition this invocation declares ended the run, among the ones '
                               'the contract carries: which of them a run trips is the operator\'s '
                               'statement, and the exits, cleanup, unknowns and untested platforms '
                               'in these entries are what a reader compares it against'),
        'failures': failures,
        'observations': row['figures'],
        'artifacts': [by_name[name] for name in row['cites'] if name in by_name],
    } for row in runs]
