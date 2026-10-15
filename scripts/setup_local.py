#!/usr/bin/env python3
"""Provision a fresh, isolated pinned Quickstart network and the two proven fixtures.

Local development only. Never resets an existing container or network. Generated
signing identities are private and stay under the ignored output directory.
"""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
IMAGE = 'stellar/quickstart@sha256:4f8dfd693deba57fddef7f0bc722e4782c2e606c8675e3adc5438cebae249295'
PASSPHRASE = 'Standalone Network ; February 2017'
ARTIFACTS = {
    'pool': '274d85836c4e9adf8995f9240c6452cb0ce2e4751e1b10baffffdbfb90319ba1',
    'pool_factory': 'e304170e67184748839dda5fd3b6dd5d8bf1e54055a07b536cede24da072c7a9',
    'tansu': '7962f42df518b6a24289d938b3f7e2f2a45d5d058a7781d04c9a1d1e86fc3cb6',
}


def configure(template, replacements, config_dir):
    """Fixture adapter only: replace provisioning values, preserve engine schema."""
    def visit(value):
        if isinstance(value, str):
            for old, new in replacements.items():
                value = value.replace(old, new)
            return value
        if isinstance(value, list):
            result = [visit(item) for item in value]
            if result and result[0] == 'stellar':
                result[1:1] = ['--config-dir', str(config_dir)]
            return result
        if isinstance(value, dict):
            return {key: visit(item) for key, item in value.items()}
        return value
    return visit(template)


