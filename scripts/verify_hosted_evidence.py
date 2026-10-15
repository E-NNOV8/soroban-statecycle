#!/usr/bin/env python3
"""Verify retained hosted provenance, archive integrity and recovered raw state offline."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import stat
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DEFAULT = ROOT / 'docs/hosted-proof/20261015'
sys.path.insert(0, str(ROOT / 'src'))
from archival_regression.campaign import same


def member(root, name):
    if not isinstance(name, str) or not name or PurePosixPath(name).is_absolute() or '..' in PurePosixPath(name).parts:
        raise ValueError('unsafe retained evidence path')
    path = root / name
    if path.is_symlink() or not path.resolve().is_relative_to(root.resolve()):
        raise ValueError('retained evidence path escapes bundle')
    return path


def validate_bundle(root):
    manifest = json.loads((root / 'manifest.json').read_text())
    if manifest['schema'] != 'statecycle/hosted-evidence-v1' or not manifest['files']:
        raise ValueError('unsupported or empty hosted manifest')
    producer = manifest['producer_sha']
    if not isinstance(producer, str) or len(producer) != 40 or any(c not in '0123456789abcdef' for c in producer):
        raise ValueError('invalid producer SHA')
    actual = {p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file()} - {'manifest.json'}
    if actual != set(manifest['files']):
        raise ValueError('hosted evidence file set differs from manifest')
    for name, digest in manifest['files'].items():
        path = member(root, name)
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise ValueError('hosted evidence checksum mismatch: ' + name)
        if path.suffix == '.json':
            json.loads(path.read_text())
    for run in manifest['runs']:
        data = json.loads(member(root, run['metadata']).read_text())
        jobs = json.loads(member(root, run['jobs']).read_text())['jobs']
        if (data['id'] != run['id'] or data['head_sha'] != producer or data['status'] != 'completed'
                or data['conclusion'] != 'success' or data['repository']['full_name'] != manifest['repository']
                or not jobs or any(j['run_id'] != run['id'] or j['head_sha'] != producer
                                   or j['conclusion'] != 'success' for j in jobs)):
            raise ValueError('hosted run/job provenance or result mismatch')
    artifact = manifest['artifact']
    metadata = json.loads(member(root, artifact['metadata']).read_text())['artifacts']
    matches = [a for a in metadata if a['id'] == artifact['id']]
    digest = 'sha256:' + hashlib.sha256(member(root, artifact['archive']).read_bytes()).hexdigest()
    if (len(matches) != 1 or matches[0]['digest'] != artifact['digest'] or artifact['retained_digest'] != digest
            or matches[0]['workflow_run']['head_sha'] != producer
            or matches[0]['workflow_run']['id'] != artifact['run_id']
            or artifact['run_id'] not in {r['id'] for r in manifest['runs']}):
        raise ValueError('artifact digest/producer mismatch')
    # Every application proof member retains its original hosted bytes. Only
    # disclosed non-proof Docker diagnostics may differ from original hashes.
    redactions = {r['member']: r for r in artifact['redactions']}
    if set(redactions) != {'setup/docker.log'}:
        raise ValueError('unexpected hosted evidence redaction')
    with zipfile.ZipFile(member(root, artifact['archive'])) as bundle:
        if set(bundle.namelist()) != set(artifact['original_members']):
            raise ValueError('hosted artifact member set mismatch')
        for name, original_hash in artifact['original_members'].items():
            retained_hash = hashlib.sha256(bundle.read(name)).hexdigest()
            if name in redactions:
                if (redactions[name]['original_sha256'] != original_hash
                        or redactions[name]['retained_sha256'] != retained_hash):
                    raise ValueError('diagnostic redaction hash mismatch')
            elif retained_hash != original_hash:
                raise ValueError('hosted proof member changed: ' + name)
    if not manifest['cases'] or len({c['directory'] for c in manifest['cases']}) != len(manifest['cases']):
        raise ValueError('missing/duplicate hosted campaign cases')
    return manifest


def extract(archive, destination):
    with zipfile.ZipFile(archive) as bundle:
        names = set()
        if sum(i.file_size for i in bundle.infolist()) > 128 * 1024 * 1024:
            raise ValueError('hosted archive exceeds uncompressed size bound')
        for item in bundle.infolist():
            member(destination, item.filename)
            if item.filename in names or stat.S_ISLNK(item.external_attr >> 16):
                raise ValueError('duplicate/symlink hosted archive member')
            names.add(item.filename)
        bundle.extractall(destination)


def verify(root=DEFAULT):
    manifest = validate_bundle(root)
    if not shutil.which('stellar'):
        raise ValueError('Stellar CLI required; hosted raw replay cannot be skipped')
    results = []
    with tempfile.TemporaryDirectory(prefix='statecycle-hosted-proof-') as temp:
        target = Path(temp)
        extract(member(root, manifest['artifact']['archive']), target)
        for case in manifest['cases']:
            evidence = member(target, case['directory'])
            state = json.loads((evidence / 'checkpoint.json').read_text())
            config = json.loads((evidence / 'config.json').read_text())
            runner = json.loads((evidence / 'runner.json').read_text())
            result = json.loads((evidence / 'campaign-result.json').read_text())
            if (result['result'] != 'PASS' or state['phase'] != 'verified'
                    or [r['phase'] for r in runner] != ['archive', 'negative', 'restore-checkpoint', 'resume-retry', 'verify']
                    or [r['exit_code'] for r in runner] != [2, 1, 2, 0, 0]
                    or json.loads(runner[1]['stdout'])['reason'] != 'required explicit restoration omitted'
                    or len(state['restore_batches']) < 2
                    or not same(state['before'], state['control']) or not same(state['before'], state['after'])):
                raise ValueError('incomplete hosted lifecycle: ' + case['name'])
            if config.get('mutation_required') and (same(state['initial'], state['before'])
                    or not same(state['before'], config['expected_before']) or not same(state['initial'], config['expected_initial'])):
                raise ValueError('hosted X/Y mutation assertion mismatch')
            process = subprocess.run([sys.executable, '-m', 'archival_regression.campaign',
                                      '--config', str(evidence / 'config.json'), '--output', str(evidence),
                                      '--phase', 'verify'], cwd=ROOT,
                                     env={**os.environ, 'PYTHONPATH': str(ROOT / 'src')},
                                     capture_output=True, text=True, timeout=120)
            if process.returncode != 0 or json.loads(process.stdout)['result'] != 'PASS':
                raise ValueError('hosted raw recovery replay failed: ' + case['name'] + ': ' + process.stdout)
            results.append(case['name'])
    return {'result': 'PASS', 'producer_sha': manifest['producer_sha'], 'campaigns': results,
            'scope': 'retained hosted provenance/integrity and offline recovery; no new live campaign'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bundle', type=Path, default=DEFAULT)
    args = parser.parse_args()
    try:
        print(json.dumps(verify(args.bundle)))
    except (OSError, ValueError, KeyError, TypeError, IndexError, AttributeError,
            zipfile.BadZipFile, subprocess.TimeoutExpired) as error:
        print(json.dumps({'result': 'INCONCLUSIVE', 'reason': str(error)}))
        sys.exit(2)
