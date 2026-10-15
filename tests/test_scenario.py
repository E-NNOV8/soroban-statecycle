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

if __name__=='__main__':unittest.main()
