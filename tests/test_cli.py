"""Tests for archival_regression CLI entry point."""
import json
import subprocess
import sys
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCENARIO = ROOT / 'docs/experiments/stage2/blend-live.scenario.json'


class CliTests(unittest.TestCase):
    def test_validate_only_dry_run_exits_zero(self):
        cmd = [
            sys.executable,
            '-m',
            'archival_regression',
            'run',
            str(SCENARIO),
            '--validate-only',
        ]
        res = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True)
        self.assertEqual(res.returncode, 0, f"CLI stderr: {res.stderr}")
        data = json.loads(res.stdout)
        self.assertEqual(data.get('result'), 'VALID')
        self.assertEqual(data.get('protocol'), 28)

    def test_run_without_output_and_without_validate_fails(self):
        cmd = [
            sys.executable,
            '-m',
            'archival_regression',
            'run',
            str(SCENARIO),
        ]
        res = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True)
        self.assertEqual(res.returncode, 2)
        data = json.loads(res.stdout)
        self.assertEqual(data.get('result'), 'INCONCLUSIVE')


if __name__ == '__main__':
    unittest.main()
