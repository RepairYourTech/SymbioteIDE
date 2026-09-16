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

* the platform it names must be one the committed contract applies to and one the
  session it starts can be — it may not name a display server other than that
  session's own, nor another operating system than the machine it runs on — its stop
  condition must be one the contract declares and is recorded as the invocation's
  declaration among them, and every predeclared measurement must be either observed
  or named with the reason the instrument could not see it — a measurement that is
  simply absent is refused rather than left silent.
  These are the crate's own terms, applied here as well so a run the contract does
  not answer is refused before anything in the tree is written rather than after —
  the run's own records under the ignored ``artifacts/`` directory exist by then,
  and every path the repository commits is untouched; the ceilings are not among
  them, because a result records what was observed and the crate is what judges an
  observation against its maximum;
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

A program this driver runs and the host does not have — Xvfb, kwin_wayland, cargo,
git, the candidate binary — fails with the operating system's own message, which
names it; the driver adds no refusal of its own there, because it could not say more
than that message already says. Every document it reads is different: the contract
document, the dossier it merges into and the build log it cites are refused by name,
because nothing else in the failure would say which file was short.

The terms a run is judged against — the platforms the contract applies to, the stop
conditions it declares and the measurements it predeclares — are read from the
committed contract document through one reader (``Contract``), so the driver keeps
no copy of them: an invocation that names no contract and no platform is given the
document's own, and either term is still refused below if the contract does not
declare it. Two things the driver states rather than reads, and why nothing can read
them for it: where the document lives (the crate names the same path in Rust, and
nothing committed maps a contract id to its document) and which platform a run
takes when none is named (the document lists applicability as a set, so the first
platform it applies to is the order the document chose, not a term it declares).

The driver is a package, one concern per module, so each can be read and changed
alone. Where each concern lives:

* ``shellproof/contract.py`` — the terms a run is judged against, read from the
  document, and where the tree that holds it is;
* ``shellproof/session.py`` — where a run happens, and the one thing that closes a
  session, kills its process group and removes its runtime directory;
* ``shellproof/observation.py`` — the host and the process tree as they ran: one app
  lifetime, sampled, with nothing decided about it, and what was left of it;
* ``shellproof/measure.py`` — the sequence both entry points share: run it, sample it,
  clean up after it, read its own log;
* ``shellproof/records.py`` — what a run records about itself, and the shape each
  version of a session record carries, so a record is held to the version it declares
  rather than dated against prose;
* ``shellproof/checks.py`` — what a result may not claim, whether the platform a result
  records is one the session it starts can be, what the path an invocation selected
  would never read, and whether the value it supplies could be the contract's
  fingerprint;
* ``shellproof/publication.py`` — how this run's entries and the evidence they cite
  reach the tree, and which revision they belong to;
* this file — the terms an invocation left out, the two entry points and the gate: the
  concern a reader reaches for when the run itself is what changed.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time

# The fixture's own suite loads this file by path, so the package beside it is not on
# the import path as it is for `python3 run-proof.py`; this puts this file's own
# directory there so the concerns import either way a reader arrives.
if str(Path(__file__).resolve().parent) not in sys.path:
    sys.path.insert(0, str(Path(__file__).resolve().parent))

# The names this file publishes: what the entry points below use, plus the ones the
# fixture's own suite reads through this module, because `test_run_proof.py` loads this
# file rather than the package. Each is defined in the concern that owns it.
from shellproof.checks import (ATTESTED_BY, attested, fingerprint_problems, platform_problems,
                               result_problems, revision_problems, uncommitted,
                               unconsumed_options, untested_platforms)
from shellproof.contract import CONTRACTS_PATH, REPO, ROOT, Contract
from shellproof.measure import measure
from shellproof.observation import (EXITED_STATES, cleanup, processes, survivors_of, trace_stats,
                                    unreaped_of)
from shellproof.publication import (build_record, figures_of, ledger_refusals, logged_revision,
                                    merged_runs, peak_note, publish, publish_slug, result_runs)
from shellproof.records import (SESSION_SCHEMA_VERSION, SESSION_SHAPES, UNKEPT_PATHS, hardware_of,
                                host_system, read_session_record, session_record, sha256_of)
from shellproof.session import Session, open_session


