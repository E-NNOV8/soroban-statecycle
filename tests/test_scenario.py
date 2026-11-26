"""General configuration/verifier behavior, separate from live Core proof."""
import copy
import json
from pathlib import Path
import sys
import os
import subprocess
import tempfile
import unittest
from archival_regression.campaign import Campaign,Inconclusive,Violation,compare
from archival_regression.scenario import fresh,load_scenario,validate

ROOT=Path(__file__).resolve().parents[1]
class ScenarioTests(unittest.TestCase):
    def setUp(self):self.config=load_scenario(ROOT/'scenarios/blend-config-admin.json')

    def test_invalid_phase_configuration_exits_inconclusive_in_real_process(self):
        del self.config['hooks']['restore']
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'invalid.json'
            path.write_text(json.dumps(self.config))
            process=subprocess.run([sys.executable,'-m','archival_regression.campaign','--config',str(path),'--output',str(Path(folder)/'out'),'--phase','archive'],cwd=ROOT,env={**os.environ,'PYTHONPATH':str(ROOT/'src')},capture_output=True,text=True)
            self.assertEqual(process.returncode,2,process.stderr)
            self.assertEqual(json.loads(process.stdout)['result'],'INCONCLUSIVE')
            self.assertFalse((Path(folder)/'out').exists())

    def test_named_instance_model_parses(self):
        self.assertEqual(set(self.config['entries']),{'instance'})

    def test_target_label_does_not_change_engine_model(self):
        cfg=copy.deepcopy(self.config);cfg['name']='another-application'
        validate(cfg)
        self.assertEqual(cfg['entries'],self.config['entries'])

    def test_fresh_salt_is_resolved_without_changing_source(self):
        cfg=fresh(self.config,'ab'*32)
        self.assertIn('ab'*32,cfg['hooks']['deploy'])
        self.assertIn('{salt}',self.config['hooks']['deploy'])

    def test_invalid_salt_is_rejected(self):
        with self.assertRaises(Inconclusive):fresh(self.config,'bad')

    def test_missing_restore_is_invalid_configuration(self):
        del self.config['hooks']['restore']
        with self.assertRaises(Inconclusive):validate(self.config)

    def test_shell_string_is_not_an_argv_hook(self):
        self.config['hooks']['setup']='echo setup'
        with self.assertRaises(Inconclusive):validate(self.config)

    def test_temporary_storage_cannot_be_archival_observation(self):
        self.config['entries']['instance']['contract_data']['durability']='temporary'
        with self.assertRaises(Inconclusive):validate(self.config)

    def test_binding_requires_declared_entry(self):
        self.config['bindings']['get_admin']['entry']='missing'
        with self.assertRaises(Inconclusive):validate(self.config)

    def test_mutation_requires_distinct_declared_values(self):
        self.config.update(mutation_required=True,expected_initial={'value':1},expected_before={'value':1})
        self.config['hooks']['mutate']=[sys.executable,'-c','print(1)']
        with self.assertRaises(Inconclusive):validate(self.config)

    def test_hook_failure_is_infrastructure_inconclusive_and_retained(self):
        with tempfile.TemporaryDirectory() as folder:
            c=Campaign(self.config,Path(folder))
            with self.assertRaises(Inconclusive):c.command([sys.executable,'-c','import sys; print("fixture hook failed",file=sys.stderr); sys.exit(3)'])
            r=json.loads(next(Path(folder).glob('evidence-*.json')).read_text())
            self.assertEqual(r['exit_code'],3)
            self.assertIn('fixture hook failed',r['stderr'])

    def test_raw_projection_reuses_original_real_instance(self):
        source=ROOT/'docs/experiments/stage1/host-resume-20261015'
        raw=json.loads(json.loads((source/'pilot-live-decoded.json').read_text())['stdout'])
        public=json.loads((source/'pilot-checkpoint.json').read_text())['baseline']
        compare({'instance':raw},public,self.config['bindings'])

    def test_persistent_scalar_supports_same_binding_model(self):
        raw={'record':{'contract_data':{'val':{'string':'mutated-Y'}}}}
        bindings={'observation':{'entry':'record','path':['contract_data','val']}}
        compare(raw,{'observation':'mutated-Y'},bindings)
        with self.assertRaises(Violation):compare(raw,{'observation':'initial-X'},bindings)

    def test_missing_raw_field_is_inconclusive(self):
        with self.assertRaises(Inconclusive):compare({'record':{}},{'value':1},{'value':{'entry':'record','path':['contract_data','val']}})

    def test_unbounded_archive_timeout_is_rejected(self):
        self.config['archive_timeout_seconds']=1000000000
        with self.assertRaises(Inconclusive):validate(self.config)

    def test_ttl_ledger_bounds_validation(self):
        cfg = copy.deepcopy(self.config)
        cfg['min_ttl_ledgers'] = 100
        cfg['max_ttl_ledgers'] = 500
        validate(cfg)

        # Invalid: min < 1
        cfg['min_ttl_ledgers'] = 0
        with self.assertRaises(Inconclusive):validate(cfg)

        # Invalid: max < min
        cfg['min_ttl_ledgers'] = 500
        cfg['max_ttl_ledgers'] = 200
        with self.assertRaises(Inconclusive):validate(cfg)

    def test_scval_decoding_timepoint_and_duration(self):
        from archival_regression.campaign import scval
        self.assertEqual(scval({'timepoint': 1732014000}), 1732014000)
        self.assertEqual(scval({'duration': 3600}), 3600)

    def test_executable_hash_validation(self):
        from archival_regression.scenario import validate_executable_hash
        valid_hash = 'a' * 64
        self.assertEqual(validate_executable_hash({'wasm': valid_hash}), valid_hash)
        self.assertEqual(validate_executable_hash({'wasm_hash': valid_hash.upper()}), valid_hash)
        self.assertEqual(validate_executable_hash({'token': True}), 'token')

        # Invalid length
        with self.assertRaises(Inconclusive):
            validate_executable_hash({'wasm': 'short'})
        # Invalid non-hex characters
        with self.assertRaises(Inconclusive):
            validate_executable_hash({'wasm': 'z' * 64})
        # Empty or non-dict
        with self.assertRaises(Inconclusive):
            validate_executable_hash({})
        with self.assertRaises(Inconclusive):
            validate_executable_hash(None)

    def test_archive_witness_metadata_validation(self):
        from archival_regression.scenario import validate_archive_witness_metadata
        valid = {'ledger_sequence': 1234, 'eviction_keys': ['key1', 'key2']}
        self.assertTrue(validate_archive_witness_metadata(valid))

        cfg = copy.deepcopy(self.config)
        cfg['archive_witness_metadata'] = valid
        validate(cfg)

        with self.assertRaises(Inconclusive):
            validate_archive_witness_metadata('not-a-dict')

        with self.assertRaises(Inconclusive):
            validate_archive_witness_metadata({'ledger_sequence': -5})

        with self.assertRaises(Inconclusive):
            validate_archive_witness_metadata({'eviction_keys': []})

        with self.assertRaises(Inconclusive):
            validate_archive_witness_metadata({'eviction_keys': ['']})

    def test_archival_checkpoint_fixture_structure(self):
        fixture_p = ROOT / 'examples' / 'fixtures' / 'archival_checkpoint_fixture.json'
        self.assertTrue(fixture_p.exists())
        data = json.loads(fixture_p.read_text())
        self.assertEqual(data['scenario_name'], 'vault_archival_recovery')
        self.assertEqual(len(data['checkpoints']), 3)
        self.assertIn('live_until_ledger', data['checkpoints'][0])

    def test_ttl_monotonicity_validator(self):
        from archival_regression.scenario import validate_ttl_monotonicity
        # Monotonic checkpoints
        cps = [
            {'ledger': 100, 'live_until_ledger': 500, 'archived': False},
            {'ledger': 200, 'live_until_ledger': 600, 'archived': False},
        ]
        valid, err = validate_ttl_monotonicity(cps)
        self.assertTrue(valid)
        self.assertIsNone(err)

        # Regressed checkpoints
        bad_cps = [
            {'ledger': 100, 'live_until_ledger': 600, 'archived': False},
            {'ledger': 200, 'live_until_ledger': 400, 'archived': False},
        ]
        bad_valid, bad_err = validate_ttl_monotonicity(bad_cps)
        self.assertFalse(bad_valid)
        self.assertIn('regressed from previous', bad_err)

    def test_contract_durability_verifier(self):
        from archival_regression.scenario import verify_contract_durability
        before = {"admin": "GAAA", "balance": 1000, "active": True}
        after_same = {"admin": "GAAA", "balance": 1000, "active": True}
        valid, err = verify_contract_durability(before, after_same)
        self.assertTrue(valid)
        self.assertIsNone(err)

        # Mutated value
        after_mutated = {"admin": "GAAA", "balance": 999, "active": True}
        valid_mut, err_mut = verify_contract_durability(before, after_mutated)
        self.assertFalse(valid_mut)
        self.assertIn("Value mismatch for key 'balance'", err_mut)

        # Key mismatch
        after_missing = {"admin": "GAAA", "balance": 1000}
        valid_miss, err_miss = verify_contract_durability(before, after_missing)
        self.assertFalse(valid_miss)
        self.assertIn("Key set mismatch", err_miss)

        # Type mismatch
        after_type = {"admin": "GAAA", "balance": "1000", "active": True}
        valid_type, err_type = verify_contract_durability(before, after_type)
        self.assertFalse(valid_type)
        self.assertIn("Type mismatch for key 'balance'", err_type)

    def test_archival_dependency_graph_validation(self):
        from archival_regression.scenario import verify_archival_dependency_graph
        clean_graph = {"router": ["pool", "token"], "pool": ["token"], "token": []}
        ok, err = verify_archival_dependency_graph(clean_graph)
        self.assertTrue(ok)
        self.assertIsNone(err)

        cyclic = {"a": ["b"], "b": ["a"]}
        ok_cyc, err_cyc = verify_archival_dependency_graph(cyclic)
        self.assertFalse(ok_cyc)
        self.assertIn("circular archival dependency", err_cyc)

        missing = {"router": ["ghost_pool"]}
        ok_miss, err_miss = verify_archival_dependency_graph(missing)
        self.assertFalse(ok_miss)
        self.assertIn("undeclared contract", err_miss)

    def test_compute_restoration_order_restores_dependencies_first(self):
        from archival_regression.scenario import compute_restoration_order
        graph = {"router": ["pool", "token"], "pool": ["token"], "token": []}
        order = compute_restoration_order(graph)
        self.assertEqual(order.index("token"), 0)
        self.assertLess(order.index("pool"), order.index("router"))

    def test_export_restoration_plan_structure(self):
        from archival_regression.scenario import export_restoration_plan
        graph = {"router": ["pool"], "pool": ["token"], "token": []}
        plan = export_restoration_plan(graph)
        self.assertEqual(plan["total_contracts"], 3)
        self.assertEqual(plan["restoration_order"], ["token", "pool", "router"])

if __name__=='__main__':unittest.main()

