"""Offline revalidation of retained real second-target Core evidence."""
import copy
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from archival_regression.campaign import Campaign,Inconclusive,Violation,compare
from archival_regression.scenario import load_scenario

ROOT=Path(__file__).resolve().parents[1]
EVIDENCE=ROOT/'docs/experiments/stage2'

@unittest.skipUnless(shutil.which('stellar'),'CLI XDR decoding unavailable; live evidence is not replaced')
class Stage2EvidenceTests(unittest.TestCase):
    def copied(self,directory,name='tansu-live'):
        source=EVIDENCE/name
        state=json.loads((source/'checkpoint.json').read_text())
        config=json.loads((source/'config.json').read_text())
        target=Path(directory)
        (target/'checkpoint.json').write_text(json.dumps(state))
        references={w['rpc_evidence'] for w in state['archive_witnesses'].values()}
        references.update(t['evidence'] for t in state['restore_batches'][0])
        references.update(state[k] for k in ['raw_before_evidence','raw_after_evidence'])
        for name in references:(target/name).write_bytes((source/name).read_bytes())
        return Campaign(config,target)

    def test_both_real_scenarios_use_identical_verifier(self):
        for name in ['blend-live','tansu-live']:
            with self.subTest(name=name),tempfile.TemporaryDirectory() as folder:
                self.copied(folder,name).verify_recovery()

    def test_mutated_commit_is_independently_present_in_raw_state(self):
        s=json.loads((EVIDENCE/'tansu-live/checkpoint.json').read_text())
        config=load_scenario(ROOT/'scenarios/tansu-project-commit.json')
        self.assertNotEqual(s['initial'],s['before'])
        self.assertEqual(s['after'],config['expected_before'])
        compare(s['raw_initial'],s['initial'],config['bindings'])
        compare(s['raw_after'],s['after'],config['bindings'])
        self.assertNotEqual(s['raw_initial']['commit'],s['raw_after']['commit'])
        self.assertEqual(s['raw_before'],s['raw_after'])

    def test_missing_one_of_three_archive_witnesses_is_inconclusive(self):
        with tempfile.TemporaryDirectory() as folder:
            c=self.copied(folder)
            del c.state['archive_witnesses']['commit']
            with self.assertRaises(Inconclusive):c.validate_archive()

    def test_missing_archival_metadata_cannot_be_checkpoint_proof(self):
        with tempfile.TemporaryDirectory() as folder:
            c=self.copied(folder)
            (Path(folder)/c.state['archive_witnesses']['commit']['rpc_evidence']).unlink()
            with self.assertRaises(OSError):c.validate_archive()

    def test_missing_post_restore_raw_receipt_is_inconclusive(self):
        with tempfile.TemporaryDirectory() as folder:
            c=self.copied(folder)
            (Path(folder)/c.state['raw_after_evidence']).unlink()
            with self.assertRaises(OSError):c.verify_recovery()

    def test_wrong_mutable_value_is_a_violation(self):
        with tempfile.TemporaryDirectory() as folder:
            c=self.copied(folder)
            c.state['after']=copy.deepcopy(c.state['initial'])
            with self.assertRaises(Violation):c.verify_recovery()

    def test_restore_receipts_must_cover_all_named_entries(self):
        with tempfile.TemporaryDirectory() as folder:
            c=self.copied(folder)
            c.state['restore_batches'][0]=c.state['restore_batches'][0][:2]
            with self.assertRaises(Inconclusive):c.verify_recovery()

    def test_negative_record_is_actual_unrestored_availability_observation(self):
        s=json.loads((EVIDENCE/'tansu-live/checkpoint.json').read_text())
        r=json.loads((EVIDENCE/'tansu-live'/s['negative_observation']['rpc_evidence']).read_text())
        result=json.loads(r['body'])['result']
        self.assertGreaterEqual(result['latestLedger'],max(w['ledger'] for w in s['archive_witnesses'].values()))
        for entry in result['entries']:
            self.assertLess(entry['liveUntilLedgerSeq'],result['latestLedger'])
        self.assertEqual(len(s['negative_observation']['required_keys_unavailable_live']),3)

if __name__=='__main__':unittest.main()