def run_xvfb(args, artifacts, binary):
    """The X11 entry point: one measured lifetime, reported where the tool prints."""
    session = open_session(args, artifacts)
    measured = None
    try:
        measured = measure(session, artifacts, binary, 'app.log', args.seconds, 0, trace=False)
        probes_denied = all(f'PROOF_PREVIEW_REPORT {command}: denied' in measured['text']
                            for command in ['snapshot', 'stop_ptys'])
        summary = {
            'platform': 'Linux Xvfb X11 only',
            'session': session.name,
            'app_exit': measured['code'],
            'elapsed_seconds': measured['elapsed'],
            'markers': measured['marks'],
            'preview_self_reported_denials': probes_denied,
            'max_sum_rss_kib': max((row['sum_rss_kib'] for row in measured['samples']), default=0),
            'max_sum_pss_kib': max((row['sum_pss_kib'] for row in measured['samples']), default=0),
            'rss_caveat': 'RSS sum double-counts shared pages; PSS is reported beside it; Xvfb is instrumentation outside the app tree',
            'observed_survivors_before_cleanup': measured['survivors_before_cleanup'],
            'samples': measured['samples'],
        }
        (artifacts / 'process-tree.json').write_text(json.dumps(summary, indent=2) + '\n')
        print(json.dumps({key: value for key, value in summary.items() if key != 'samples'}))
        print(artifacts)
        if measured['code'] or not probes_denied or 'PROOF_READY' not in measured['text']:
            raise SystemExit('Proof failure: inspect artifacts')
    finally:
        session.close()
        if measured is not None:
            print(json.dumps(measured['cancellation']))


