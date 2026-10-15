#!/usr/bin/env python3
"""Read-only integrity check and offline replay of retained Core-backed evidence."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
RETAINED = ('docs/experiments/stage1', 'docs/experiments/stage2',
            'docs/history/stage2-source', 'docs/vendor/blend', 'docs/licenses',
            'docs/experiments/public-preparation-20261015')


def retained_files(root):
    files = {p.relative_to(root).as_posix() for folder in RETAINED
             for p in (root / folder).rglob('*')
             if p.is_file() and '__pycache__' not in p.parts and p.suffix != '.pyc'}
    files.update(('docs/stage1-flagship.md', 'docs/stage2-reusability.md'))
    return files


def verify(root=ROOT):
    manifest = json.loads((root / 'docs/evidence-integrity.json').read_text())
    expected = manifest['files']
    if manifest['schema'] != 'statecycle/retained-evidence-v1' or not expected:
        raise ValueError('invalid or empty integrity manifest')
    if set(expected) != retained_files(root):
        raise ValueError('retained file set differs from manifest')
    for name, digest in expected.items():
        path = root / name
        if path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise ValueError('retained bytes differ: ' + name)
        if path.suffix == '.json':
            json.loads(path.read_text())
    # Verify historical manifest references against immutable retained files or
    # source snapshots, not the renamed/current README and active source tree.
    known_hashes = set(expected.values())
    for sums in (root / 'docs/experiments').rglob('SHA256SUMS'):
        for line in sums.read_text().splitlines():
            digest, name = line.split(None, 1)
            name = name.strip()
            if name.startswith('docs/experiments/'):
                if expected.get(name) != digest:
                    raise ValueError('historical checksum mismatch: ' + name)
            elif name != 'README.md' and digest not in known_hashes:
                current = root / name
                if not current.is_file() or hashlib.sha256(current.read_bytes()).hexdigest() != digest:
                    raise ValueError('historical source snapshot missing: ' + name)
    # Check every checkpoint reference, independently of recorded PASS labels.
    for checkpoint in (root / 'docs/experiments').rglob('checkpoint.json'):
        state = json.loads(checkpoint.read_text())
        def references(value):
            if isinstance(value, dict):
                for key, item in value.items():
                    if (key.endswith('_evidence') or key == 'evidence') and isinstance(item, str):
                        p = Path(item)
                        if p.is_absolute() or '..' in p.parts or not (checkpoint.parent / p).is_file():
                            raise ValueError('missing/unsafe checkpoint receipt: ' + item)
                    references(item)
            elif isinstance(value, list):
                for item in value:
                    references(item)
        references(state)
    return len(expected)


def replay(root=ROOT):
    if not shutil.which('stellar'):
        raise ValueError('Stellar CLI required; offline XDR replay cannot be skipped')
    base = root / 'docs/experiments'
    cases = [(base / 'stage2' / name, base / 'stage2' / (name + '.scenario.json'))
             for name in ('blend-live', 'tansu-live')]
    old = base / 'stage1/host-resume-20261015'
    cases += [(old / f'run-{i}', old / f'run-{i}-config.json') for i in range(1, 6)]
    for source, config in cases:
        with tempfile.TemporaryDirectory(prefix='statecycle-evidence-') as temp:
            target = Path(temp) / 'evidence'
            shutil.copytree(source, target, ignore=shutil.ignore_patterns('__pycache__'))
            process = subprocess.run([sys.executable, '-m', 'archival_regression.campaign',
                                      '--config', str(config), '--output', str(target),
                                      '--phase', 'verify'], cwd=root,
                                     env={**os.environ, 'PYTHONPATH': str(root / 'src')},
                                     capture_output=True, text=True)
            if process.returncode != 0:
                raise ValueError(f'offline evidence replay failed: {source.name}: {process.stdout} {process.stderr}')
    return len(cases)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--hashes-only', action='store_true', help='integrity only, without semantic XDR replay')
    args = parser.parse_args()
    try:
        count = verify()
        replays = 0 if args.hashes_only else replay()
        print(json.dumps({'result': 'PASS', 'retained_files': count, 'offline_replays': replays}))
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(json.dumps({'result': 'INCONCLUSIVE', 'reason': str(error)}))
        sys.exit(2)