class Setup:
    def __init__(self, args):
        self.args = args
        self.output = args.output.resolve()
        self.config_dir = self.output / 'stellar'
        self.base = f'http://localhost:{args.port}'
        self.net = ['--rpc-url', self.base + '/rpc', '--network-passphrase', PASSPHRASE]
        self.records = []

    def save(self):
        (self.output / 'setup-evidence.json').write_text(json.dumps(self.records, indent=2) + '\n')

    def command(self, argv, stdin=None, private=False, timeout=180):
        p = subprocess.run(argv, input=stdin, capture_output=True, text=True, timeout=timeout)
        self.records.append({'argv': argv, 'stdin': None if private else stdin,
                             'exit_code': p.returncode,
                             'stdout': '[private identity output omitted]' if private else p.stdout,
                             'stderr': '[private identity output omitted]' if private else p.stderr})
        self.save()
        if p.returncode:
            raise RuntimeError('command failed: ' + ' '.join(argv) + ('' if private else ': ' + p.stderr))
        return p.stdout.strip()

    def cli(self, *argv, **kwargs):
        return self.command(['stellar', '--config-dir', str(self.config_dir), *argv], **kwargs)

    def http(self, path, payload=None):
        data = None if payload is None else json.dumps(payload).encode()
        request = urllib.request.Request(self.base + path, data=data,
                                         headers={'Content-Type': 'application/json'})
        with urllib.request.urlopen(request, timeout=15) as response:
            result = json.load(response)
        self.records.append({'url': self.base + path, 'request': payload, 'response': result})
        self.save()
        return result

    def rpc(self, method, params=None):
        body = self.http('/rpc', {'jsonrpc': '2.0', 'id': 1, 'method': method, 'params': params or {}})
        if 'error' in body:
            raise RuntimeError(str(body['error']))
        return body['result']

    def encode(self, typ, value):
        return self.cli('xdr', 'encode', '--type', typ, stdin=json.dumps(value))

    def decode(self, typ, value):
        return json.loads(self.cli('xdr', 'decode', '--type', typ, '--input', 'single-base64', value))

    def profile(self, admin):
        key = self.encode('LedgerKey', {'config_setting': {'config_setting_id': 'state_archival'}})
        def read():
            result = self.rpc('getLedgerEntries', {'keys': [key]})
            return self.decode('LedgerEntryData', result['entries'][0]['xdr'])['config_setting']['state_archival']
        settings = read()
        settings.update(min_persistent_ttl=64, min_temporary_ttl=16, starting_eviction_scan_level=1)
        xdr = self.encode('ConfigUpgradeSet', {'updated_entry': [{'state_archival': settings}]})
        account = self.http('/accounts/' + admin)
        lines = self.command(['docker', 'exec', self.args.container, 'stellar-core',
                              'get-settings-upgrade-txs', admin, account['sequence'], PASSPHRASE,
                              '--xdr', xdr]).splitlines()
        for i in range(0, len(lines) - 1, 2):
            signed = self.cli('tx', 'sign', *self.net, '--sign-with-key', 'statecycle-admin', stdin=lines[i])
            self.cli('tx', 'send', *self.net, stdin=signed)
            if self.rpc('getTransaction', {'hash': lines[i+1]})['status'] != 'SUCCESS':
                raise RuntimeError('configuration upgrade transaction unsuccessful')
        upgrade = self.decode('ConfigUpgradeSetKey', lines[-1])
        payload_key = self.encode('LedgerKey', {'contract_data': {
            'contract': upgrade['contract_id'], 'key': {'bytes': upgrade['content_hash']},
            'durability': 'temporary'}})
        if not self.rpc('getLedgerEntries', {'keys': [payload_key]})['entries']:
            self.cli('contract', 'invoke', '--id', upgrade['contract_id'], *self.net,
                     '--source', 'statecycle-admin', '--', 'write', '--xdr_bytes', base64.b64decode(xdr).hex())
        for _ in range(10):
            result = self.command(['docker', 'exec', self.args.container, 'curl', '-sSG',
                'http://localhost:11626/upgrades?mode=set&upgradetime=1970-01-01T00:00:00Z',
                '--data-urlencode', 'configupgradesetkey=' + lines[-1]])
            if 'Error' not in result:
                break
            time.sleep(1)
        else:
            raise RuntimeError('Core rejected configuration upgrade')
        for _ in range(30):
            if read() == settings:
                return settings
            time.sleep(1)
        raise RuntimeError('Core configuration readback did not match')

    def run(self):
        if self.output.exists():
            raise RuntimeError('refusing existing setup output; use a fresh --output and container')
        self.output.mkdir(parents=True)
        version = self.command(['stellar', '--version'])
        if not version.startswith('stellar 28.1.0'):
            raise RuntimeError('Stellar CLI 28.1.0 required')
        if not re.fullmatch(r'[a-zA-Z0-9][a-zA-Z0-9_.-]+', self.args.container):
            raise RuntimeError('invalid container name')
        existing = self.command(['docker', 'ps', '-a', '--format', '{{.Names}}']).splitlines()
        if self.args.container in existing:
            raise RuntimeError('refusing an existing container; no network reset is performed')
        self.command(['docker', 'run', '--detach', '--name', self.args.container,
                      '--publish', f'127.0.0.1:{self.args.port}:8000', IMAGE,
                      '--local', '--enable', 'rpc,horizon'])
        deadline = time.monotonic() + 180
        while True:
            try:
                if self.rpc('getHealth')['status'] == 'healthy':
                    break
            except (OSError, ValueError, RuntimeError):
                pass
            if time.monotonic() >= deadline:
                raise RuntimeError('Quickstart readiness timeout')
            time.sleep(2)
        network = self.rpc('getNetwork')
        if network['passphrase'] != PASSPHRASE or network['protocolVersion'] != 28:
            raise RuntimeError('unsupported network profile')
        self.command(['docker', 'exec', self.args.container, 'stellar-core', 'version'])
        addresses = {}
        for role in ('admin', 'oracle', 'backstop'):
            self.cli('keys', 'generate', 'statecycle-' + role, private=True)
            addresses[role] = self.cli('keys', 'address', 'statecycle-' + role)
        # RPC becomes healthy before Horizon/Friendbot have finished starting.
        for attempt in range(60):
            try:
                self.http('/friendbot?addr=' + addresses['admin'])
                break
            except OSError:
                if attempt == 59:
                    raise
                time.sleep(2)
        source = ['--source', 'statecycle-admin']
        for name, expected in ARTIFACTS.items():
            wasm = ROOT / 'docs/experiments/stage2/artifacts' / (name + '.wasm')
            if hashlib.sha256(wasm.read_bytes()).hexdigest() != expected:
                raise RuntimeError('fixture WASM hash mismatch: ' + name)
            if self.cli('contract', 'upload', '--wasm', str(wasm), *self.net, *source) != expected:
                raise RuntimeError('uploaded hash mismatch: ' + name)
        factory = self.cli('contract', 'deploy', '--wasm-hash', ARTIFACTS['pool_factory'],
                           *self.net, *source, '--', '--pool_init_meta', json.dumps({
                            'pool_hash': ARTIFACTS['pool'], 'backstop': addresses['backstop'],
                            'blnd_id': addresses['oracle']})).strip('"')
        sac = self.cli('contract', 'asset', 'deploy', '--asset', 'ARST2:' + addresses['admin'],
                       *self.net, *source).strip('"')
        profile = self.profile(addresses['admin'])
        replacements = {
            'http://localhost:8000/rpc': self.base + '/rpc',
            'archival-admin': 'statecycle-admin',
            'GBPQSQQRS34EPU6QF3RLGHIHA7NQHNYQ4UUUSIYR6T4F5FLLIHQZMQEN': addresses['admin'],
            'GDIGJ6OENHTHPP6TZA5C5NV2D5JD5TEHIZKWT6OYTUP7YSB4ZHVKU7HJ': addresses['oracle'],
            'GDTR7BTCCRJOKFNVR4UARUUITZQM7MPVFQTOXO233ZD2TZPZOYUVXBKL': addresses['backstop'],
            'CA3JX42W32345O6AIUO3GJGBXHP2VZCAHQBZOC3RKJ6QAVSNG2VBKMYC': factory,
            'CADGP5CFFEKXJLYFXHS7R5EPSMEBUC5HJBK6SKAYXA6KWDER6QMBLOPG': sac,
            'Stage 1 repetition 1': 'Soroban StateCycle',
        }
        scenarios = self.output / 'scenarios'
        scenarios.mkdir()
        for path in sorted((ROOT / 'scenarios').glob('*.json')):
            config = configure(json.loads(path.read_text()), replacements, self.config_dir)
            (scenarios / path.name).write_text(json.dumps(config, indent=2) + '\n')
        result = {'result': 'PASS', 'meaning': 'setup only; no archival claim',
                  'image': IMAGE, 'network': network, 'profile': profile,
                  'addresses': addresses, 'factory': factory, 'sac': sac,
                  'scenarios': str(scenarios), 'python': sys.version}
        (self.output / 'setup-result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port', type=int, default=8000)
    parser.add_argument('--container', default='statecycle-local')
    parser.add_argument('--output', type=Path, default=ROOT / '.local/statecycle')
    args = parser.parse_args()
    if not 1024 <= args.port <= 65535:
        parser.error('port must be in 1024..65535')
    try:
        Setup(args).run()
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(json.dumps({'result': 'INCONCLUSIVE', 'reason': str(error)}))
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main())
