"""The runner's own rules: what a run may publish, and what it must refuse.

These hold the parts of ``run-proof.py`` that decide what the result artifact is
allowed to claim — the attestation read from the fixture's log, the contract
coverage of its measurements, the citations it publishes and the revision it
records — without needing a session, a binary or a compositor. The rules matter
because the artifact is the only thing a later pass reads: an attestation that
can be typed, a predeclared measurement left silent, an artifact that recorded
nothing cited as evidence, or a revision that is not the tree, would each let the
dossier claim more than the run shows.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import tempfile
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


if __name__ == '__main__':
    unittest.main()
