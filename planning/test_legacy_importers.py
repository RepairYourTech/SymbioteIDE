"""Historical entrypoints must refuse execution before imports or credentials."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class LegacyImporterTests(unittest.TestCase):
    def test_retired_entrypoints_refuse_without_credentials(self):
        cases = [
            (sys.executable, '.symbiote-planning/v2/import_issues_v2.py'),
            (sys.executable, 'planning/bootstrap/import_symbiote_roadmap_v2.py'),
            (sys.executable, 'planning/bootstrap/run_symbiote_roadmap_v2_final.py'),
            ('node', 'planning/symbiote-v2/importer.cjs'),
        ]
        with tempfile.TemporaryDirectory() as directory:
            for runtime, path in cases:
                with self.subTest(path=path):
                    result = subprocess.run(
                        [runtime, str(ROOT / path)], cwd=directory,
                        env={'PATH': os.environ['PATH']},
                        text=True, capture_output=True, timeout=10,
                    )
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn('RETIRED_STALE_IMPORTER', result.stderr)
                    self.assertEqual(list(Path(directory).iterdir()), [])


if __name__ == '__main__':
    unittest.main()
