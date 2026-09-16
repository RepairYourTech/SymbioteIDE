"""The runner's own rules: what a run may publish, and what it must refuse.

These hold the parts of the driver that decide what the result artifact is
allowed to claim — the terms it reads from the contract document, the attestation
read from the fixture's log, the contract coverage of its measurements, the
citations it publishes, the revision it records, the paths it does not keep and the
survivors it counts — and the one measured lifetime both entry points share, driven
here with a stand-in session and a stand-in app rather than a real display. The
rules matter because the artifact is the only thing a later pass reads: an
attestation that can be typed, a predeclared measurement left silent, an artifact
that recorded nothing cited as evidence, a revision that is not the tree, a path the
run removed published as provenance, a term answered from a copy of its own, or an
unreaped process counted as running, would each let the dossier claim more than the
run shows.
"""
import argparse
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

RUNNER = Path(__file__).resolve().parent / 'run-proof.py'
spec = importlib.util.spec_from_file_location('run_proof', RUNNER)
run_proof = importlib.util.module_from_spec(spec)
spec.loader.exec_module(run_proof)

# The names these cases read from the concern that owns each one. The entry point binds
# only what its own entry points read, so a rule is taken from its module rather than
# through `run_proof`; `run-proof.py`'s load above is what put this directory on the
# import path.
from shellproof.checks import ATTESTED_BY, uncommitted  # noqa: E402
from shellproof.observation import (EXITED_STATES, cleanup, processes, survivors_of,  # noqa: E402
                                    unreaped_of)
from shellproof.publication import logged_revision  # noqa: E402
from shellproof.records import (SESSION_SCHEMA_VERSION, SESSION_SHAPES, UNKEPT_PATHS,  # noqa: E402
                                read_session_record, sha256_of)
from shellproof.session import Session  # noqa: E402

# A contract of this suite's own, built as the driver reads the committed one. It
# is deliberately not the committed contract's terms: the driver has to answer
# with the contract it was handed, not with anything kept beside it.
CONTRACT = run_proof.Contract(
    id='#38/desktop-shell-representative-workload',
    applicable_platforms=('Linux Wayland on the reference compositor', 'Linux X11'),
    stop_conditions=('#38: an untested platform stays pending and keeps the choice from settling',),
    measurement_names=('cold_start_to_first_frame_seconds', 'worst_input_starvation_ms'))


def contract_document(identifier='SUITE/contract', platforms=('Suite Platform',),
                      measurements=('suite_measurement',)):
    """A contract document of this suite's own, shaped as the committed one is."""
    return {'contracts': [{'id': identifier, 'applicable_platforms': list(platforms),
                           'stop_conditions': ['suite: a thing ended the run'],
                           'measurements': [{'name': name, 'unit': 's', 'maximum': 1.0}
                                            for name in measurements]}]}


def args(**overrides):
    values = {'platform': 'Linux Wayland on the reference compositor',
              'stop_condition': CONTRACT.stop_conditions[0],
              'unobservable': ['worst_input_starvation_ms=the driver issues no input'],
              'limitation': []}
    values.update(overrides)
    return argparse.Namespace(**values)


def run(figures, exercised):
    """One run as the driver records it: observations are measurement/figure pairs."""
    return {'figures': [{'measurement': name, 'observed': 1.0} for name in figures],
            'exercised': exercised}


def declared_options(reads):
    """The options the parser declares as read only there, from its own rows."""
    return sorted(entry['option'] for entry in run_proof.build_parser()[1]
                  if entry['reads'] == reads)


class Attestation(unittest.TestCase):
    PTY = '#38: three active terminal tabs with bounded scrollback in real PTYs'
    PREVIEW = "#38: a live integrated Preview of the run's own output"
    AUTHORITY = ('#38: Preview origins, localhost included, that inherit neither workbench nor Host '
                 'authority and reach the app only through validated bridge operations')
    STREAMS = '#38: four concurrent agent streams, dispatched and supervised at once'

    def log(self, ptys, reports):
        return ''.join(f'PROOF_PTY_START id={index} pid=Some(1)\n' for index in range(ptys)) \
            + ''.join(f'PROOF_PREVIEW_REPORT {command}: denied\n' for command in ['snapshot'] * reports) \
            + 'PROOF_READY preview_origin=http://127.0.0.1:1 synthetic_streams=4 real_ptys=3\n'

    def test_an_obligation_is_attested_only_where_its_marker_is(self):
        complete = run_proof.attested(self.log(3, 2))
        self.assertIn(self.PTY, complete)
        self.assertIn(self.PREVIEW, complete)
        self.assertIn(self.AUTHORITY, complete)
        short = run_proof.attested(self.log(2, 2))
        self.assertNotIn(self.PTY, short)
        self.assertIn(self.PREVIEW, short)

    def test_a_fixture_marker_is_required_of_every_attested_obligation(self):
        thin = run_proof.attested(self.log(3, 1))
        self.assertNotIn(self.AUTHORITY, thin, 'two bridge reports are what the denial shows')
        self.assertEqual(run_proof.attested(''), [])

    def test_an_obligation_the_fixture_cannot_show_cannot_be_attested(self):
        for obligation, _, _ in ATTESTED_BY:
            self.assertNotEqual(obligation, self.STREAMS)
        text = self.log(3, 2) + 'synthetic_streams=4\n'
        self.assertNotIn(self.STREAMS, run_proof.attested(text),
                         'four synthetic streams are not four agent streams')


