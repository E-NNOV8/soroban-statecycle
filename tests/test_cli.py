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

    def test_validate_with_summary_flag(self):
        cmd = [
            sys.executable,
            '-m',
            'archival_regression',
            'run',
            str(SCENARIO),
            '--validate-only',
            '--summary',
        ]
        res = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True)
        self.assertEqual(res.returncode, 0, f"CLI stderr: {res.stderr}")
        data = json.loads(res.stdout)
        self.assertEqual(data.get('result'), 'VALID')
        self.assertIn('phases', data)
        self.assertEqual(data['phases'], ['archive', 'negative', 'restore-checkpoint', 'resume-retry', 'verify'])
        self.assertIn('bindings_count', data)

    def test_cli_guide_exists_and_contains_sections(self):
        guide = ROOT / 'docs' / 'CLI_GUIDE.md'
        self.assertTrue(guide.exists())
        content = guide.read_text()
        self.assertIn('# Soroban Statecycle Archival Regression Campaign Guide', content)
        self.assertIn('Lifecycle Phases Verified', content)
        self.assertIn('--phase archive', content)
        self.assertIn('--phase recover', content)


if __name__ == '__main__':

    unittest.main()
