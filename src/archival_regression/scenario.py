"""Small declarative campaign model. Commands are trusted local argv hooks."""
import copy
import json
from pathlib import Path
from .errors import Inconclusive


def load_scenario(path):
    config=json.loads(Path(path).read_text())
    validate(config)
    return config


def validate(config):
    if not isinstance(config,dict) or config.get('schema')!='archival-regression/scenario-v2':
        raise Inconclusive('scenario-v2 schema required')
    for name in ('rpc_url','passphrase','protocol','hooks','entries','bindings'):
        if name not in config:
            raise Inconclusive('missing scenario field: '+name)
    if not all(isinstance(config[name], str) and config[name].strip() for name in ('rpc_url', 'passphrase')):
        raise Inconclusive('rpc_url and passphrase require nonempty strings')
    if not isinstance(config['hooks'], dict):
        raise Inconclusive('hooks requires an object')
    if not isinstance(config['protocol'],int) or config['protocol']!=28:
        raise Inconclusive('only verified protocol 28 is supported')
    if not isinstance(config['entries'],dict) or not config['entries']:
        raise Inconclusive('nonempty named ledger entries required')
    for name,key in config['entries'].items():
        if not name or not name.strip():
            raise Inconclusive('empty entry name disallowed')
        if (not isinstance(name,str) or not isinstance(key,dict) or
                set(key)!={'contract_data'} or not isinstance(key['contract_data'], dict)
                or key['contract_data'].get('durability')!='persistent'):
            raise Inconclusive('observation requires explicit persistent contract-data key')
        if (set(key['contract_data'])!={'contract','key','durability'}
                or not isinstance(key['contract_data']['contract'], str)
                or not key['contract_data']['contract']):
            raise Inconclusive('incomplete contract-data key')
    if not isinstance(config['bindings'],dict) or not config['bindings']:
        raise Inconclusive('nonempty raw/public bindings required')
    for name,binding in config['bindings'].items():
        if (not isinstance(binding,dict) or binding.get('entry') not in config['entries']
                or not isinstance(binding.get('path'),list) or not binding['path']
                or not isinstance(binding.get('value_path', []), list)
                or any(type(part) not in (str, int) for part in binding['path'] + binding.get('value_path', []))):
            raise Inconclusive('raw binding requires declared entry and path')
        if name in ('deploy', 'setup', 'mutate', 'restore', 'advance'):
            raise Inconclusive('public binding name is reserved: ' + name)
        if name not in config['hooks']:
            raise Inconclusive('public binding requires command hook')
    for phase in ('deploy','restore'):
        if phase not in config['hooks']:
            raise Inconclusive('missing hook: '+phase)
    for name,commands in config['hooks'].items():
        if not isinstance(name, str) or not name:
            raise Inconclusive('hook name requires a nonempty string')
        if name in ('deploy', *config['bindings']) and isinstance(commands, list) and commands and isinstance(commands[0], list):
            raise Inconclusive('deploy/public hooks require one argv command: ' + name)
        commands=commands if isinstance(commands,list) and commands and isinstance(commands[0],list) else [commands]
        for command in commands:
            if not isinstance(command,list) or not command or not all(isinstance(a,str) and a for a in command):
                raise Inconclusive('hook must be argv or list of argv: '+name)
    if 'mutation_required' in config and not isinstance(config['mutation_required'], bool):
        raise Inconclusive('mutation_required requires a boolean')
    for field in ('expected_initial', 'expected_before'):
        if field in config and (not isinstance(config[field], dict) or set(config[field]) != set(config['bindings'])):
            raise Inconclusive(field + ' must cover exactly the public bindings')
    if 'mutate' in config['hooks'] and 'expected_initial' not in config:
        raise Inconclusive('mutation hook requires expected_initial')
    if config.get('mutation_required'):
        if not config['hooks'].get('mutate') or 'expected_initial' not in config or 'expected_before' not in config:
            raise Inconclusive('mutable scenario requires mutation hook and X/Y assertions')
        if json.dumps(config['expected_initial'], sort_keys=True) == json.dumps(config['expected_before'], sort_keys=True):
            raise Inconclusive('mutable scenario must declare distinct X and Y')
    timeout = config.get('archive_timeout_seconds', 240)
    if type(timeout) not in (int, float) or not 1 <= timeout <= 3600:
        raise Inconclusive('archive timeout must be bounded to 1..3600 seconds')
    if 'min_ttl_ledgers' in config:
        if type(config['min_ttl_ledgers']) is not int or config['min_ttl_ledgers'] < 1:
            raise Inconclusive('min_ttl_ledgers must be a positive integer')
    if 'max_ttl_ledgers' in config:
        if type(config['max_ttl_ledgers']) is not int or config['max_ttl_ledgers'] < config.get('min_ttl_ledgers', 1):
            raise Inconclusive('max_ttl_ledgers must be >= min_ttl_ledgers')
    if 'archive_witness_metadata' in config:
        validate_archive_witness_metadata(config['archive_witness_metadata'])


def validate_archive_witness_metadata(metadata):
    """Validate optional archive witness metadata structure and fields."""
    if not isinstance(metadata, dict):
        raise Inconclusive('archive witness metadata must be an object')
    if 'ledger_sequence' in metadata:
        seq = metadata['ledger_sequence']
        if not isinstance(seq, int) or seq < 0:
            raise Inconclusive('witness ledger_sequence must be a non-negative integer')
    if 'eviction_keys' in metadata:
        keys = metadata['eviction_keys']
        if not isinstance(keys, list) or not keys:
            raise Inconclusive('witness eviction_keys must be a non-empty list')
        for k in keys:
            if not isinstance(k, str) or not k.strip():
                raise Inconclusive('eviction key must be a non-empty string')
    return True


def validate_executable_hash(executable):
    """Validate contract instance executable format and SHA256 byte hash integrity."""
    if not isinstance(executable, dict) or not executable:
        raise Inconclusive('contract executable requires a nonempty object')
    wasm_key = 'wasm' if 'wasm' in executable else ('wasm_hash' if 'wasm_hash' in executable else None)
    if wasm_key:
        h = executable[wasm_key]
        if not isinstance(h, str) or len(h) != 64 or any(c not in '0123456789abcdefABCDEF' for c in h):
            raise Inconclusive('invalid contract wasm hash: must be 64-character hex string')
        return h.lower()
    if 'token' in executable:
        return 'token'
    raise Inconclusive('unsupported contract executable type')


def fresh(config, salt):
    """Resolve only explicitly declared fresh salt placeholders."""
    if len(salt)!=64 or any(c not in '0123456789abcdef' for c in salt):
        raise Inconclusive('salt requires 32 bytes of lowercase hex')
    def visit(value):
        if isinstance(value,str):return value.replace('{salt}',salt)
        if isinstance(value,list):return [visit(v) for v in value]
        if isinstance(value,dict):return {k:visit(v) for k,v in value.items()}
        return value
    return visit(copy.deepcopy(config))


def validate_ttl_monotonicity(checkpoints):
    """Validate that live_until_ledger is strictly non-decreasing across active ledger checkpoints."""
    prev_live_until = None
    for i, cp in enumerate(checkpoints):
        if not isinstance(cp, dict):
            continue
        live_until = cp.get("live_until_ledger")
        if live_until is not None:
            if prev_live_until is not None and live_until < prev_live_until and not cp.get("archived", False):
                return False, f"Checkpoint {i} live_until_ledger ({live_until}) regressed from previous ({prev_live_until})"
            prev_live_until = live_until
    return True, None

