"""Failure semantics exercised against retained real Core/Blend observations.
These tests check the verifier; live campaigns supply protocol proof separately.
"""
import copy
import json
from pathlib import Path
import unittest
import tempfile
from archival_regression.campaign import Campaign, Inconclusive, Violation, compare, require_recovered, scval

ROOT=Path(__file__).resolve().parents[1]
EVIDENCE=ROOT/'docs/experiments/stage1/host-resume-20261015'

class CampaignTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw=json.loads(json.loads((EVIDENCE/'pilot-live-decoded.json').read_text())['stdout'])
        cls.public=json.loads((EVIDENCE/'pilot-checkpoint.json').read_text())['baseline']
        cls.bindings={'get_config':'Config','get_admin':'Admin'}
        cls.witness=json.loads((EVIDENCE/'pilot-archived.json').read_text())['archive_witness']

    def test_observed_public_values_match_independent_raw_instance(self):
        compare(self.raw,self.public,self.bindings)

    def test_wrong_admin_is_violation(self):
        public=copy.deepcopy(self.public)
        public['get_admin']=public['get_config']['oracle']
        with self.assertRaises(Violation):compare(self.raw,public,self.bindings)

    def test_changed_config_is_violation(self):
        public=copy.deepcopy(self.public)
        public['get_config']['max_positions']+=1
        with self.assertRaises(Violation):compare(self.raw,public,self.bindings)

    def test_missing_public_field_is_inconclusive(self):
        with self.assertRaises(Inconclusive):compare(self.raw,{},self.bindings)

    def test_missing_raw_config_is_inconclusive(self):
        raw=copy.deepcopy(self.raw)
        storage=raw['contract_data']['val']['contract_instance']['storage']
        storage[:]=[p for p in storage if p['key']!={'symbol':'Config'}]
        with self.assertRaises(Inconclusive):compare(raw,self.public,self.bindings)

    def test_no_core_witness_is_inconclusive_even_with_expired_ttl(self):
        with self.assertRaises(Inconclusive):require_recovered({'expired':True,'rpc_entry_missing':True})

    def test_actual_archive_with_omitted_restore_is_violation(self):
        with self.assertRaisesRegex(Violation,'restoration omitted'):
            require_recovered({'archive_witness':self.witness})

    def test_missing_after_observation_is_inconclusive(self):
        with self.assertRaises(Inconclusive):
            require_recovered({'archive_witness':self.witness,'restore_transactions':[{'hash':'placeholder-for-classifier-only'}]})

    def test_duplicate_map_key_is_inconclusive(self):
        with self.assertRaises(Inconclusive):scval({'map':[{'key':{'symbol':'a'},'val':{'u32':1}}]*2})

    def test_unknown_scval_is_inconclusive(self):
        with self.assertRaises(Inconclusive):scval({'unrecognized':0})

    def test_large_integer_remains_exact(self):
        self.assertEqual(scval({'i128':'170141183460469231731687303715884105727'}),'170141183460469231731687303715884105727')

    def retained_campaign(self, folder):
        source=EVIDENCE/'run-1'
        state=json.loads((source/'checkpoint.json').read_text())
        config=json.loads((source/'config.json').read_text())
        (folder/'checkpoint.json').write_text(json.dumps(state))
        for name in [state['archive_witness']['rpc_evidence'],state['restore_transactions'][0]['evidence']]:
            (folder/name).write_bytes((source/name).read_bytes())
        return Campaign(config,folder)

    def test_actual_raw_core_metadata_and_restore_receipt_reverify(self):
        with tempfile.TemporaryDirectory() as directory:
            self.retained_campaign(Path(directory)).verify_recovery()

    def test_wrong_key_cannot_use_actual_eviction_of_other_entry(self):
        with tempfile.TemporaryDirectory() as directory:
            campaign=self.retained_campaign(Path(directory))
            campaign.state['key']['contract_data']['contract']=self.raw['contract_data']['contract']
            with self.assertRaises(Inconclusive):campaign.validate_archive()

    def test_checkpoint_cannot_replace_missing_raw_archive_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            campaign=self.retained_campaign(Path(directory))
            (Path(directory)/campaign.state['archive_witness']['rpc_evidence']).unlink()
            with self.assertRaises(OSError):campaign.validate_archive()

    def test_failed_raw_restore_receipt_is_inconclusive(self):
        with tempfile.TemporaryDirectory() as directory:
            campaign=self.retained_campaign(Path(directory))
            path=Path(directory)/campaign.state['restore_transactions'][0]['evidence']
            record=json.loads(path.read_text())
            data=json.loads(record['body']); data['result']['status']='FAILED'
            record['body']=json.dumps(data); path.write_text(json.dumps(record))
            with self.assertRaises(Inconclusive):campaign.verify_recovery()

    def test_ledger_sequence_monotonic_advancement(self):
        with tempfile.TemporaryDirectory() as directory:
            campaign=self.retained_campaign(Path(directory))
            self.assertEqual(campaign.track_ledger_sequence(100),100)
            self.assertEqual(campaign.track_ledger_sequence(101),101)
            self.assertEqual(campaign.state['last_ledger_sequence'],101)

    def test_ledger_sequence_regression_raises_violation(self):
        with tempfile.TemporaryDirectory() as directory:
            campaign=self.retained_campaign(Path(directory))
            campaign.track_ledger_sequence(105)
            with self.assertRaises(Violation):
                campaign.track_ledger_sequence(104)

    def test_invalid_ledger_sequence_raises_inconclusive(self):
        with tempfile.TemporaryDirectory() as directory:
            campaign=self.retained_campaign(Path(directory))
            with self.assertRaises(Inconclusive):
                campaign.track_ledger_sequence(-1)
            with self.assertRaises(Inconclusive):
                campaign.track_ledger_sequence("not-an-int")

    def test_multi_entry_restoration_success_and_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            campaign = self.retained_campaign(Path(directory))
            entries = [
                {'key': 'contract_a_data', 'restored': True},
                {'key': 'contract_b_instance', 'restored': True},
            ]
            tx = {'tx_hash': '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef'}
            result = campaign.verify_multi_entry_restoration(entries, tx)
            self.assertEqual(result['status'], 'VERIFIED')
            self.assertEqual(result['count'], 2)

            unrestored = [
                {'key': 'contract_a_data', 'restored': True},
                {'key': 'contract_b_instance', 'restored': False},
            ]
            with self.assertRaises(Violation):
                campaign.verify_multi_entry_restoration(unrestored, tx)

    def test_campaign_phase_timings_csv_export(self):
        from archival_regression.campaign import export_phase_timings_csv
        timings = {
            'deploy': {'duration': 1.25, 'status': 'COMPLETED'},
            'archive': {'duration': 4.50, 'status': 'COMPLETED'},
            'restore': {'duration': 2.10, 'status': 'COMPLETED'},
        }
        csv = export_phase_timings_csv(timings)
        self.assertTrue(csv.startswith("phase,duration_seconds,status\n"))
        self.assertIn("deploy,1.25,COMPLETED", csv)
        self.assertIn("archive,4.50,COMPLETED", csv)

    def test_dry_run_validates_scenario_schema_without_side_effects(self):
        from archival_regression.campaign import dry_run_validate
        from archival_regression.scenario import load_scenario
        cfg = load_scenario(ROOT / 'scenarios/blend-config-admin.json')
        result = dry_run_validate(cfg)
        self.assertEqual(result['status'], 'VALID')
        self.assertGreaterEqual(result['entries_count'], 1)

        with self.assertRaises(Inconclusive):
            dry_run_validate("not-a-dict")

    def test_html_summary_report_export(self):
        from archival_regression.campaign import export_html_report
        report = {
            "result": "PASS",
            "reason": "All verifications succeeded",
            "phase": "verify",
            "checkpoints": [
                {"ledger": 100, "live_until_ledger": 500, "archived": False},
                {"ledger": 200, "live_until_ledger": 600, "archived": True},
            ],
        }
        html = export_html_report(report)
        self.assertIn("<!DOCTYPE html>", html)
        self.assertIn("Archival Regression Report - verify", html)
        self.assertIn("badge-PASS", html)
        self.assertIn("All verifications succeeded", html)
        self.assertIn("<td>100</td><td>500</td><td>No</td>", html)
        self.assertIn("<td>200</td><td>600</td><td>Yes</td>", html)

if __name__=='__main__':unittest.main()