def run_wayland(args, artifacts, binary, commit):
    """The Wayland entry point: two measured lifetimes, then the dossier."""
    contract = Contract.read(REPO / args.contracts, args.contract)
    session = open_session(args, artifacts)
    runs = []
    try:
        # Run 1: the app's own lifetime, with its protocol log captured, so the
        # first-frame figure comes from the client's own clock. The clean locked
        # build of this candidate happened before it, on the same commit and
        # hardware, so run 1 records and cites it.
        first = measure(session, artifacts, binary, 'app-1.log', args.seconds, 0, trace=True)
        stats = trace_stats(first['log'])
        figures, peak = figures_of(first['samples'], None, stats, args.build_seconds, memory=False)
        (artifacts / 'process-tree-1.json').write_text(json.dumps(
            {'session': session.name, 'run': 1, 'app_exit': first['code'],
             'elapsed_seconds': first['elapsed'], 'markers': first['marks'], 'trace': stats,
             'peak_sample': peak, 'samples': first['samples'],
             'cleaned_after_self_exit': first['cancellation']}, indent=2) + '\n')
        runs.append({'run': 1, 'code': first['code'], 'elapsed': first['elapsed'],
                     'marks': first['marks'], 'figures': figures, 'peak': peak,
                     'cancellation': first['cancellation'], 'exercised': attested(first['text']),
                     'cites': ['app-1.log', 'process-tree-1.json', 'session.json', 'cleanup.json',
                               'kwin.log'] + ([Path(args.build_log).name] if args.build_log else [])})

        # Run 2: the same workload, cancelled from under the driver, so the
        # cancellation figures are about a live tree rather than a reaped one.
        second = measure(session, artifacts, binary, 'app-2.log', 300,
                         args.cancel_after if args.cancel_after is not None else 25, trace=False)
        figures, peak = figures_of(second['samples'], second['cancellation'], None, None)
        (artifacts / 'process-tree-2.json').write_text(json.dumps(
            {'session': session.name, 'run': 2, 'app_exit': second['code'],
             'elapsed_seconds': second['elapsed'], 'markers': second['marks'],
             'cancellation': second['cancellation'], 'peak_sample': peak,
             'samples': second['samples']}, indent=2) + '\n')
        runs.append({'run': 2, 'code': second['code'], 'elapsed': second['elapsed'],
                     'marks': second['marks'], 'figures': figures, 'peak': peak,
                     'cancellation': second['cancellation'], 'exercised': attested(second['text']),
                     'notes': [peak_note(second['samples'], peak)],
                     'cites': ['app-2.log', 'process-tree-2.json', 'session.json', 'cleanup.json',
                               'kwin.log']})

        (artifacts / 'session.json').write_text(json.dumps(
            session_record(session, binary, (artifacts / 'kwin.log').stat().st_size,
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
        entries = result_runs(args, contract, runs, published, session.name,
                              hardware_of(session.env), commit)
        merged = merged_runs(REPO / args.results, contract, args.contract_sha256, args.platform, entries)
        (REPO / args.results).write_text(json.dumps({
            'schema_version': 1,
            'contract': contract.id,
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
        session.close()


def named_defaults(args):
    """Fill in the contract terms an invocation left out, from the committed document.

    Nothing here can introduce a term: the document names its own contract and the
    platforms that contract applies to, and either value is refused below if the
    contract does not declare it. An invocation that names both is not second-guessed,
    so the document is read only when something is actually missing.
    """
    if args.contract is None or args.platform is None:
        named = Contract.named_by_the_document(args.contracts)
        args.contract = args.contract or named.id
        args.platform = args.platform or named.default_platform()
    return args


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--session', choices=['xvfb', 'wayland'], default='xvfb',
                        help='the disposable session to run in (default: xvfb)')
    parser.add_argument('--interact', action='store_true',
                        help='refused: this driver drives no input and captures no screenshot, so '
                             'there is nothing for this option to ask for')
    parser.add_argument('--seconds', type=int, default=20,
                        help='seconds run 1 lets the app run before it exits on its own')
    parser.add_argument('--cancel-after', type=int, default=None,
                        help='Wayland: seconds into run 2 to request cancellation (default: 25)')
    parser.add_argument('--binary', default=None, help='the built app to run')
    parser.add_argument('--build-seconds', type=float, default=None,
                        help='a clean locked build of this candidate, measured separately; needs '
                             '--build-log, which is what bears the figure')
    parser.add_argument('--build-log', default=None,
                        help='where that build wrote its log, which has to name the revision it built')
    parser.add_argument('--commit', default=None, help='the revision the binary was built from')
    parser.add_argument('--platform', default=None,
                        help='the contract platform this run exercised, which the contract has to '
                             'apply to and the session it starts has to be able to be (it may not '
                             'name another display server or operating system); the rest are '
                             'derived as untested (default: the first platform the contract '
                             'document applies to)')
    parser.add_argument('--publish', default=None,
                        help='repository-relative root to publish the cited artifacts under, in a '
                             'directory per platform, so a second platform cannot overwrite the first')
    parser.add_argument('--results', default=None,
                        help='repository-relative result artifact to write (a Wayland run; the X11 '
                             'entry point prints its figures and publishes none)')
    parser.add_argument('--contracts', default=CONTRACTS_PATH,
                        help='the committed contract document this run was measured against')
    parser.add_argument('--contract', default=None,
                        help='the contract in the committed document this run was measured '
                             'against (default: the document\'s own first contract)')
    parser.add_argument('--contract-sha256', default=None,
                        help="the contract's fingerprint, which the ledger holds: the crate's "
                             "SHA-256 of the contract's own canonical JSON, not the document "
                             "file's, and not this driver's to derive")
    parser.add_argument('--stop-condition', default=None,
                        help='the condition this invocation declares ended the run, which the '
                             'contract has to declare; each run entry records it as declared, '
                             'beside the exits, cleanup, unknowns and untested platforms a reader '
                             'compares it against')
    parser.add_argument('--unobservable', nargs='*', default=[],
                        help='measurement=reason pairs this run reports unknown rather than met')
    parser.add_argument('--limitation', nargs='*', default=[],
                        help='what this instrument cannot show, recorded with every run')
    args = parser.parse_args()
    problems = unconsumed_options(args)
    if problems:
        raise SystemExit('refusing to run: this invocation asks for something the path it selected '
                         'never reads, and:\n  ' + '\n  '.join(problems))
    args = named_defaults(args)

    if os.environ.get('SYMBIOTE_PROOF_AUTHORIZED') != '1':
        raise SystemExit('Execution gate: root must establish #170/#173 prerequisites and set SYMBIOTE_PROOF_AUTHORIZED=1.')
    binary = Path(args.binary) if args.binary else next(
        (candidate for candidate in (ROOT / 'src-tauri/target/release/symbiote-linux-shell-proof',
                                     ROOT / 'src-tauri/target/debug/symbiote-linux-shell-proof')
         if candidate.exists()), None)
    if binary is None or not binary.exists():
        raise SystemExit('Build first: npm ci --ignore-scripts; npm run build; cargo build --locked -j 4 --manifest-path src-tauri/Cargo.toml')
    if args.results and args.session == 'xvfb':
        raise SystemExit('--results needs --session wayland: the X11 entry point measures an Xvfb '
                         "session and records it as 'Linux Xvfb X11 only', which no contract in the "
                         'document applies to, so a result from it would measure none of the '
                         "contract's applicability; its figures are printed where the run happens")
    if args.results and args.publish is None:
        raise SystemExit('--results needs --publish: a result cites the artifacts it stands on')
    if args.results and args.contract_sha256 is None:
        raise SystemExit('--results needs --contract-sha256: a result records the contract it was '
                         'measured against, and the ledger is what re-hashes that fingerprint')
    if args.results:
        problems = fingerprint_problems(args.contract_sha256, REPO / args.contracts)
        if problems:
            raise SystemExit("refusing to publish a result: the fingerprint has to be the "
                             "contract's, and:\n  " + '\n  '.join(problems))
        problems = platform_problems(args.platform, args.session, host_system())
        if problems:
            raise SystemExit('refusing to publish a result: the platform it would record has to be '
                             'one this session can be, and:\n  ' + '\n  '.join(problems))
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
