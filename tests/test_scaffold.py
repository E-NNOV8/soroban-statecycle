"""Tests for scenario scaffolding utility."""
import json
import subprocess
import sys
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
sys.path.insert(0, str(ROOT / 'src'))

from scaffold_scenario import build_scenario_template
from archival_regression.scenario import validate


class ScaffoldScenarioTests(unittest.TestCase):
    def test_default_scaffold_produces_valid_scenario(self):
        scenario = build_scenario_template(
            name='my-app',
            bindings=['get_val'],
        )
        self.assertEqual(scenario['schema'], 'archival-regression/scenario-v2')
        self.assertEqual(scenario['protocol'], 28)
        self.assertIn('instance', scenario['entries'])
        self.assertIn('get_val', scenario['bindings'])
        self.assertIn('get_val', scenario['hooks'])
        self.assertIn('deploy', scenario['hooks'])
        self.assertIn('restore', scenario['hooks'])
        # Confirms it passes StateCycle's official validator
        validate(scenario)

    def test_mutation_scaffold_produces_valid_distinct_assertions(self):
        scenario = build_scenario_template(
            name='mutable-app',
            bindings=['get_state', 'get_owner'],
            include_mutation=True,
        )
        self.assertTrue(scenario.get('mutation_required'))
        self.assertIn('mutate', scenario['hooks'])
        self.assertIn('expected_initial', scenario)
        self.assertIn('expected_before', scenario)
        self.assertNotEqual(scenario['expected_initial'], scenario['expected_before'])
        self.assertEqual(set(scenario['expected_initial']), {'get_state', 'get_owner'})
        validate(scenario)

    def test_missing_bindings_fails_closed(self):
        with self.assertRaises(ValueError):
            build_scenario_template(name='no-bindings', bindings=[])

    def test_invalid_protocol_fails_closed(self):
        with self.assertRaises(ValueError):
            build_scenario_template(name='bad-proto', bindings=['get_count'], protocol=25)

    def test_reserved_binding_name_fails_closed(self):
        with self.assertRaises(ValueError):
            build_scenario_template(name='reserved', bindings=['restore'])

    def test_cli_invocation_generates_valid_json(self):
        cmd = [
            sys.executable,
            str(ROOT / 'scripts' / 'scaffold_scenario.py'),
            '--name', 'cli-scaffolded',
            '--bindings', 'get_val,get_status',
            '--has-mutation',
        ]
        res = subprocess.run(cmd, capture_output=True, text=True, check=True)
        data = json.loads(res.stdout)
        self.assertEqual(data['name'], 'cli-scaffolded')
        self.assertIn('get_status', data['bindings'])
        validate(data)


if __name__ == '__main__':
    unittest.main()
