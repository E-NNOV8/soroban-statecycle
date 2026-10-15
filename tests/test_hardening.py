"""Malformed configuration boundaries and retained hosted evidence enforcement."""
import copy
import hashlib
import importlib.util
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
import zipfile
from archival_regression.scenario import load_scenario, validate
from archival_regression.errors import Inconclusive

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('hosted', ROOT / 'scripts/verify_hosted_evidence.py')
hosted = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(hosted)


class ScenarioBoundaryTests(unittest.TestCase):
    def test_invariants_distinguish_json_boolean_and_integer(self):
        from archival_regression.campaign import compare, same
        from archival_regression.errors import Violation
        raw = {'entry': {'value': {'u32': 1}}}
        bindings = {'read': {'entry': 'entry', 'path': ['value']}}
        compare(raw, {'read': 1}, bindings)
        with self.assertRaises(Violation):
            compare(raw, {'read': True}, bindings)
        self.assertFalse(same({'nested': [1]}, {'nested': [True]}))
        self.assertTrue(same({'nested': [1]}, {'nested': [1]}))

    def test_malformed_nested_key_is_inconclusive_before_any_hook(self):
        config = load_scenario(ROOT / 'scenarios/blend-config-admin.json')
        for value in (None, [], 'bad', 42):
            with self.subTest(value=value), tempfile.TemporaryDirectory() as temp:
                config['entries']['instance']['contract_data'] = value
                path = Path(temp) / 'scenario.json'
                path.write_text(json.dumps(config))
                output = Path(temp) / 'output'
                process = subprocess.run([sys.executable, '-m', 'archival_regression', 'run', str(path), '--output', str(output)],
                    cwd=ROOT, env={**os.environ, 'PYTHONPATH': str(ROOT / 'src')}, capture_output=True, text=True)
                self.assertEqual(process.returncode, 2, process.stderr)
                self.assertEqual(json.loads(process.stdout)['result'], 'INCONCLUSIVE')
                self.assertNotIn('Traceback', process.stderr)
                self.assertFalse(output.exists())

    def test_unsupported_hook_shape_is_rejected_before_execution(self):
        for name in ('deploy', 'get_config'):
            config = load_scenario(ROOT / 'scenarios/blend-config-admin.json')
            config['hooks'][name] = [config['hooks'][name]]
            with self.assertRaisesRegex(Inconclusive, 'one argv'):
                validate(config)

    def test_wrong_field_shapes_and_incomplete_assertions_are_rejected(self):
        original = load_scenario(ROOT / 'scenarios/blend-config-admin.json')
        changes = [{'hooks': None}, {'rpc_url': []}, {'passphrase': ''}, {'archive_timeout_seconds': '240'},
                   {'mutation_required': 'false'}, {'expected_before': {}},
                   {'bindings': {'restore': original['bindings']['get_config']}},
                   {'bindings': {'get_config': {'entry': 'instance', 'path': ['contract_data'], 'value_path': None}}}]
        for change in changes:
            with self.subTest(change=change):
                config = copy.deepcopy(original); config.update(change)
                with self.assertRaises(Inconclusive):
                    validate(config)


class HostedEvidenceTests(unittest.TestCase):
    def test_retained_bundle_provenance_and_integrity(self):
        self.assertEqual(hosted.validate_bundle(hosted.DEFAULT)['producer_sha'], 'c19751db48702fe075b24da749a65a611ba3d1b6')

    def test_missing_or_changed_hosted_artifact_fails_closed(self):
        for missing in (True, False):
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as temp:
                target = Path(temp) / 'proof'; shutil.copytree(hosted.DEFAULT, target)
                path = target / 'core-artifact.sanitized.zip'
                if missing: path.unlink()
                else: path.write_bytes(b'corrupted archive')
                with self.assertRaises(ValueError): hosted.validate_bundle(target)

    def test_rehashed_metadata_cannot_claim_another_producer(self):
        with tempfile.TemporaryDirectory() as temp:
            target = Path(temp) / 'proof'; shutil.copytree(hosted.DEFAULT, target)
            path = target / 'core-run.json'; run = json.loads(path.read_text()); run['head_sha'] = 'a' * 40
            path.write_text(json.dumps(run))
            path = target / 'manifest.json'; manifest = json.loads(path.read_text())
            manifest['files']['core-run.json'] = hashlib.sha256((target / 'core-run.json').read_bytes()).hexdigest()
            path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, 'provenance'): hosted.validate_bundle(target)

    def test_unsafe_zip_members_never_escape_temporary_directory(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp); archive = root / 'unsafe.zip'; target = root / 'extracted'; target.mkdir()
            with zipfile.ZipFile(archive, 'w') as bundle: bundle.writestr('../escape.json', '{}')
            with self.assertRaisesRegex(ValueError, 'unsafe'): hosted.extract(archive, target)
            self.assertFalse((root / 'escape.json').exists())

    def test_public_artifact_contains_no_stellar_secret_keys(self):
        manifest = hosted.validate_bundle(hosted.DEFAULT)
        with zipfile.ZipFile(hosted.DEFAULT / manifest['artifact']['archive']) as bundle:
            for name in bundle.namelist():
                self.assertIsNone(re.search(rb'\bS[A-Z2-7]{55}\b', bundle.read(name)), name)

    def test_rehashed_changed_application_proof_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            target = Path(temp) / 'proof'; shutil.copytree(hosted.DEFAULT, target)
            manifest_path = target / 'manifest.json'; manifest = json.loads(manifest_path.read_text())
            name = manifest['artifact']['archive']; path = target / name
            with zipfile.ZipFile(path) as bundle:
                contents = [(i, bundle.read(i.filename)) for i in bundle.infolist()]
            with zipfile.ZipFile(path, 'w') as bundle:
                for info, data in contents:
                    bundle.writestr(info, data + b' ' if info.filename == 'blend/checkpoint.json' else data)
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            manifest['files'][name] = digest; manifest['artifact']['retained_digest'] = 'sha256:' + digest
            manifest_path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, 'proof member changed'):
                hosted.validate_bundle(target)

    def test_retained_raw_campaigns_pass_the_current_verifier(self):
        self.assertEqual(hosted.verify()['campaigns'], ['blend', 'tansu'])
