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


if __name__ == '__main__':
    unittest.main()
