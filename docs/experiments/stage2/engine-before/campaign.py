"""Local Core lifecycle campaign with declarative command hooks and raw XDR proof.

Exit 0: verified recovery; 1: supplied lifecycle/invariant violation;
2: incomplete campaign or missing infrastructure/evidence. No target branches.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import time
from urllib.request import Request, build_opener, ProxyHandler
from urllib.parse import urlsplit
from .preflight import NoRedirect


class Inconclusive(Exception):
    pass


class Violation(Exception):
    pass


def scval(value):
    """Decode supported scalar/map SCVals; reject unsupported/missing variants."""
    if not isinstance(value, dict) or len(value) != 1:
        raise Inconclusive('unsupported SCVal')
    tag, data = next(iter(value.items()))
    if tag in ('address', 'symbol', 'string', 'u32', 'i32', 'bool', 'bytes'):
        return data
    if tag in ('i128', 'u128', 'i64', 'u64'):
        return str(data)
    if tag == 'map' and isinstance(data, list):
        result = {}
        for pair in data:
            key = scval(pair['key'])
            if key in result:
                raise Inconclusive('duplicate map key')
            result[key] = scval(pair['val'])
        return result
    raise Inconclusive('unsupported SCVal tag: '+tag)


def compare(raw, public, bindings):
    try:
        storage = raw['contract_data']['val']['contract_instance']['storage']
        values = {scval(p['key']): scval(p['val']) for p in storage}
        if len(values) != len(storage):
            raise Inconclusive('duplicate instance key')
        for fn, key in bindings.items():
            if values[key] != public[fn]:
                raise Violation('raw/public mismatch: '+fn)
    except (KeyError, TypeError) as e:
        raise Inconclusive('missing raw/public assertion input') from e


def require_recovered(state):
    if not state.get('archive_witness'):
        raise Inconclusive('missing actual Core eviction witness')
    if not state.get('restore_transactions'):
        raise Violation('required explicit restoration omitted')
    if not state.get('after'):
        raise Inconclusive('missing post-restore observations')
    if state['before'] != state['after']:
        raise Violation('public invariant changed')
    if state['raw_before'] != state['raw_after']:
        raise Violation('raw instance changed')


class Campaign:
    def __init__(self, config, output):
        self.config = config
        self.output = output
        parsed = urlsplit(config['rpc_url'])
        if (parsed.scheme != 'http' or parsed.hostname not in
                ('localhost', '127.0.0.1', '::1') or parsed.username or
                parsed.password or parsed.query or parsed.fragment):
            raise Inconclusive('plain loopback RPC required')
        self.state_path = output/'checkpoint.json'
        self.state = json.loads(self.state_path.read_text()) if self.state_path.exists() else {}
        self.index = len(list(output.glob('evidence-*.json')))

    def save(self):
        temp = self.output/'checkpoint.tmp'
        temp.write_text(json.dumps(self.state, indent=2)+'\n')
        temp.replace(self.state_path)

    def record(self, data):
        path = self.output/f'evidence-{self.index:05d}.json'
        self.index += 1
        with path.open('x') as f:
            json.dump(data, f, indent=2)
            f.write('\n')
        return path.name

    def command(self, argv, stdin=None):
        argv = [a.replace('{id}', self.state.get('id', '')) for a in argv]
        record = {'argv': argv, 'stdin': stdin}
        try:
            result = subprocess.run(argv, input=stdin, capture_output=True, text=True, timeout=90)
            record.update(exit_code=result.returncode, stdout=result.stdout, stderr=result.stderr)
        except (OSError, subprocess.TimeoutExpired) as e:
            record.update(exit_code=None, error=str(e))
        filename = self.record(record)
        if record['exit_code'] != 0:
            raise Inconclusive('command failed: '+filename)
        return record['stdout'], record['stderr'], filename

    def rpc(self, method, params=None):
        payload = {'jsonrpc':'2.0', 'id':1, 'method':method}
        if params is not None:
            payload['params'] = params
        record = {'url':self.config['rpc_url'], 'request':payload}
        try:
            req = Request(self.config['rpc_url'],data=json.dumps(payload).encode(),headers={'Content-Type':'application/json'})
            with build_opener(ProxyHandler({}),NoRedirect()).open(req,timeout=20) as response:
                record['body'] = response.read(32*1024*1024+1).decode()
            data = json.loads(record['body'])
        except (OSError, ValueError) as e:
            record['error'] = str(e)
            self.record(record)
            raise Inconclusive('RPC unavailable') from e
        filename = self.record(record)
        if data.get('id') != 1 or data.get('jsonrpc') != '2.0' or 'error' in data or 'result' not in data:
            raise Inconclusive('RPC error: '+filename)
        return data['result'], filename

    def encode(self, typ, value):
        return self.command(['stellar','xdr','encode','--type',typ],json.dumps(value))[0].strip()

    def decode(self, typ, value):
        return json.loads(self.command(['stellar','xdr','decode','--type',typ,'--input','single-base64',value])[0])

    def hook(self, name):
        out, err, filename = self.command(self.config['hooks'][name])
        return out, err, filename

    def transaction(self, err):
        hashes = re.findall(r'(?:Signing transaction: |Transaction hash is )([a-f0-9]{64})',err)
        if len(hashes) != 1:
            raise Inconclusive('missing unique transaction hash')
        data, filename = self.rpc('getTransaction',{'hash':hashes[0]})
        if data.get('status') != 'SUCCESS':
            raise Inconclusive('transaction not successful: '+filename)
        envelope = self.decode('TransactionEnvelope',data['envelopeXdr'])
        meta = self.decode('TransactionMeta',data['resultMetaXdr'])
        return {'hash':hashes[0],'ledger':data['ledger'],'evidence':filename,'envelope':envelope,'metadata':meta}

    def public(self, label):
        values = {}
        for name in self.config['bindings']:
            out, err, _ = self.hook(name)
            values[name] = json.loads(out)
            self.state.setdefault('getter_transactions',[]).append(self.transaction(err))
        self.state[label] = values
        self.save()
        return values

    def raw(self):
        result, filename = self.rpc('getLedgerEntries',{'keys':[self.state['key_xdr']]})
        if len(result.get('entries',[])) != 1:
            raise Inconclusive('missing exact raw instance: '+filename)
        entry = result['entries'][0]
        if entry['key'] != self.state['key_xdr'] or entry['liveUntilLedgerSeq'] < result['latestLedger']:
            raise Inconclusive('raw instance not live or wrong key')
        return self.decode('LedgerEntryData',entry['xdr']), entry, result['latestLedger']

    def archive(self):
        if self.state:
            raise Inconclusive('refusing to deploy over existing campaign')
        network, _ = self.rpc('getNetwork')
        if network['passphrase'] != self.config['passphrase'] or network['protocolVersion'] != self.config['protocol']:
            raise Inconclusive('network/profile mismatch')
        out, err, _ = self.hook('deploy')
        self.state['id'] = json.loads(out)
        if not re.fullmatch('C[A-Z2-7]{55}',self.state['id']):
            raise Inconclusive('invalid deployed contract id')
        self.state['deploy_transaction'] = self.transaction(err)
        self.state['key'] = {'contract_data':{'contract':self.state['id'],'key':'ledger_key_contract_instance','durability':'persistent'}}
        self.state['key_xdr'] = self.encode('LedgerKey',self.state['key'])
        self.public('before')
        raw, entry, ledger = self.raw()
        compare(raw,self.state['before'],self.config['bindings'])
        self.state.update(raw_before=raw, live_entry=entry,live_ledger=ledger)
        self.public('control')
        control_raw, _, _ = self.raw()
        compare(control_raw,self.state['control'],self.config['bindings'])
        if self.state['control'] != self.state['before'] or control_raw != raw:
            raise Violation('unchanged live control changed')
        self.state['control_result'] = 'PASS'
        self.save()
        cursor=ledger
        deadline=time.monotonic()+self.config.get('archive_timeout_seconds',240)
        while time.monotonic()<deadline:
            result, filename = self.rpc('getLedgers',{'startLedger':cursor,'pagination':{'limit':200}})
            for item in result['ledgers']:
                seq=item['sequence']
                if seq != cursor:
                    raise Inconclusive('ledger metadata coverage gap')
                decoded=self.decode('LedgerCloseMeta',item['metadataXdr'])
                body=decoded.get('v2')
                if body is None or body['ledger_header']['header']['ledger_seq'] != seq or body['ledger_header']['hash'] != item['hash']:
                    raise Inconclusive('unsupported or inconsistent ledger metadata')
                if self.state['key'] in body['evicted_keys']:
                    if seq <= entry['liveUntilLedgerSeq']:
                        raise Inconclusive('eviction precedes recorded TTL')
                    self.state['archive_witness']={'ledger':seq,'hash':item['hash'],'rpc_evidence':filename,'key':self.state['key']}
                    self.state['phase']='archived'
                    self.save()
                    return
                cursor=seq+1
            time.sleep(3)
        raise Inconclusive('no actual Core eviction witness within bound')

    def restore(self):
        self.validate_archive()
        out,err,_ = self.hook('restore')
        tx=self.transaction(err)
        body=tx['envelope']['tx']['tx']['operations'][0]['body']
        if 'restore_footprint' not in body:
            raise Inconclusive('expected real RestoreFootprint transaction')
        footprint=tx['envelope']['tx']['tx']['ext']['v1']['resources']['footprint']
        first_restore=not self.state.get('restore_transactions')
        if first_restore and self.state['key'] not in footprint['read_write']:
            raise Inconclusive('restore did not target observed key')
        if first_restore:
            operations=tx['metadata']['v4']['operations']
            restored=[change['restored']['data'] for op in operations
                      for change in op['changes'] if 'restored' in change]
            if self.state['raw_before'] not in restored:
                raise Inconclusive('metadata lacks exact restored instance contents')
        else:
            # The CLI drops already-live keys from a retry's footprint. An empty
            # successful RestoreFootprint is valid only with an observed live,
            # unchanged instance and a previously proven restoration.
            if self.state['key'] not in footprint['read_write']:
                if footprint['read_write'] or footprint['read_only']:
                    raise Inconclusive('retry targets unexpected keys')
                current, _, _=self.raw()
                if current!=self.state['raw_before']:
                    raise Violation('restore retry changed raw instance')
                tx['retry_semantics']='already-live empty-footprint no-op; raw state verified'
        self.state.setdefault('restore_transactions',[]).append(tx)
        self.state['phase']='restored-checkpoint'
        self.save()

    def validate_archive(self):
        witness=self.state.get('archive_witness')
        if not witness:
            raise Inconclusive('missing actual Core eviction witness')
        filename=witness['rpc_evidence']
        if Path(filename).name != filename:
            raise Inconclusive('invalid evidence path')
        record=json.loads((self.output/filename).read_text())
        result=json.loads(record['body'])['result']
        matches=[item for item in result['ledgers'] if item['sequence']==witness['ledger']]
        if len(matches)!=1:
            raise Inconclusive('missing raw eviction metadata')
        item=matches[0]
        meta=self.decode('LedgerCloseMeta',item['metadataXdr'])['v2']
        header=meta['ledger_header']
        if (header['hash']!=witness['hash'] or header['header']['ledger_seq']!=witness['ledger']
                or self.state['key'] not in meta['evicted_keys']
                or witness['ledger']<=self.state['live_entry']['liveUntilLedgerSeq']):
            raise Inconclusive('raw Core eviction witness does not match checkpoint')

    def verify_recovery(self):
        self.validate_archive()
        require_recovered(self.state)
        compare(self.state['raw_after'],self.state['after'],self.config['bindings'])
        # Re-read the retained RPC receipt: checkpoint flags/inline metadata do
        # not establish a successful restoration.
        first=self.state['restore_transactions'][0]
        filename=first['evidence']
        if Path(filename).name!=filename:
            raise Inconclusive('invalid restore evidence path')
        receipt=json.loads(json.loads((self.output/filename).read_text())['body'])['result']
        if receipt['status']!='SUCCESS' or receipt['txHash']!=first['hash']:
            raise Inconclusive('missing successful raw restoration receipt')
        if receipt['ledger']<=self.state['archive_witness']['ledger']:
            raise Inconclusive('restoration does not follow actual archival')
        meta=self.decode('TransactionMeta',receipt['resultMetaXdr'])['v4']
        restored=[change['restored']['data'] for op in meta['operations']
                  for change in op['changes'] if 'restored' in change]
        if self.state['raw_before'] not in restored:
            raise Inconclusive('raw receipt lacks restored instance contents')

    def recover(self):
        self.restore()
        self.public('after')
        raw, entry, ledger = self.raw()
        compare(raw,self.state['after'],self.config['bindings'])
        self.state.update(raw_after=raw,restored_entry=entry,restored_ledger=ledger)
        self.verify_recovery()
        self.state['phase']='verified'
        self.save()


def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--phase',choices=['archive','restore','recover','verify'],required=True)
    args=parser.parse_args(argv)
    result='INCONCLUSIVE'; reason=''; code=2
    try:
        config_bytes=args.config.read_bytes()
        config=json.loads(config_bytes)
        if args.phase=='archive':
            args.output.mkdir(parents=True,exist_ok=False)
            (args.output/'config.json').write_bytes(config_bytes)
        else:
            if (args.output/'config.json').read_bytes()!=config_bytes:
                raise Inconclusive('resume configuration changed')
        campaign=Campaign(config,args.output)
        if args.phase=='verify':
            campaign.verify_recovery()
        else:
            getattr(campaign,args.phase)()
        if args.phase in ('recover','verify'):
            result='PASS'; code=0
        else:
            reason='checkpoint preserved; recovery is not complete'
    except Violation as e:
        result='FAIL'; reason=str(e); code=1
    except (Inconclusive,OSError,ValueError,KeyError,TypeError) as e:
        reason=str(e)
    report={'result':result,'reason':reason,'phase':args.phase,'exit_code':code}
    if args.output.is_dir():
        report_path=args.output/('report-'+args.phase+'.json')
        if report_path.exists():
            report_path=args.output/('report-'+args.phase+'-'+str(time.time_ns())+'.json')
        report_path.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report))
    return code

if __name__=='__main__':
    sys.exit(main())
