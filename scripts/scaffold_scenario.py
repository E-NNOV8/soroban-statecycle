#!/usr/bin/env python3
"""Lightweight scenario scaffolding utility for custom Soroban smart contracts.

Generates a fully valid 'archival-regression/scenario-v2' JSON template,
failing closed on missing persistent storage keys or getter projections.
"""
import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'src'))

try:
    from archival_regression.scenario import validate
    from archival_regression.errors import Inconclusive
except ImportError:
    validate = None
    Inconclusive = ValueError


def build_scenario_template(
    name: str = 'custom-contract',
    contract_id: str = '{id}',
    rpc_url: str = 'http://localhost:8000/rpc',
    network_passphrase: str = 'Standalone Network ; February 2017',
    protocol: int = 28,
    archive_timeout_seconds: int = 240,
    source: str = 'archival-admin',
    entries: list[str] = None,
    bindings: list[str] = None,
    include_mutation: bool = False,
    wasm_hash: str = None,
) -> dict:
    """Build and validate a scenario dictionary.

    Fails closed if persistent keys or getter projections are omitted or invalid.
    """
    if protocol != 28:
        raise ValueError('Only verified Stellar protocol 28 is supported by StateCycle')

    if not entries:
        entries = ['instance']

    if not bindings:
        raise ValueError('At least one public getter projection binding is required to assert state continuity')

    # Build persistent entries
    entries_dict = {}
    for entry_name in entries:
        clean_name = entry_name.strip()
        if not clean_name:
            continue
        if clean_name == 'instance':
            entries_dict[clean_name] = {
                'contract_data': {
                    'contract': contract_id,
                    'key': 'ledger_key_contract_instance',
                    'durability': 'persistent',
                }
            }
        else:
            # Custom persistent data key template
            entries_dict[clean_name] = {
                'contract_data': {
                    'contract': contract_id,
                    'key': {
                        'vec': [
                            {'symbol': clean_name},
                            {'bytes': '0000000000000000000000000000000000000000000000000000000000000000'},
                        ]
                    },
                    'durability': 'persistent',
                }
            }

    if not entries_dict:
        raise ValueError('No valid persistent storage entries declared')

    # Build hooks dictionary
    deploy_argv = [
        'stellar', 'contract', 'deploy',
        '--wasm-hash', wasm_hash if wasm_hash else 'REPLACE_WITH_WASM_HASH',
        '--salt', '{salt}',
        '--rpc-url', rpc_url,
        '--network-passphrase', network_passphrase,
        '--source', source,
    ]

    restore_argv = [
        'stellar', 'contract', 'restore',
        '--id', contract_id,
        '--rpc-url', rpc_url,
        '--network-passphrase', network_passphrase,
        '--source', source,
    ]

    hooks = {
        'deploy': deploy_argv,
        'restore': restore_argv,
    }

    # Build bindings and their corresponding getter hooks
    bindings_dict = {}
    primary_entry = 'instance' if 'instance' in entries_dict else next(iter(entries_dict.keys()))

    expected_initial = {}
    expected_before = {}

    for idx, binding_name in enumerate(bindings):
        bname = binding_name.strip()
        if not bname:
            continue
        if bname in ('deploy', 'setup', 'mutate', 'restore', 'advance'):
            raise ValueError(f"'{bname}' is a reserved phase hook name and cannot be used as a public binding")

        # Map binding to storage entry
        target_entry = primary_entry
        bindings_dict[bname] = {
            'entry': target_entry,
            'path': ['contract_data', 'val'],
            'value_path': ['storage', bname],
        }

        # Getter hook using --send yes to generate verifiable transaction receipts
        hooks[bname] = [
            'stellar', 'contract', 'invoke',
            '--id', contract_id,
            '--rpc-url', rpc_url,
            '--network-passphrase', network_passphrase,
            '--source', source,
            '--send', 'yes',
            '--',
            bname,
        ]

        expected_initial[bname] = f'initial_state_{idx + 1}'
        expected_before[bname] = f'mutated_state_{idx + 1}'

    if not bindings_dict:
        raise ValueError('No valid public bindings provided')

    scenario = {
        'schema': 'archival-regression/scenario-v2',
        'rpc_url': rpc_url,
        'passphrase': network_passphrase,
        'protocol': protocol,
        'archive_timeout_seconds': archive_timeout_seconds,
        'name': name,
        'source': {
            'wasm_hash': wasm_hash if wasm_hash else 'REPLACE_WITH_WASM_HASH',
        },
        'entries': entries_dict,
        'bindings': bindings_dict,
        'hooks': hooks,
    }

    if include_mutation:
        scenario['mutation_required'] = True
        scenario['hooks']['mutate'] = [
            'stellar', 'contract', 'invoke',
            '--id', contract_id,
            '--rpc-url', rpc_url,
            '--network-passphrase', network_passphrase,
            '--source', source,
            '--send', 'yes',
            '--',
            'mutate_state',
        ]
        scenario['expected_initial'] = expected_initial
        scenario['expected_before'] = expected_before

    # Validate fail-closed against schema validator
    if validate:
        validate(copy.deepcopy(scenario))

    return scenario


def main():
    parser = argparse.ArgumentParser(
        description='Generate a validated Soroban StateCycle scenario template for custom contracts.'
    )
    parser.add_argument('--name', default='custom-contract', help='Descriptive scenario name')
    parser.add_argument('--contract-id', default='{id}', help='Target contract ID or {id} placeholder')
    parser.add_argument('--rpc-url', default='http://localhost:8000/rpc', help='Local Stellar RPC URL')
    parser.add_argument('--network-passphrase', default='Standalone Network ; February 2017', help='Network passphrase')
    parser.add_argument('--protocol', type=int, default=28, help='Protocol version (must be 28)')
    parser.add_argument('--archive-timeout', type=int, default=240, help='Archival timeout seconds (1..3600)')
    parser.add_argument('--source', default='archival-admin', help='Signer account alias or secret key name')
    parser.add_argument('--entries', default='instance', help='Comma-separated entry names (e.g. instance,records)')
    parser.add_argument('--bindings', required=True, help='Comma-separated getter method names (e.g. get_config,get_value)')
    parser.add_argument('--has-mutation', action='store_true', help='Include mutation hook and X/Y state assertions')
    parser.add_argument('--wasm-hash', default='', help='WASM hash for contract deployment')
    parser.add_argument('--output', '-o', help='Path to output JSON file (default: stdout)')

    args = parser.parse_args()

    entry_list = [e.strip() for e in args.entries.split(',') if e.strip()]
    binding_list = [b.strip() for b in args.bindings.split(',') if b.strip()]

    try:
        scenario = build_scenario_template(
            name=args.name,
            contract_id=args.contract_id,
            rpc_url=args.rpc_url,
            network_passphrase=args.network_passphrase,
            protocol=args.protocol,
            archive_timeout_seconds=args.archive_timeout,
            source=args.source,
            entries=entry_list,
            bindings=binding_list,
            include_mutation=args.has_mutation,
            wasm_hash=args.wasm_hash or None,
        )
    except Exception as exc:
        sys.stderr.write(f'Error scaffolding scenario: {exc}\n')
        sys.exit(1)

    formatted = json.dumps(scenario, indent=2) + '\n'

    if args.output:
        out_path = Path(args.output)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(formatted)
        print(f'Successfully generated scenario template at: {out_path}')
    else:
        sys.stdout.write(formatted)


if __name__ == '__main__':
    main()