class ContractTerms(unittest.TestCase):
    """The terms a run is judged against come from the document, not from the driver."""

    def document(self, directory, **shaped):
        path = Path(directory) / 'spike-contracts.json'
        path.write_text(json.dumps(contract_document(**shaped)))
        return path

    def test_the_terms_are_the_documents_own(self):
        with tempfile.TemporaryDirectory() as directory:
            path = self.document(directory, identifier='SUITE/other', platforms=('A', 'B'),
                                 measurements=('m_one', 'm_two'))
            contract = run_proof.Contract.read(path, 'SUITE/other')
            self.assertTrue(contract.applies_to('B'))
            self.assertFalse(contract.applies_to('Suite Platform'))
            self.assertTrue(contract.declares('suite: a thing ended the run'))
            self.assertFalse(contract.declares('#38: something else ended it'))
            self.assertEqual(contract.measurement_names, ('m_one', 'm_two'))
            # A run answering this document's terms is accepted: the names judged are
            # the ones the document it was handed declares, not the committed one's.
            self.assertEqual(run_proof.result_problems(
                args(platform='A', stop_condition='suite: a thing ended the run',
                     unobservable=['m_one=what this document cannot see']),
                contract, [run(['m_two'], ['attested'])]), [])

    def test_a_document_that_holds_no_such_contract_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            path = self.document(directory, identifier='SUITE/other')
            with self.assertRaises(SystemExit):
                run_proof.Contract.read(path, '#38/desktop-shell-representative-workload')

    def test_a_document_that_names_no_contract_is_refused_by_name(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'spike-contracts.json'
            path.write_text(json.dumps({'contracts': []}))
            for refuse in (lambda: run_proof.Contract.read(path, 'anything'),
                           lambda: run_proof.Contract.read(path),
                           lambda: run_proof.Contract.named_by_the_document(path)):
                with self.assertRaises(SystemExit) as caught:
                    refuse()
                self.assertIn('names no contract', str(caught.exception))

    def test_a_document_that_cannot_be_read_is_refused_by_name(self):
        with tempfile.TemporaryDirectory() as directory:
            missing = Path(directory) / 'absent.json'
            with self.assertRaises(SystemExit) as caught:
                run_proof.Contract.read(missing, 'anything')
            self.assertIn('does not exist', str(caught.exception))
            broken = Path(directory) / 'broken.json'
            broken.write_text('{not json')
            with self.assertRaises(SystemExit) as caught:
                run_proof.Contract.named_by_the_document(broken)
            self.assertIn('cannot be read as JSON', str(caught.exception))

    def test_a_contract_without_the_terms_a_run_answers_is_refused_by_name(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'spike-contracts.json'
            for shaped, names in (
                    ({'contracts': [{'id': 'C', 'stop_conditions': ['s'],
                                     'measurements': [{'name': 'm'}]}]},
                     'the platforms it applies to'),
                    ({'contracts': [{'id': 'C', 'applicable_platforms': ['p'],
                                     'stop_conditions': ['s'], 'measurements': [{'unit': 's'}]}]},
                     'the names of the measurements it predeclares')):
                path.write_text(json.dumps(shaped))
                with self.assertRaises(SystemExit) as caught:
                    run_proof.Contract.read(path, 'C')
                self.assertIn(names, str(caught.exception))

    def test_an_invocation_that_names_no_terms_is_given_the_documents(self):
        with tempfile.TemporaryDirectory() as directory:
            path = self.document(directory, identifier='SUITE/other', platforms=('Suite First',
                                                                                 'Suite Second'))
            named = run_proof.named_defaults(argparse.Namespace(contracts=path, contract=None,
                                                                platform=None))
            self.assertEqual((named.contract, named.platform), ('SUITE/other', 'Suite First'))

    def test_an_invocation_that_names_them_keeps_its_own(self):
        named = run_proof.named_defaults(argparse.Namespace(contracts='no-such-document.json',
                                                            contract='X', platform='Y'))
        self.assertEqual((named.contract, named.platform), ('X', 'Y'),
                         'a document is read for a term the command line did not give')


class ResultRules(unittest.TestCase):
    def problems(self, **overrides):
        rows = overrides.pop('runs', [run(['cold_start_to_first_frame_seconds'], ['attested'])])
        return run_proof.result_problems(args(**overrides), CONTRACT, rows)

    def test_a_platform_the_contract_does_not_apply_to_is_refused(self):
        found = self.problems(platform='Linux Xvfb X11 only')
        self.assertTrue(any('Linux Xvfb X11 only' in problem for problem in found), found)

    def test_a_stop_condition_the_contract_does_not_declare_is_refused(self):
        found = self.problems(stop_condition='#38: something else ended it')
        self.assertTrue(any('not a stop condition' in problem for problem in found), found)

    def test_a_predeclared_measurement_left_silent_is_refused(self):
        found = self.problems(unobservable=[])
        self.assertTrue(any('worst_input_starvation_ms' in problem and 'silent' in problem
                            for problem in found), found)

    def test_a_measurement_the_contract_never_declared_is_refused(self):
        found = self.problems(runs=[run(['cold_start_to_first_frame_seconds', 'installer_size_mib'],
                                        ['attested'])])
        self.assertTrue(any('installer_size_mib' in problem for problem in found), found)
        found = self.problems(unobservable=['worst_input_starvation_ms=no input', 'idle_pss=no idle'])
        self.assertTrue(any('idle_pss' in problem for problem in found), found)

    def test_a_run_attesting_nothing_is_refused(self):
        found = self.problems(runs=[run(['cold_start_to_first_frame_seconds'], [])])
        self.assertTrue(any('attests no obligation' in problem for problem in found), found)

    def test_a_platform_this_session_cannot_be_is_refused(self):
        """The session bears the platform a result records; the invocation supplies the label."""
        for platform in ('macOS 14 arm64', 'Windows 11 x86_64', 'Linux X11'):
            with self.subTest(platform=platform):
                self.assertTrue(run_proof.platform_problems(platform, 'wayland', 'Linux'),
                                f'{platform} is not a platform a Wayland session on Linux can be')
        self.assertEqual(
            run_proof.platform_problems('Linux Wayland on the reference compositor', 'wayland', 'Linux'),
            [], 'the platform this session can be is still accepted')
        self.assertEqual(run_proof.platform_problems('Suite One', 'wayland', 'Linux'), [],
                         'a label naming neither fact is not contradicted from here')


class Publication(unittest.TestCase):
    def test_an_artifact_that_recorded_nothing_is_not_published(self):
        with tempfile.TemporaryDirectory() as temporary:
            source, target = Path(temporary) / 'run', Path(temporary) / 'publish'
            source.mkdir()
            (source / 'app.log').write_text('PROOF_READY\n')
            (source / 'kwin.log').write_text('')
            published = run_proof.publish([source / 'app.log', source / 'kwin.log', source / 'gone.log'],
                                          target)
            self.assertEqual([item['artifact'].split('/')[-1] for item in published], ['app.log'])

    def test_untracked_files_are_not_a_reason_to_refuse(self):
        status = '?? docs/proofs/results/desktop-shell.json\n?? .freebuff/\n'
        self.assertEqual(uncommitted(status), [])
        self.assertEqual(len(uncommitted(status + ' M spikes/linux-shell/run-proof.py\n')), 1)
        self.assertEqual(len(uncommitted('M  crates/symbiote-architecture/src/lib.rs\n')), 1)


FINGERPRINT = 'f' * 64
WAYLAND = 'Linux Wayland on the reference compositor'

# The fingerprint the ledger holds for the committed contract, taken from the result
# it accepted rather than computed here. The crate's fingerprint is
# `fingerprint(contract)`: the SHA-256 of the contract's own canonical JSON
# (`spike.rs`), which Python cannot reach without re-implementing the convention the
# driver deliberately does not. The committed result records exactly that value, and
# the crate's own suite is what refuses a result whose contract has moved since. A
# contract of this suite's own — the fixture above, or a synthetic document — has no
# ledger to be right about, so its cases carry a value of the right shape: what they
# hold is not the identity.
COMMITTED_RESULT = json.loads(
    (run_proof.REPO / 'docs/proofs/results/desktop-shell.json').read_text())
COMMITTED_FINGERPRINT = COMMITTED_RESULT['contract_sha256']
SYNTHETIC_FINGERPRINT = 'a' * 64


def dossier(runs, contract=CONTRACT.id, fingerprint=FINGERPRINT):
    return {'schema_version': 1, 'contract': contract, 'contract_sha256': fingerprint,
            'untested_platforms': ['Linux X11'], 'runs': runs}


# A stand-in for the candidate: it emits exactly the markers this fixture attests an
# obligation by, so a run built around it can support a result. One owner, because
# both the entry-point cases and the publishing cases run it.
MARKED_APP = ('#!/bin/sh\n'
              'echo PROOF_PTY_START\n'
              'echo PROOF_PTY_START\n'
              'echo PROOF_PTY_START\n'
              'echo "PROOF_READY preview_origin=http://127.0.0.1:5173"\n'
              'echo "PROOF_PREVIEW_REPORT snapshot: denied"\n'
              'echo "PROOF_PREVIEW_REPORT stop_ptys: denied"\n')


def committed_tree():
    """Every committed recorded file by its bytes: what a run must leave alone."""
    root = run_proof.REPO / 'docs/proofs/results'
    return {path: sha256_of(path) for path in sorted(root.rglob('*')) if path.is_file()}


# The revision gate reads the real git state, so the cases that publish need a tree
# with no modified tracked file. CI checks out one; a case skipped here is skipped for
# that reason and says so, rather than failing for a reason it does not hold.
TREE_IS_COMMITTED = not uncommitted(subprocess.run(
    ['git', '-C', str(run_proof.REPO), 'status', '--porcelain'],
    capture_output=True, text=True).stdout)


def entry(platform, observed):
    return {'platform': platform, 'version': 'v', 'hardware': 'h', 'commit': 'c' * 40,
            'exercised': ['an attested obligation'], 'outcome': 'stop_condition_triggered',
            'stop_condition': CONTRACT.stop_conditions[0], 'failures': ['what it did not see'],
            'observations': [{'measurement': 'cold_start_to_first_frame_seconds', 'observed': observed}],
            'artifacts': [{'artifact': 'docs/proofs/results/desktop-shell/app.log', 'sha256': 'a' * 64}]}


class Merge(unittest.TestCase):
    """A dossier holds one contract's runs, and this driver runs one platform."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.path = Path(self.directory.name) / 'desktop-shell.json'

    def test_a_second_platform_leaves_the_firsts_recorded_run_intact(self):
        wayland = entry(WAYLAND, 0.1)
        self.path.write_text(json.dumps(dossier([wayland])))
        x11 = entry('Linux X11', 0.2)
        merged = run_proof.merged_runs(self.path, CONTRACT, FINGERPRINT, 'Linux X11', [x11])
        self.assertEqual(merged[0], wayland, 'the first platform\'s run must survive a merge verbatim')
        self.assertEqual(merged[1], x11)
        self.assertEqual(run_proof.untested_platforms(CONTRACT, merged), [],
                         'both platforms are measured once both have a run')

    def test_a_platform_with_a_recorded_run_is_not_declared_untested(self):
        self.assertEqual(run_proof.untested_platforms(CONTRACT, [entry(WAYLAND, 0.1)]), ['Linux X11'])
        self.assertEqual(run_proof.untested_platforms(CONTRACT, []),
                         list(CONTRACT.applicable_platforms))

    def test_rerunning_a_platform_replaces_only_its_own_entries(self):
        wayland, old = entry(WAYLAND, 0.1), entry('Linux X11', 0.2)
        self.path.write_text(json.dumps(dossier([wayland, old])))
        fresh = entry('Linux X11', 0.3)
        merged = run_proof.merged_runs(self.path, CONTRACT, FINGERPRINT, 'Linux X11', [fresh])
        self.assertEqual([row for row in merged if row['platform'] == 'Linux X11'], [fresh])
        self.assertEqual([row for row in merged if row['platform'] != 'Linux X11'], [wayland])

    def test_a_dossier_that_cannot_be_read_is_refused_by_name(self):
        self.path.write_text('{not json')
        with self.assertRaises(SystemExit) as caught:
            run_proof.merged_runs(self.path, CONTRACT, FINGERPRINT, 'Linux X11', [])
        self.assertIn('cannot be read as JSON', str(caught.exception))
        self.path.write_text('[]')
        with self.assertRaises(SystemExit) as caught:
            run_proof.merged_runs(self.path, CONTRACT, FINGERPRINT, 'Linux X11', [])
        self.assertIn('not a dossier', str(caught.exception))

    def test_a_dossier_of_another_contract_or_other_thresholds_is_refused(self):
        self.path.write_text(json.dumps(dossier([], contract='SOME/OTHER')))
        with self.assertRaises(SystemExit):
            run_proof.merged_runs(self.path, CONTRACT, FINGERPRINT, 'Linux X11', [])
        self.path.write_text(json.dumps(dossier([], fingerprint='e' * 64)))
        with self.assertRaises(SystemExit):
            run_proof.merged_runs(self.path, CONTRACT, FINGERPRINT, 'Linux X11', [])

    def test_one_platforms_evidence_does_not_land_on_anothers(self):
        self.assertNotEqual(run_proof.publish_slug(WAYLAND), run_proof.publish_slug('Linux X11'))
        self.assertEqual(run_proof.publish_slug('Linux Wayland on the reference compositor'),
                         'linux-wayland-on-the-reference-compositor')


class Revision(unittest.TestCase):
    """The revision a run records has to be the tree it ran in and built from."""

    def test_a_clean_tree_at_the_recorded_revision_publishes(self):
        self.assertEqual(run_proof.revision_problems('a' * 40, 'a' * 40, '?? a-new-file\n'), [])

    def test_a_dirty_tree_or_another_revision_is_refused(self):
        self.assertTrue(run_proof.revision_problems('a' * 40, 'a' * 40, ' M spikes/linux-shell/run-proof.py\n'))
        self.assertTrue(run_proof.revision_problems('b' * 40, 'a' * 40, ''))

    def test_the_build_log_has_to_name_the_revision_it_built(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / 'build.log'
            log.write_text('+ npm run build\n   Compiling symbiote v0.1.0\n    Finished\n')
            self.assertIsNone(logged_revision(log.read_text()))
            arguments = argparse.Namespace(build_log=str(log), build_seconds=1.0, publish=None,
                                           platform='Linux X11')
            with self.assertRaises(SystemExit):
                run_proof.build_record(arguments, 'a' * 40)
            log.write_text('+ git rev-parse HEAD\n' + 'a' * 40 + '\n+ npm run build\n')
            record = run_proof.build_record(arguments, 'a' * 40)
            self.assertEqual(record['revision'], 'a' * 40)
            self.assertEqual(record['commands'], ['+ git rev-parse HEAD', '+ npm run build'])
            with self.assertRaises(SystemExit):
                run_proof.build_record(arguments, 'b' * 40)

    def test_a_build_log_that_cannot_be_read_is_refused_by_name(self):
        arguments = argparse.Namespace(build_log='/no/such/build.log', build_seconds=1.0,
                                       publish=None, platform='Linux X11')
        with self.assertRaises(SystemExit) as caught:
            run_proof.build_record(arguments, 'a' * 40)
        self.assertIn('/no/such/build.log', str(caught.exception))


class Survivors(unittest.TestCase):
    """A survivor is a process still running, not a /proc entry still listed."""

    def row(self, state, ticks=42):
        return {7: {'pid': 7, 'name': 'app', 'ppid': 1, 'start_ticks': ticks, 'state': state}}

    def test_a_process_that_has_exited_is_not_a_survivor(self):
        for state in 'SR':
            self.assertEqual(survivors_of({7: 42}, self.row(state)), [7])
        for state in EXITED_STATES:
            self.assertEqual(survivors_of({7: 42}, self.row(state)), [],
                             f'a process in state {state} has exited and runs nothing')
            self.assertEqual(unreaped_of({7: 42}, self.row(state)), [7],
                             'and is named rather than lost')

    def test_a_pid_the_kernel_reused_is_not_the_process_this_run_saw(self):
        self.assertEqual(survivors_of({7: 42}, self.row('S', ticks=43)), [])
        self.assertEqual(unreaped_of({7: 42}, self.row('S', ticks=43)), [])

    @unittest.skipUnless(hasattr(os, 'fork'), 'needs a /proc to ask')
    def test_a_child_never_reaped_is_not_counted_as_running(self):
        """The same question put to the kernel rather than to a fixture."""
        pid = os.fork()
        if pid == 0:
            os._exit(0)
        try:
            deadline = time.monotonic() + 10
            table = processes()
            while time.monotonic() < deadline and table.get(pid, {}).get('state') != 'Z':
                time.sleep(0.01)
                table = processes()
            self.assertEqual(table.get(pid, {}).get('state'), 'Z',
                             'the child exited and this process never reaped it')
            seen = {pid: table[pid]['start_ticks']}
            self.assertEqual(survivors_of(seen, table), [])
            self.assertEqual(unreaped_of(seen, table), [pid])
        finally:
            os.waitpid(pid, 0)

    @unittest.skipUnless(hasattr(os, 'fork'), 'needs a /proc to ask')
    def test_the_cleanup_record_counts_what_runs_and_names_what_it_left(self):
        """The figure the artifact publishes, asked of a real unreaped child."""
        pid = os.fork()
        if pid == 0:
            os._exit(0)
        try:
            table, deadline = None, time.monotonic() + 10
            while time.monotonic() < deadline:
                table = processes()
                if table.get(pid, {}).get('state') == 'Z':
                    break
                time.sleep(0.01)
            record = cleanup({pid: table[pid]['start_ticks']}, [])
            self.assertEqual(record['survivors_after_cancel_request'], 0,
                             'nothing is running, however many entries /proc lists')
            self.assertEqual(record['unreaped_observed_pids'], [pid])
            self.assertEqual(record['remaining_observed_pids'], [])
            self.assertEqual(record['listening_ports_after_cancel_request'], [])
        finally:
            os.waitpid(pid, 0)


class Ended:
    """The process of a session this suite stands in: it has already ended."""

    def poll(self):
        return 0

    def wait(self, timeout=None):
        return 0


class Closes:
    """A session log that counts being closed, so a session left open is visible."""

    def __init__(self):
        self.closes = 0

    def close(self):
        self.closes += 1


class RecordedSession(Session):
    """A session that counts being closed: one nobody closes was never cleaned up."""

    def __init__(self, **fields):
        super().__init__(**fields)
        self.closes = 0

    def close(self):
        self.closes += 1
        super().close()


class EntryPoint(unittest.TestCase):
    """Both entry points run one measured lifetime through one place, and close it.

    The X11 entry point is driven with a stand-in session and a stand-in app, because
    what the shared sequence has to preserve is what surrounds the app — run it,
    sample it, clean up after it, read its own log, close the session — not the
    display it happens to run on.
    """

    APP = ('#!/bin/sh\n'
           'echo PROOF_READY\n'
           'echo "PROOF_PREVIEW_REPORT snapshot: denied"\n'
           'echo "PROOF_PREVIEW_REPORT stop_ptys: denied"\n')

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.artifacts = Path(self.directory.name) / 'artifacts'
        self.artifacts.mkdir()
        self.app = self.artifacts / 'probe'
        self.app.write_text(self.APP)
        self.app.chmod(0o755)
        self.session = RecordedSession(kind='suite', name='suite session', process=Ended(),
                                       env=dict(os.environ), log=Closes(), runtime=None)

    def test_the_entry_point_runs_the_lifetime_and_leaves_no_session_behind(self):
        printed = io.StringIO()
        with mock.patch.object(run_proof, 'open_session', return_value=self.session):
            with contextlib.redirect_stdout(printed):
                run_proof.run_xvfb(argparse.Namespace(interact=False, seconds=1), self.artifacts,
                                   self.app)
        self.assertEqual(self.session.closes, 1, 'the session it opened has to be closed')
        record = json.loads((self.artifacts / 'process-tree.json').read_text())
        self.assertEqual(record['session'], 'suite session')
        self.assertEqual(record['app_exit'], 0)
        self.assertTrue(record['preview_self_reported_denials'])
        self.assertEqual(json.loads(printed.getvalue().splitlines()[-1])
                         ['survivors_after_cancel_request'], 0)

    @unittest.skipUnless(Path('/bin/sh').exists(), 'needs a shell for the stand-in app')
    def test_the_shared_lifetime_reports_what_an_entry_point_consumes(self):
        # The stand-in app writes its own markers, so the log this returns is not
        # an empty file that any implementation would satisfy.
        measured = run_proof.measure(self.session, self.artifacts, self.app, 'app.log', 1, 0,
                                     trace=False)
        self.assertEqual((measured['log_name'], measured['code']), ('app.log', 0))
        self.assertEqual(measured['text'], (self.artifacts / 'app.log').read_text())
        self.assertIn('PROOF_READY', measured['text'])
        self.assertEqual(measured['survivors_before_cleanup'], [])
        self.assertEqual(measured['cancellation']['survivors_after_cancel_request'], 0)


class Publishing(unittest.TestCase):
    """Nothing is published for a run that cannot support its own result.

    The Wayland entry point is driven with a stand-in session and a stand-in app, and
    every path it writes is inside this case's own temporary directory, so what a
    refusal leaves behind — and what a publishing run does write — can be counted
    without reaching the tree the repository commits: that tree is read before and
    after each drive, by bytes, and has to be identical.

    The ledger's own read-back of what was published is stood in for here, because
    it runs cargo and is not what these cases are about; the artifact it would read
    is checked by ``results`` below and by the commits that carry it.
    """

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.artifacts = self.root / 'artifacts'
        self.artifacts.mkdir()
        (self.artifacts / 'kwin.log').write_text('')
        self.session = RecordedSession(kind='wayland', name='suite compositor',
                                       process=Ended(), env=dict(os.environ), log=Closes(),
                                       runtime=self.root / 'runtime')
        self.publish = self.root / 'publish'
        self.dossier = self.root / 'desktop-shell.json'
        contract = run_proof.Contract.read(run_proof.REPO / run_proof.CONTRACTS_PATH)
        self.arguments = argparse.Namespace(
            seconds=1, cancel_after=0, build_seconds=None, build_log=None,
            publish=str(self.publish), results=str(self.dossier),
            platform=contract.default_platform(), contracts=run_proof.CONTRACTS_PATH,
            contract=contract.id, contract_sha256='f' * 64,
            stop_condition=contract.stop_conditions[0], limitation=[],
            unobservable=[f'{name}=this case measures nothing'
                          for name in contract.measurement_names])

    def drive(self, markers):
        before = committed_tree()
        app = self.artifacts / 'probe'
        app.write_text(markers)
        app.chmod(0o755)
        printed = io.StringIO()
        with mock.patch.object(run_proof, 'open_session', return_value=self.session), \
             mock.patch.object(run_proof, 'ledger_refusals', return_value=[]):
            with contextlib.redirect_stdout(printed):
                run_proof.run_wayland(self.arguments, self.artifacts, app, 'a' * 40)
        self.assertEqual(committed_tree(), before,
                         'a run must not touch a file the repository commits')
        return printed.getvalue()

    def test_a_run_that_cannot_attest_what_it_ran_publishes_nothing(self):
        with self.assertRaises(SystemExit) as caught:
            self.drive('#!/bin/sh\necho "no marker this fixture attests"\n')
        self.assertIn('attests no obligation', str(caught.exception))
        self.assertFalse(self.publish.exists(), 'an artifact directory was published')
        self.assertFalse(self.dossier.exists(), 'a dossier was written')

    def test_a_run_that_does_not_answer_the_contract_publishes_nothing(self):
        self.arguments.unobservable = ['not_predeclared=this case measures nothing']
        with self.assertRaises(SystemExit) as caught:
            self.drive(MARKED_APP)
        self.assertIn('not_predeclared', str(caught.exception))
        self.assertFalse(self.publish.exists(), 'an artifact directory was published')
        self.assertFalse(self.dossier.exists(), 'a dossier was written')

    def test_a_run_that_supports_its_result_publishes_it(self):
        printed = self.drive(MARKED_APP)
        self.assertIn(f'wrote {self.dossier}', printed)
        document = json.loads(self.dossier.read_text())
        self.assertEqual(document['contract'], self.arguments.contract)
        self.assertEqual(document['untested_platforms'],
                         ['Linux X11', 'Windows 11 x86_64', 'macOS 14 arm64'])
        self.assertEqual(len(document['runs']), 2)
        published = [item for row in document['runs'] for item in row['artifacts']]
        self.assertTrue(published, 'a published run cites the artifacts it stands on')
        for item in published:
            path = run_proof.REPO / item['artifact']
            self.assertTrue(path.exists(), f"{item['artifact']} is cited and not published")
            self.assertEqual(sha256_of(path), item['sha256'])


class MainEntryPoint(unittest.TestCase):
    """The driver's own entry point, driven the way the command line drives it.

    This is the region that exists only in production: the terms an invocation left
    out, both gates, and what a run writes when it publishes. ``main`` is called as
    the command line calls it — through the parser and the environment — with two
    stand-ins and only two: the session it would start, and the ledger's cargo
    read-back, which the fixture job has no Rust toolchain to run. What the terms are
    and what a run may write is not stood in for anywhere. The fingerprint these cases
    supply is the one the ledger holds for the committed contract, taken from the result
    it accepted, because the crate's fingerprint is the SHA-256 of the contract's own
    canonical JSON and Python cannot compute it without re-implementing that convention.

    The revision gate reads the real git state, so a checkout with edits cannot
    publish; CI checks out a clean tree and drives it unpatched. Where a tracked file
    is modified — a working checkout, or a proof that mutates this file on purpose —
    the status the gate reads is stood in for and the case says so, because the gate
    itself is not what these cases hold.
    """

    SYNTHETIC = {'contracts': [{'id': 'SUITE/other-contract',
                               'applicable_platforms': ['Suite One', 'Suite Two'],
                               'stop_conditions': ['suite: a thing ended the run'],
                               'measurements': [{'name': name, 'unit': 's', 'maximum': 1.0}
                                                for name in (
                                                    'workload_process_tree_pss_mib',
                                                    'unattributed_process_tree_memory_percent',
                                                    'orphaned_processes_after_cancel',
                                                    'orphaned_listening_ports_after_cancel',
                                                    'suite_unseen_measurement')]}]}

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.app = self.root / 'probe'
        self.app.write_text(MARKED_APP)
        self.app.chmod(0o755)
        self.publish = self.root / 'publish'
        self.dossier = self.root / 'desktop-shell.json'
        self.contract = run_proof.Contract.read(run_proof.REPO / run_proof.CONTRACTS_PATH)
        self.runtime = self.root / 'runtime'
        self.session = RecordedSession(kind='wayland', name='suite compositor', process=Ended(),
                                       env=dict(os.environ), log=Closes(), runtime=self.runtime)

    def start(self, arguments, artifacts):
        """The session the driver starts, with the files the real one would leave.

        ``start_wayland`` opens ``kwin.log`` for the compositor and creates its private
        runtime directory, and ``start_xvfb`` opens ``xvfb.log``; this stands in for the
        compositor itself, not for those, so it creates them where the real ones do
        before returning.
        """
        (artifacts / ('xvfb.log' if arguments.session == 'xvfb' else 'kwin.log')).write_text('')
        if arguments.session == 'wayland':
            self.runtime.mkdir(mode=0o700, exist_ok=True)
        return self.session

    def arguments(self, *extra, results=True, session='wayland', contract=None,
                  unobservable=None, stop_condition=None, fingerprint=COMMITTED_FINGERPRINT):
        """The command line this case drives, naming no contract and no platform.

        Those two terms are the ones an invocation may leave out; a case that wants
        different terms names them through ``extra``, exactly as an operator would.
        The fingerprint is the ledger's own for the committed contract unless a case is
        about it.
        """
        contract = contract or self.contract
        # The line an operator would give: the session's own options, then the ones a
        # Wayland result run carries. Which of them the selected path reads is not
        # restated here — the parser declares each option's row and the cases below drive
        # that rule one option at a time.
        argv = ['--session', session, '--binary', str(self.app), '--seconds', '1']
        if session == 'wayland':
            argv += ['--cancel-after', '0']
        if results:
            unobservable = unobservable or [f'{name}=this case measures nothing'
                                            for name in contract.measurement_names]
            argv += ['--stop-condition', stop_condition or contract.stop_conditions[0],
                     '--unobservable', *unobservable,
                     '--publish', str(self.publish), '--results', str(self.dossier),
                     '--contract-sha256', fingerprint]
        return argv + list(extra)

    def clean_status(self):
        """Stand in for the git status the revision gate reads, on a dirty checkout.

        A status that names a modified tracked file is the gate doing its job, and a
        case that mutates this file to prove a rule would otherwise only ever see
        that refusal. On a clean tree — CI, and this repository when it is committed —
        nothing is stood in for and the gate reads the tree itself.
        """
        if TREE_IS_COMMITTED:
            return contextlib.nullcontext()
        real = subprocess.run

        def run(command, *args, **kwargs):
            if 'status' in command and '--porcelain' in command:
                return subprocess.CompletedProcess(command, 0, stdout='', stderr='')
            return real(command, *args, **kwargs)
        return mock.patch.object(run_proof.subprocess, 'run', side_effect=run)

    def drive(self, argv, authorized=True):
        before = committed_tree()
        environment = dict(os.environ)
        environment.pop('SYMBIOTE_PROOF_AUTHORIZED', None)
        if authorized:
            environment['SYMBIOTE_PROOF_AUTHORIZED'] = '1'
        printed = io.StringIO()
        with mock.patch.object(sys, 'argv', ['run-proof.py'] + argv), \
             mock.patch.dict(os.environ, environment, clear=True), \
             mock.patch.object(run_proof, 'ROOT', self.root), \
             mock.patch.object(run_proof, 'open_session', side_effect=self.start), \
             mock.patch.object(run_proof, 'ledger_refusals', return_value=[]), \
             self.clean_status():
            with contextlib.redirect_stdout(printed):
                run_proof.main()
        self.assertEqual(committed_tree(), before,
                         'a run through the entry point must not touch a committed file')
        return printed.getvalue()

    def records(self):
        """What the run wrote under the artifacts root it was given, by file name."""
        return sorted(path.name for path in (self.root / 'artifacts').rglob('*') if path.is_file())

    def test_the_gate_variable_is_refused_before_anything_is_written(self):
        with self.assertRaises(SystemExit) as caught:
            self.drive(self.arguments(), authorized=False)
        self.assertIn('SYMBIOTE_PROOF_AUTHORIZED', str(caught.exception))
        self.assertEqual(self.records(), [], 'a run started without the gate')
        self.assertFalse(self.publish.exists())
        self.assertFalse(self.dossier.exists())

    def test_a_result_without_the_contract_fingerprint_is_refused(self):
        """A result records the contract it was measured against, so it has to name it.

        The fingerprint is not this driver's to derive — the ledger re-hashes the
        committed document and holds the answer — so a run that was told nothing
        records nothing the ledger can read, which is a refusal now rather than a
        dossier nothing can open.
        """
        argv = self.arguments()
        fingerprint = argv.index('--contract-sha256')
        with self.assertRaises(SystemExit) as caught:
            self.drive(argv[:fingerprint] + argv[fingerprint + 2:])
        self.assertIn('contract-sha256', str(caught.exception))
        self.assertEqual(self.records(), [], 'a run started with no fingerprint to record')
        self.assertFalse(self.publish.exists())
        self.assertFalse(self.dossier.exists())

    def test_the_committed_fingerprint_is_the_ledgers_and_not_the_documents(self):
        """Two quantities of the same length, and the cases carry the ledger's.

        The crate's fingerprint is the SHA-256 of the contract's own canonical JSON; the
        document file's SHA-256 is a different quantity, and a run recording it is
        reported by the ledger as measured against a contract that is not the one
        committed now. Python can compute the second and not the first, so the result the
        ledger accepted is where this suite takes the first from.
        """
        self.assertEqual(COMMITTED_RESULT['contract'], self.contract.id,
                         'the committed result is of another contract')
        self.assertEqual(COMMITTED_FINGERPRINT, COMMITTED_RESULT['contract_sha256'])
        self.assertNotEqual(COMMITTED_FINGERPRINT,
                            sha256_of(run_proof.REPO / run_proof.CONTRACTS_PATH),
                            'the fingerprint is the document file\'s hash, not the contract\'s')
        argv = self.arguments()
        supplied = argv[argv.index('--contract-sha256') + 1]
        self.assertEqual(supplied, COMMITTED_FINGERPRINT,
                         'the cases drive with a value that is not the ledger\'s')

    def test_a_fingerprint_that_is_the_documents_hash_is_refused(self):
        """The value an operator reaches for, refused by name before anything is written."""
        document = sha256_of(run_proof.REPO / run_proof.CONTRACTS_PATH)
        with self.assertRaises(SystemExit) as caught:
            self.drive(self.arguments(fingerprint=document))
        self.assertIn('the document file', str(caught.exception))
        self.assertEqual(self.records(), [], 'a run started with the document\'s hash as a fingerprint')
        self.assertFalse(self.publish.exists())
        self.assertFalse(self.dossier.exists())

    def test_a_value_that_is_not_a_fingerprint_is_refused(self):
        with self.assertRaises(SystemExit) as caught:
            self.drive(self.arguments(fingerprint=''))
        self.assertIn('is not a fingerprint', str(caught.exception))
        self.assertEqual(self.records(), [], 'a run started with no fingerprint to record')
        self.assertFalse(self.publish.exists())
        self.assertFalse(self.dossier.exists())

    def test_an_x11_run_that_asks_for_a_result_is_refused(self):
        """The X11 entry point measures a session no contract applies to, so it never publishes.

        The path prints its own summary and writes its own ignored records; what it cannot do
        is contribute a run to a contract, and an invocation that asks for a result artifact
        used to get silence and exit 0 instead of being told that.
        """
        with self.assertRaises(SystemExit) as caught:
            self.drive(self.arguments(session='xvfb'))
        self.assertIn('Linux Xvfb X11 only', str(caught.exception))
        self.assertEqual(self.records(), [], 'an Xvfb run started before it was refused')
        self.assertFalse(self.publish.exists(), 'an artifact directory was published')
        self.assertFalse(self.dossier.exists(), 'a dossier was written')

    def test_result_options_without_a_result_are_refused(self):
        """An option that says what a result records, on a run that asks for none.

        Every one of these is read by the publication block and nowhere else, so an
        invocation naming one without `--results` used to have it accepted and dropped.
        Which options those are comes from the parser's own rows, so the case cannot pass
        while the rule and this list disagree about the group.
        """
        values = {'--publish': str(self.publish), '--platform': self.contract.id,
                  '--contract-sha256': COMMITTED_FINGERPRINT,
                  '--stop-condition': self.contract.stop_conditions[0],
                  '--unobservable': 'cold_start_to_first_frame_seconds=nothing',
                  '--limitation': 'nothing'}
        self.assertEqual(sorted(values), declared_options('result'),
                         'the driver declares a different set of result options than this case drives')
        for option, value in sorted(values.items()):
            with self.subTest(option=option):
                case = self.__class__('test_a_run_that_supports_its_result_publishes_it_through_main')
                case.setUp()
                self.addCleanup(case.doCleanups)
                with self.assertRaises(SystemExit) as caught:
                    case.drive(case.arguments(option, value, results=False))
                self.assertIn(option, str(caught.exception))
                self.assertEqual(case.records(), [], f'a run started with {option} and no result')
                self.assertFalse(case.publish.exists())
                self.assertFalse(case.dossier.exists())

    def test_a_wayland_option_on_the_x11_path_is_refused(self):
        """The X11 entry point prints one lifetime: no cancellation, build, or contract.

        The group is the parser's own; this case only says what value to give each option
        it declares as the Wayland path's.
        """
        values = {'--cancel-after': '5', '--build-seconds': '1.0',
                  '--build-log': '/tmp/this-case-builds-nothing.log',
                  '--contract': self.contract.id}
        self.assertEqual(sorted(values), declared_options('wayland'),
                         'the driver declares a different set of Wayland options than this case drives')
        for option, value in sorted(values.items()):
            with self.subTest(option=option):
                case = self.__class__('test_a_run_that_supports_its_result_publishes_it_through_main')
                case.setUp()
                self.addCleanup(case.doCleanups)
                with self.assertRaises(SystemExit) as caught:
                    case.drive(case.arguments(option, value, results=False, session='xvfb'))
                self.assertIn(option, str(caught.exception))
                self.assertEqual(case.records(), [], f'an Xvfb run started with {option}')

    def test_every_option_declares_which_paths_read_it(self):
        """The option→path matrix is the parser's, and every option has a row in it.

        The refusal rule, the help text and the group cases all take their rows from these
        declarations, so an option added to the command line without one would be an option
        nothing refuses and nothing labels. Both directions are asked of the parser: every
        option string it prints has a row, and every row's label is printed once for it.
        """
        parser, declared = run_proof.build_parser()
        # The option lines of `--help` are where every option string the parser carries is
        # written down: the usage line and the parsed namespace each show one form per action,
        # so an alias and a second action sharing a destination would be invisible in them.
        help_text = parser.format_help()
        listed = [re.split(r'\s{2,}', line.strip(), 1)[0] for line in help_text.splitlines()
                  if line.startswith('  -')]
        printed = set(re.findall(r'-{1,2}[a-z][a-z0-9-]*', ' '.join(listed))) - {'-h', '--help'}
        spelled = {flag for entry in declared for flag in entry['flags']}
        self.assertEqual(sorted(printed), sorted(spelled),
                         'the parser carries an option the matrix does not declare')
        labels = [run_proof.READS_LABEL[entry['reads']] for entry in declared]
        for label in sorted(set(labels) - {''}):
            with self.subTest(label=label):
                self.assertEqual(help_text.count(label), labels.count(label),
                                 'the labels --help prints are not the rows\u2019 own')

    def test_a_revision_without_a_record_is_refused(self):
        with self.assertRaises(SystemExit) as caught:
            self.drive(self.arguments('--commit', 'b' * 40, results=False))
        self.assertIn('--commit', str(caught.exception))
        self.assertEqual(self.records(), [], 'a run started with a revision it records nowhere')

    def test_a_build_figure_with_nothing_to_bear_it_is_refused(self):
        """A build time is the invocation's own word, so the log has to bear it.

        Nothing this driver runs measures a build: `--build-seconds` is the number an
        operator supplies, and `--build-log` is the only thing that can stand behind it.
        A result recording the figure with no log would state an observation nothing
        observed, and the session record would carry no build at all.
        """
        for results in (False, True):
            with self.subTest(results=results):
                case = self.__class__('test_a_run_that_supports_its_result_publishes_it_through_main')
                case.setUp()
                self.addCleanup(case.doCleanups)
                with self.assertRaises(SystemExit) as caught:
                    case.drive(case.arguments('--build-seconds', '12.5', results=results))
                self.assertIn('--build-log', str(caught.exception))
                self.assertEqual(case.records(), [], 'a run started on a figure nothing bears')
                self.assertFalse(case.publish.exists(), 'an artifact directory was published')
                self.assertFalse(case.dossier.exists(), 'a dossier was written')

    def test_a_platform_the_session_cannot_be_is_refused_through_main(self):
        """A kwin Wayland session on this host cannot publish a run for macOS, Windows or X11.

        Before this rule the invocation's word was all there was: the dossier recorded
        the platform, the untested list the ledger reads was derived from it, and the
        crate cannot catch it because the platform is a string inside the result.
        """
        for platform in ('macOS 14 arm64', 'Windows 11 x86_64', 'Linux X11'):
            with self.subTest(platform=platform):
                case = self.__class__('test_a_run_that_supports_its_result_publishes_it_through_main')
                case.setUp()
                self.addCleanup(case.doCleanups)
                with self.assertRaises(SystemExit) as caught:
                    case.drive(case.arguments('--platform', platform))
                self.assertIn(platform, str(caught.exception))
                self.assertEqual(case.records(), [], 'a run started under a platform it cannot be')
                self.assertFalse(case.publish.exists(), 'an artifact directory was published')
                self.assertFalse(case.dossier.exists(), 'a dossier was written')

    def test_the_session_record_carries_what_the_platform_has_to_name(self):
        self.drive(self.arguments())
        document = json.loads(self.dossier.read_text())
        platform = document['runs'][0]['platform']
        sessions = [Path(item['artifact']) for item in document['runs'][0]['artifacts']
                    if item['artifact'].endswith('session.json')]
        self.assertTrue(sessions, 'the run cites the session record it was measured in')
        record = read_session_record(sessions[0])
        self.assertEqual(record['schema_version'], SESSION_SCHEMA_VERSION,
                         'the record declares which shape it is, in the record')
        self.assertEqual(record['display'], 'Wayland')
        self.assertEqual(record['system'], run_proof.host_system())
        self.assertEqual(run_proof.platform_problems(platform, 'wayland', record['system']), [])

    def test_a_condition_is_recorded_as_the_declaration_it_is(self):
        self.drive(self.arguments())
        for row in json.loads(self.dossier.read_text())['runs']:
            self.assertEqual(row['stop_condition'], self.contract.stop_conditions[0])
            self.assertIn('declares', row['stop_condition_note'],
                          'which condition a run tripped is the operator\'s statement, and says so')
            self.assertIn(row['stop_condition'], row['failures'],
                          'the condition stays beside the facts a reader compares it with')

    def test_a_build_figure_with_its_log_records_the_tree_that_log_names(self):
        head = subprocess.run(['git', '-C', str(run_proof.REPO), 'rev-parse', 'HEAD'],
                              capture_output=True, text=True).stdout.strip()
        log = self.root / 'build.log'
        log.write_text('+ git rev-parse HEAD\n' + head + '\n+ cargo build --locked\n')
        printed = self.drive(self.arguments('--build-seconds', '12.5', '--build-log', str(log),
                                            results=False))
        self.assertIn('clean_locked_build_seconds', printed)
        sessions = list((self.root / 'artifacts').rglob('session.json'))
        self.assertEqual(len(sessions), 1, 'the run records the build it was given')
        record = json.loads(sessions[0].read_text())
        self.assertEqual(record['build']['seconds'], 12.5)
        self.assertEqual(record['build']['revision'], head)
        self.assertEqual(record['build']['log_sha256'], sha256_of(log))

    def test_interaction_capture_is_refused_rather_than_promised(self):
        """The option used to lengthen the run while taking no screenshot and driving nothing."""
        with self.assertRaises(SystemExit) as caught:
            self.drive(self.arguments('--interact', results=False))
        self.assertIn('no screenshot', str(caught.exception))
        self.assertEqual(self.records(), [], 'a run started under an unimplemented option')

    def test_results_without_a_publish_root_is_refused(self):
        argv = self.arguments()
        with self.assertRaises(SystemExit) as caught:
            self.drive([a for a in argv if a != '--publish' and a != str(self.publish)])
        self.assertIn('--results needs --publish', str(caught.exception))
        self.assertEqual(self.records(), [], 'a run started without the root it would publish into')
        self.assertFalse(self.dossier.exists())

    def test_a_binary_that_does_not_exist_is_refused(self):
        with self.assertRaises(SystemExit) as caught:
            self.drive(self.arguments('--binary', str(self.root / 'not-built'), results=False))
        self.assertIn('Build first', str(caught.exception))
        self.assertEqual(self.records(), [], 'a run started with no binary to run')

    def test_the_x11_entry_point_writes_only_its_own_records(self):
        self.drive(self.arguments(results=False, session='xvfb'))
        self.assertIn('process-tree.json', self.records())
        self.assertIn('app.log', self.records())
        self.assertFalse(self.publish.exists(), 'an artifact directory was published')
        self.assertFalse(self.dossier.exists(), 'a dossier was written')

    def test_a_document_that_names_no_contract_is_refused_through_main(self):
        empty = self.root / 'empty-contracts.json'
        empty.write_text(json.dumps({'contracts': []}))
        with self.assertRaises(SystemExit) as caught:
            self.drive(self.arguments('--contracts', str(empty), results=False, session='xvfb'))
        self.assertIn('names no contract', str(caught.exception))
        self.assertEqual(self.records(), [], 'a run started with no terms to answer')

    def test_a_term_the_contract_does_not_answer_is_refused_through_main(self):
        with self.assertRaises(SystemExit) as caught:
            self.drive(self.arguments(unobservable=['not_predeclared=this case']))
        self.assertIn('not_predeclared', str(caught.exception))
        self.assertFalse(self.publish.exists(), 'an artifact directory was published')
        self.assertFalse(self.dossier.exists(), 'a dossier was written')

    def test_a_recorded_revision_that_is_not_the_tree_is_refused_through_main(self):
        with self.assertRaises(SystemExit) as caught:
            self.drive(self.arguments('--commit', 'b' * 40))
        self.assertIn('the run would record revision', str(caught.exception))
        self.assertEqual(self.records(), [], 'a run started under a revision it cannot stand on')
        self.assertFalse(self.publish.exists())
        self.assertFalse(self.dossier.exists())

    def test_an_invocation_that_names_no_terms_takes_the_documents_own(self):
        """The terms an invocation leaves out come from the document it was pointed at."""
        path = self.root / 'spike-contracts.json'
        path.write_text(json.dumps(self.SYNTHETIC))
        synthetic = run_proof.Contract.read(path)
        self.drive(self.arguments('--contracts', str(path), contract=synthetic,
                                  fingerprint=SYNTHETIC_FINGERPRINT,
                                  unobservable=['suite_unseen_measurement=this case']))
        document = json.loads(self.dossier.read_text())
        self.assertEqual(document['contract'], 'SUITE/other-contract')
        self.assertEqual([row['platform'] for row in document['runs']], ['Suite One', 'Suite One'])
        self.assertEqual(document['untested_platforms'], ['Suite Two'])

    def test_a_run_that_supports_its_result_publishes_it_through_main(self):
        printed = self.drive(self.arguments())
        self.assertIn(f'wrote {self.dossier}', printed)
        document = json.loads(self.dossier.read_text())
        self.assertEqual(document['contract'], self.contract.id)
        self.assertEqual([row['platform'] for row in document['runs']],
                         [self.contract.default_platform()] * 2)
        self.assertEqual(document['untested_platforms'],
                         list(self.contract.applicable_platforms[1:]))
        published = [item for row in document['runs'] for item in row['artifacts']]
        self.assertTrue(published, 'a published run cites the artifacts it stands on')
        for item in published:
            path = Path(item['artifact'])
            self.assertTrue(path.exists(), f"{item['artifact']} is cited and not published")
            self.assertEqual(sha256_of(path), item['sha256'])


class UnkeptPaths(unittest.TestCase):
    """The record has to say which of its paths the run does not keep.

    A reader following the committed record's provenance landed on paths that no
    longer existed while the durable copies sat beside them. Both directions are
    held here: a declared path carries a note saying what outlives it, and a path
    the record cannot read back has to be declared.
    """

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        root = Path(self.directory.name)
        self.binary = root / 'symbiote-linux-shell-proof'
        self.binary.write_bytes(b'the candidate this run exercised')
        self.log = root / 'build-abcdefgh.log'
        self.log.write_text('+ git rev-parse HEAD\n' + 'a' * 40 + '\n')
        self.published = (root / 'publish' / run_proof.publish_slug(WAYLAND) / self.log.name)
        self.published.parent.mkdir(parents=True)
        shutil.copy2(self.log, self.published)
        arguments = argparse.Namespace(build_log=str(self.log), build_seconds=1.0,
                                       publish=str(root / 'publish'), platform=WAYLAND)
        self.record = run_proof.session_record(
            Session(kind='wayland', name='kwin 6.7.5 --virtual 1440x960',
                              process=None, env={'WAYLAND_DISPLAY': 'symbiote-proof'},
                              log=None, runtime=root / 'runtime'),
            self.binary, 0, run_proof.build_record(arguments, 'a' * 40))

    def paths(self):
        """Every string the record carries that reads as a path: a separator, no spaces."""
        found = []

        def walk(node, prefix):
            for key, value in node.items():
                dotted = f'{prefix}{key}'
                if isinstance(value, dict):
                    walk(value, f'{dotted}.')
                elif isinstance(value, str) and '/' in value and ' ' not in value:
                    found.append((dotted, value))
        walk(self.record, '')
        return found

    def test_each_unkept_path_says_what_outlives_it(self):
        for dotted in UNKEPT_PATHS:
            container, _, field = dotted.rpartition('.')
            holder = self.record['build'] if container else self.record
            self.assertIn(field, holder, f'{dotted} is declared unkept and is not recorded')
            self.assertTrue(holder.get(f'{field}_note'),
                            f'{dotted} is recorded and not kept, and says nothing about it')
        self.assertEqual(self.record['binary_sha256'], sha256_of(self.binary))
        self.assertEqual(self.record['build']['log_sha256'], sha256_of(self.published),
                         'the hash names the bytes of the copy that is kept')
        self.assertEqual(self.record['build']['published_log'], str(self.published))

    def test_a_session_with_no_runtime_directory_is_refused_by_name(self):
        session = Session(kind='xvfb', name='Xvfb :9', process=Ended(), env={},
                                    log=Closes(), runtime=None)
        with self.assertRaises(SystemExit) as caught:
            run_proof.session_record(session, self.binary, 0, None)
        self.assertIn('no private runtime directory', str(caught.exception))

    def test_a_path_the_record_cannot_read_back_has_to_be_declared(self):
        # The run is over: what it did not publish goes with it, and what it
        # published is still there. A record has to tell the two apart.
        self.binary.unlink()
        self.log.unlink()
        self.assertFalse(self.binary.exists())
        for dotted, value in self.paths():
            located = Path(value) if value.startswith('/') else run_proof.REPO / value
            self.assertEqual(dotted in UNKEPT_PATHS, not located.exists(),
                             f'{dotted} = {value}: declared unkept={dotted in UNKEPT_PATHS}, '
                             f'and it can be read={located.exists()}')


class SessionRecordVersion(unittest.TestCase):
    """The record's declared version names its shape, and the writer is held to it.

    The committed record's fields used to be dated by a paragraph in the shell proof,
    and a re-recording would have had to move that paragraph by hand. ``SESSION_SHAPES``
    holds the fields of each version now: the case below holds the writer's own output to
    the entry for the version it declares, so a field added or removed without a version
    change fails here, and the reader holds every record to the entry its version names,
    while a record written before there was a version still reads.
    """

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        binary = self.root / 'symbiote-linux-shell-proof'
        binary.write_bytes(b'the candidate this run exercised')
        session = Session(kind='wayland', name='kwin 6.7.5 --virtual 1440x960',
                                    process=None, env={'WAYLAND_DISPLAY': 'symbiote-proof'},
                                    log=None, runtime=self.root / 'runtime')
        self.written = run_proof.session_record(session, binary, 0, None)

    # A name no version carries, so a future shape cannot take it; the case checks that
    # for itself before it uses it.
    UNNAMED = 'a_field_no_version_names'

    def without(self, *fields):
        return {key: value for key, value in self.written.items() if key not in fields}

    def version_zero(self):
        """Version 0's record, in the shape the table gives that version.

        Subtracting what version 1 added from the writer's output was a second statement of
        that shape, and it stopped being version 0 as soon as a version was added.
        """
        return {key: value for key, value in self.written.items()
                if key in SESSION_SHAPES[0]}

    def read(self, record):
        path = self.root / 'session.json'
        path.write_text(json.dumps(record))
        return read_session_record(path)

    def test_the_writer_writes_the_shape_of_the_version_it_declares(self):
        version = self.written['schema_version']
        self.assertEqual(version, SESSION_SCHEMA_VERSION)
        self.assertEqual(set(self.written), set(SESSION_SHAPES[version]),
                         'a field added or removed without a version change is a shape this table '
                         'does not name')
        self.assertEqual(self.read(self.written), self.written)

    def test_a_version_this_reader_does_not_know_is_refused_by_name(self):
        for unknown in (max(SESSION_SHAPES) + 1, -1):
            with self.subTest(unknown=unknown):
                with self.assertRaises(SystemExit) as caught:
                    self.read({**self.written, 'schema_version': unknown})
                self.assertIn(f'declares schema version {unknown}', str(caught.exception))
                self.assertIn(str(sorted(SESSION_SHAPES)), str(caught.exception))

    def test_a_version_of_another_type_is_refused_naming_the_value(self):
        """A version is an integer of a known one: `"1"`, `null`, `true` and `1.0` are not."""
        for value in ('1', None, True, 1.0, f'{SESSION_SCHEMA_VERSION}'):
            with self.subTest(value=value):
                with self.assertRaises(SystemExit) as caught:
                    self.read({**self.written, 'schema_version': value})
                self.assertIn(f'declares schema version {value!r}', str(caught.exception))

    def test_a_record_declaring_no_version_is_read_as_the_shape_before_it(self):
        before = self.version_zero()
        self.assertEqual(self.read(before), before,
                         'the committed run\'s record is this shape, and is read')

    def test_a_record_missing_a_field_its_version_names_is_refused(self):
        with self.assertRaises(SystemExit) as caught:
            self.read(self.without('system'))
        self.assertIn("'system'", str(caught.exception))

    def test_a_record_carrying_a_field_its_version_does_not_name_is_refused(self):
        named = {field for shape in SESSION_SHAPES.values() for field in shape}
        self.assertNotIn(self.UNNAMED, named,
                         'this case needs a name no version carries; a version that took this '
                         'one would have to rename the fixture rather than fail here for the '
                         'wrong reason')
        with self.assertRaises(SystemExit) as caught:
            self.read({**self.written, self.UNNAMED: 'a field the shape does not name'})
        self.assertIn(f"'{self.UNNAMED}'", str(caught.exception))
        with self.assertRaises(SystemExit) as caught:
            self.read({**self.version_zero(), 'display': 'Wayland'})
        self.assertIn("'display'", str(caught.exception))

    def test_a_record_that_is_not_a_record_is_refused_by_name(self):
        with self.assertRaises(SystemExit) as caught:
            self.read(['wayland_display'])
        self.assertIn('not a record', str(caught.exception))


class CommittedResult(unittest.TestCase):
    """The committed dossier's own platform, against the session record it cites.

    The result artifact is the only thing a later pass reads, and its platform used to
    be the invocation's word alone. This holds the committed one to the rule the driver
    now applies: the platform it names is one the session record beside it substantiates.
    That record declares no schema version, so it is the shape written before version 1
    added ``schema_version``, ``display`` and ``system``: the operating-system side is read from the machine
    this suite runs on — Linux here and in CI — and the display side from the record
    itself, read through the driver's own reader rather than from a shape this test
    assumes. The pin on the declaration is what makes the paragraph in
    ``docs/proofs/linux-shell.md`` move with a re-recording.
    """

    def test_the_committed_platform_is_one_its_session_record_substantiates(self):
        document = json.loads((run_proof.REPO / 'docs/proofs/results/desktop-shell.json').read_text())
        contract = run_proof.Contract.read(run_proof.REPO / run_proof.CONTRACTS_PATH)
        for row in document['runs']:
            self.assertIn(row['platform'], contract.applicable_platforms)
            records = [run_proof.REPO / item['artifact'] for item in row['artifacts']
                       if item['artifact'].endswith('session.json')]
            self.assertTrue(records, 'a run cites the session record it was measured in')
            session = read_session_record(records[0])
            self.assertTrue(session.get('wayland_display'),
                            'the record shows the display server the platform has to name')
            self.assertNotIn(
                'schema_version', session,
                'the committed record declares no schema version, so it is version 0 in '
                'SESSION_SHAPES; a re-recording declares the current version instead, and the '
                'paragraph in docs/proofs/linux-shell.md moves with this pin')
            self.assertEqual(
                run_proof.platform_problems(row['platform'], 'wayland',
                                            session.get('system', run_proof.host_system())),
                [], f"{row['platform']} is not one the session record beside it substantiates")


class ConcernMap(unittest.TestCase):
    """The map from concerns to modules has one owner, and cannot go stale.

    Its owner is the entry point's docstring, which `argparse` prints as the description
    of `--help` — so the map a reader gets is the map held here: what `--help` prints is
    checked to name every module of the package and nothing else, and no second place the
    fixture owns may name them all again. A module counts as named when the file it lives
    in is named — `shellproof/records.py` or `records.py`, however that is quoted — so a
    citation of one or two of them in an option's help text, in a module's docstring or in
    the fixture document is not a second map and is not refused, while the same list in
    full, in any of those places, is. A module added, moved or renamed is then one edit,
    and a map that has stopped describing the package fails rather than being read as true.

    The limits are measured, not implied: a list naming most but not all of them, a list
    that names them without their files (bare stems, a table, a Python constant), and one
    in a document other than the fixture's own are caught by neither case — only the
    fixture's Python files and that one document are read.
    """

    FIXTURE = Path(__file__).resolve().parent
    DOCUMENT = FIXTURE.parents[1] / 'docs/proofs/linux-shell.md'
    MAP = re.compile(r'``shellproof/([a-z_]+)\.py``')          # the map's own form, as printed
    BY_FILE = re.compile(r'(?:shellproof/)?([a-z_]+)\.py')     # a module named by its file

    def modules(self):
        return {path.stem for path in (self.FIXTURE / 'shellproof').glob('*.py')
                if path.stem != '__init__'}

    def named(self, text):
        """Which of the package's modules this text names, by the file it names each as."""
        return {match.group(1) for match in self.BY_FILE.finditer(text)} & self.modules()

    def printed(self):
        """The map as a reader gets it: the description of `--help`, driven through main."""
        printed = io.StringIO()
        with mock.patch.object(sys, 'argv', ['run-proof.py', '--help']), \
                contextlib.redirect_stdout(printed):
            with self.assertRaises(SystemExit):
                run_proof.main()
        return printed.getvalue()

    def test_the_concern_map_names_every_module_of_the_package(self):
        named = {match.group(1) for match in self.MAP.finditer(self.printed())}
        self.assertTrue(self.modules(), 'the package holds no module for the map to name')
        self.assertEqual(named, self.modules(),
                         'the concern map `--help` prints and the package disagree: a module added '
                         'or renamed is one edit in the entry point docstring the reader sees')

    def test_the_concern_map_is_stated_once(self):
        docstring = run_proof.__doc__ or ''
        entry = (self.FIXTURE / 'run-proof.py').read_text()
        regions = [(path.name, path.read_text()) for path in
                   [self.FIXTURE / 'run-proof.py'] + sorted(
                       (self.FIXTURE / 'shellproof').glob('*.py'))]
        regions = [(name, text.replace(docstring, '', 1) if name == 'run-proof.py' else text)
                   for name, text in regions]
        regions.append((str(self.DOCUMENT.relative_to(self.FIXTURE.parents[1])),
                        self.DOCUMENT.read_text()))
        elsewhere = [name for name, text in regions if self.named(text) == self.modules()]
        self.assertEqual(elsewhere, [],
                         'the concern map is the entry point docstring, which `--help` prints; a '
                         'second place naming every module of the package is the copy that goes '
                         'stale')


if __name__ == '__main__':
    unittest.main()
