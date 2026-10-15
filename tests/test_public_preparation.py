"""Behavior checks for fixture adaptation and read-only evidence integrity."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def module(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / (name + '.py'))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


class PublicPreparationTests(unittest.TestCase):
    def test_setup_adapts_nested_argv_without_changing_input_or_salt(self):
        setup = module('setup_local')
        template = {'hooks': {'setup': [['stellar', 'invoke', 'OLD', '{id}', '{salt}']]}}
        before = copy.deepcopy(template)
        adapted = setup.configure(template, {'OLD': 'NEW'}, Path('/local/config'))
        self.assertEqual(template, before)
        self.assertEqual(adapted['hooks']['setup'][0],
                         ['stellar', '--config-dir', '/local/config', 'invoke', 'NEW', '{id}', '{salt}'])

    def test_both_fixture_templates_remain_valid_after_adaptation(self):
        setup = module('setup_local')
        from archival_regression.scenario import validate
        for path in (ROOT / 'scenarios').glob('*.json'):
            adapted = setup.configure(json.loads(path.read_text()),
                                      {'http://localhost:8000/rpc': 'http://localhost:18000/rpc'},
                                      Path('/local/config'))
            validate(adapted)
            self.assertEqual(adapted['rpc_url'], 'http://localhost:18000/rpc')

    def test_integrity_refuses_missing_changed_and_added_files(self):
        integrity = module('verify_evidence')
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for folder in integrity.RETAINED:
                (root / folder).mkdir(parents=True, exist_ok=True)
            for name in ('docs/stage1-flagship.md', 'docs/stage2-reusability.md'):
                (root / name).write_text('retained\n')
            expected = {name: hashlib.sha256((root / name).read_bytes()).hexdigest()
                        for name in integrity.retained_files(root)}
            (root / 'docs/evidence-integrity.json').write_text(json.dumps({
                'schema': 'statecycle/retained-evidence-v1', 'files': expected}))
            self.assertEqual(integrity.verify(root), 2)
            file = root / 'docs/stage1-flagship.md'
            file.write_text('modified')
            with self.assertRaises(ValueError):
                integrity.verify(root)
            file.unlink()
            with self.assertRaises((ValueError, OSError)):
                integrity.verify(root)
            file.write_text('retained\n')
            (root / integrity.RETAINED[0] / 'extra.json').write_text('{}')
            with self.assertRaises(ValueError):
                integrity.verify(root)

    def test_current_checkout_sanity(self):
        self.assertEqual(module('check_reproducibility').check()['result'], 'PASS')
