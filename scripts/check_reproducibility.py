#!/usr/bin/env python3
"""Lightweight, dependency-free checkout sanity; no live archival claim."""
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path[:0] = [str(ROOT / 'src'), str(ROOT / 'scripts')]
from archival_regression.scenario import load_scenario
from setup_local import ARTIFACTS, IMAGE
import hashlib


def check():
    if sys.version_info < (3, 12):
        raise ValueError('Python 3.12+ required')
    for name in ('README.md', 'LICENSE', 'VERSION', 'THIRD_PARTY_NOTICES.md',
                 'docs/scenarios.md', 'docs/reproducibility.md', 'docs/evidence.md',
                 'scripts/check.sh', 'scripts/install_stellar.sh', 'scripts/verify_evidence.py'):
        if not (ROOT / name).is_file():
            raise ValueError('missing public checkout file: ' + name)
    if (ROOT / 'VERSION').read_text().strip() != '0.1.0-alpha':
        raise ValueError('unexpected initial version')
    readme = (ROOT / 'README.md').read_text()
    if 'Drips' in readme or 'soroban-archival-regression' in readme:
        raise ValueError('stale/internal README naming')
    for path in (ROOT / 'scenarios').glob('*.json'):
        load_scenario(path)
    for name, expected in ARTIFACTS.items():
        artifact = ROOT / 'docs/experiments/stage2/artifacts' / (name + '.wasm')
        if hashlib.sha256(artifact.read_bytes()).hexdigest() != expected:
            raise ValueError('fixture hash mismatch: ' + name)
    for name in ('engine-tests.yml', 'local-integration.yml'):
        workflow = (ROOT / '.github/workflows' / name).read_text()
        if 'runs-on: ubuntu-24.04' not in workflow or "python-version: '3.12'" not in workflow:
            raise ValueError('workflow profile drift: ' + name)
    if len(IMAGE.split('@sha256:')[-1]) != 64:
        raise ValueError('Quickstart image not pinned')
    return {'result': 'PASS', 'scope': 'checkout/configuration sanity; no live or hosted CI claim'}


if __name__ == '__main__':
    try:
        print(json.dumps(check()))
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(json.dumps({'result': 'INCONCLUSIVE', 'reason': str(error)}))
        sys.exit(2)
