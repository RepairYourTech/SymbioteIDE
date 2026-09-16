"""The runner's own rules: what a run may publish, and what it must refuse.

These hold the parts of ``run-proof.py`` that decide what the result artifact is
allowed to claim — the attestation read from the fixture's log, the contract
coverage of its measurements, the citations it publishes, the revision it records,
the paths it does not keep and the survivors it counts — without needing a
session, a binary or a compositor. The rules matter because the artifact is the
only thing a later pass reads: an attestation that can be typed, a predeclared
measurement left silent, an artifact that recorded nothing cited as evidence, a
revision that is not the tree, a path the run removed published as provenance, or
an unreaped process counted as running, would each let the dossier claim more than
the run shows.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import tempfile
import time
import unittest

RUNNER = Path(__file__).resolve().parent / 'run-proof.py'
spec = importlib.util.spec_from_file_location('run_proof', RUNNER)
run_proof = importlib.util.module_from_spec(spec)
spec.loader.exec_module(run_proof)

CONTRACT = {
    'id': '#38/desktop-shell-representative-workload',
    'applicable_platforms': ['Linux Wayland on the reference compositor', 'Linux X11'],
    'measurements': [{'name': 'cold_start_to_first_frame_seconds', 'unit': 's', 'maximum': 3.0},
                     {'name': 'worst_input_starvation_ms', 'unit': 'ms', 'maximum': 250.0}],
    'stop_conditions': ['#38: an untested platform stays pending and keeps the choice from settling'],
}


def args(**overrides):
    values = {'platform': 'Linux Wayland on the reference compositor',
              'stop_condition': CONTRACT['stop_conditions'][0],
              'unobservable': ['worst_input_starvation_ms=the driver issues no input'],
              'limitation': []}
    values.update(overrides)
    return argparse.Namespace(**values)


def run(figures, exercised):
    """One run as the driver records it: observations are measurement/figure pairs."""
    return {'figures': [{'measurement': name, 'observed': 1.0} for name in figures],
            'exercised': exercised}


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
        for obligation, _, _ in run_proof.ATTESTED_BY:
            self.assertNotEqual(obligation, self.STREAMS)
        text = self.log(3, 2) + 'synthetic_streams=4\n'
        self.assertNotIn(self.STREAMS, run_proof.attested(text),
                         'four synthetic streams are not four agent streams')


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
        self.assertEqual(run_proof.uncommitted(status), [])
        self.assertEqual(len(run_proof.uncommitted(status + ' M spikes/linux-shell/run-proof.py\n')), 1)
        self.assertEqual(len(run_proof.uncommitted('M  crates/symbiote-architecture/src/lib.rs\n')), 1)


FINGERPRINT = 'f' * 64
WAYLAND = 'Linux Wayland on the reference compositor'


def dossier(runs, contract=CONTRACT['id'], fingerprint=FINGERPRINT):
    return {'schema_version': 1, 'contract': contract, 'contract_sha256': fingerprint,
            'untested_platforms': ['Linux X11'], 'runs': runs}


def entry(platform, observed):
    return {'platform': platform, 'version': 'v', 'hardware': 'h', 'commit': 'c' * 40,
            'exercised': ['an attested obligation'], 'outcome': 'stop_condition_triggered',
            'stop_condition': CONTRACT['stop_conditions'][0], 'failures': ['what it did not see'],
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
        self.assertEqual(run_proof.untested_platforms(CONTRACT, []), CONTRACT['applicable_platforms'])

    def test_rerunning_a_platform_replaces_only_its_own_entries(self):
        wayland, old = entry(WAYLAND, 0.1), entry('Linux X11', 0.2)
        self.path.write_text(json.dumps(dossier([wayland, old])))
        fresh = entry('Linux X11', 0.3)
        merged = run_proof.merged_runs(self.path, CONTRACT, FINGERPRINT, 'Linux X11', [fresh])
        self.assertEqual([row for row in merged if row['platform'] == 'Linux X11'], [fresh])
        self.assertEqual([row for row in merged if row['platform'] != 'Linux X11'], [wayland])

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
            self.assertIsNone(run_proof.logged_revision(log.read_text()))
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


class Survivors(unittest.TestCase):
    """A survivor is a process still running, not a /proc entry still listed."""

    def row(self, state, ticks=42):
        return {7: {'pid': 7, 'name': 'app', 'ppid': 1, 'start_ticks': ticks, 'state': state}}

    def test_a_process_that_has_exited_is_not_a_survivor(self):
        for state in 'SR':
            self.assertEqual(run_proof.survivors_of({7: 42}, self.row(state)), [7])
        for state in run_proof.EXITED_STATES:
            self.assertEqual(run_proof.survivors_of({7: 42}, self.row(state)), [],
                             f'a process in state {state} has exited and runs nothing')
            self.assertEqual(run_proof.unreaped_of({7: 42}, self.row(state)), [7],
                             'and is named rather than lost')

    def test_a_pid_the_kernel_reused_is_not_the_process_this_run_saw(self):
        self.assertEqual(run_proof.survivors_of({7: 42}, self.row('S', ticks=43)), [])
        self.assertEqual(run_proof.unreaped_of({7: 42}, self.row('S', ticks=43)), [])

    @unittest.skipUnless(hasattr(os, 'fork'), 'needs a /proc to ask')
    def test_a_child_never_reaped_is_not_counted_as_running(self):
        """The same question put to the kernel rather than to a fixture."""
        pid = os.fork()
        if pid == 0:
            os._exit(0)
        try:
            deadline = time.monotonic() + 10
            table = run_proof.processes()
            while time.monotonic() < deadline and table.get(pid, {}).get('state') != 'Z':
                time.sleep(0.01)
                table = run_proof.processes()
            self.assertEqual(table.get(pid, {}).get('state'), 'Z',
                             'the child exited and this process never reaped it')
            seen = {pid: table[pid]['start_ticks']}
            self.assertEqual(run_proof.survivors_of(seen, table), [])
            self.assertEqual(run_proof.unreaped_of(seen, table), [pid])
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
                table = run_proof.processes()
                if table.get(pid, {}).get('state') == 'Z':
                    break
                time.sleep(0.01)
            record = run_proof.cleanup({pid: table[pid]['start_ticks']}, [])
            self.assertEqual(record['survivors_after_cancel_request'], 0,
                             'nothing is running, however many entries /proc lists')
            self.assertEqual(record['unreaped_observed_pids'], [pid])
            self.assertEqual(record['remaining_observed_pids'], [])
            self.assertEqual(record['listening_ports_after_cancel_request'], [])
        finally:
            os.waitpid(pid, 0)


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
            'kwin 6.7.5 --virtual 1440x960', {'WAYLAND_DISPLAY': 'symbiote-proof'}, self.binary,
            root / 'runtime', 0, run_proof.build_record(arguments, 'a' * 40))

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
        for dotted in run_proof.UNKEPT_PATHS:
            container, _, field = dotted.rpartition('.')
            holder = self.record['build'] if container else self.record
            self.assertIn(field, holder, f'{dotted} is declared unkept and is not recorded')
            self.assertTrue(holder.get(f'{field}_note'),
                            f'{dotted} is recorded and not kept, and says nothing about it')
        self.assertEqual(self.record['binary_sha256'], run_proof.sha256_of(self.binary))
        self.assertEqual(self.record['build']['log_sha256'], run_proof.sha256_of(self.published),
                         'the hash names the bytes of the copy that is kept')
        self.assertEqual(self.record['build']['published_log'], str(self.published))

    def test_a_path_the_record_cannot_read_back_has_to_be_declared(self):
        # The run is over: what it did not publish goes with it, and what it
        # published is still there. A record has to tell the two apart.
        self.binary.unlink()
        self.log.unlink()
        self.assertFalse(self.binary.exists())
        for dotted, value in self.paths():
            located = Path(value) if value.startswith('/') else run_proof.REPO / value
            self.assertEqual(dotted in run_proof.UNKEPT_PATHS, not located.exists(),
                             f'{dotted} = {value}: declared unkept={dotted in run_proof.UNKEPT_PATHS}, '
                             f'and it can be read={located.exists()}')


if __name__ == '__main__':
    unittest.main()
