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
    if not isinstance(config['protocol'],int) or config['protocol']!=28:
        raise Inconclusive('only verified protocol 28 is supported')
    if not isinstance(config['entries'],dict) or not config['entries']:
        raise Inconclusive('nonempty named ledger entries required')
    for name,key in config['entries'].items():
        if (not isinstance(name,str) or not isinstance(key,dict) or
                set(key)!={'contract_data'} or key['contract_data'].get('durability')!='persistent'):
            raise Inconclusive('observation requires explicit persistent contract-data key')
        if set(key['contract_data'])!={'contract','key','durability'}:
            raise Inconclusive('incomplete contract-data key')
    if not isinstance(config['bindings'],dict) or not config['bindings']:
        raise Inconclusive('nonempty raw/public bindings required')
    for name,binding in config['bindings'].items():
        if (not isinstance(binding,dict) or binding.get('entry') not in config['entries']
                or not isinstance(binding.get('path'),list) or not binding['path']):
            raise Inconclusive('raw binding requires declared entry and path')
        if name not in config['hooks']:
            raise Inconclusive('public binding requires command hook')
    for phase in ('deploy','restore'):
        if phase not in config['hooks']:
            raise Inconclusive('missing hook: '+phase)
    for name,commands in config['hooks'].items():
        commands=commands if isinstance(commands,list) and commands and isinstance(commands[0],list) else [commands]
        for command in commands:
            if not isinstance(command,list) or not command or not all(isinstance(a,str) and a for a in command):
                raise Inconclusive('hook must be argv or list of argv: '+name)
    if config.get('mutation_required'):
        if not config['hooks'].get('mutate') or 'expected_initial' not in config or 'expected_before' not in config:
            raise Inconclusive('mutable scenario requires mutation hook and X/Y assertions')
        if config['expected_initial']==config['expected_before']:
            raise Inconclusive('mutable scenario must declare distinct X and Y')
    if not 1<=config.get('archive_timeout_seconds',240)<=3600:
        raise Inconclusive('archive timeout must be bounded to 1..3600 seconds')


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
