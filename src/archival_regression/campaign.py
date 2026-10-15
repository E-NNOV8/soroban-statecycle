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

from .errors import Inconclusive, Violation

def same(left, right):
    """JSON invariant equality preserves types (not Python's True == 1)."""
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(same(left[k], right[k]) for k in left)
    if isinstance(left, list):
        return len(left) == len(right) and all(same(a, b) for a, b in zip(left, right))
    return left == right


def scval(value):
    """Decode supported scalar/map SCVals; reject unsupported/missing variants."""
    if not isinstance(value, dict) or len(value) != 1:
        raise Inconclusive('unsupported SCVal')
    tag, data = next(iter(value.items()))
    if tag in ('address', 'symbol', 'string', 'u32', 'i32', 'bool', 'bytes'):
        return data
    if tag in ('i128', 'u128', 'i64', 'u64'):
        return str(data)
    if tag == 'vec' and isinstance(data, list):
        return [scval(v) for v in data]
    if tag == 'void':
        return None
    if tag == 'contract_instance':
        storage={}
        for pair in data.get('storage') or []:
            key=scval(pair['key'])
            if not isinstance(key,str) or key in storage:
                raise Inconclusive('instance projection requires unique string keys')
            storage[key]=scval(pair['val'])
        return {'executable':data['executable'],'storage':storage}
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
        if bindings and all(isinstance(binding,dict) for binding in bindings.values()):
            for fn,binding in bindings.items():
                value=raw[binding['entry']]
                for part in binding['path']:
                    value=value[part]
                value=scval(value)
                for part in binding.get('value_path',[]):
                    value=value[part]
                if not same(value, public[fn]):
                    raise Violation('raw/public mismatch: '+fn)
            return
        storage = raw['contract_data']['val']['contract_instance']['storage']
        values = {scval(p['key']): scval(p['val']) for p in storage}
        if len(values) != len(storage):
            raise Inconclusive('duplicate instance key')
        for fn, key in bindings.items():
            if not same(values[key], public[fn]):
                raise Violation('raw/public mismatch: '+fn)
    except (KeyError, TypeError, IndexError) as e:
        raise Inconclusive('missing raw/public assertion input') from e


def require_recovered(state):
    if not state.get('archive_witness'):
        raise Inconclusive('missing actual Core eviction witness')
    if not state.get('restore_transactions'):
        raise Violation('required explicit restoration omitted')
    if not state.get('after'):
        raise Inconclusive('missing post-restore observations')
    if not same(state['before'], state['after']):
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
        self.index = 1+max((int(p.stem.split('-')[1]) for p in output.glob('evidence-*.json')),default=-1)

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

    def commands(self,name):
        commands=self.config['hooks'][name]
        if commands and isinstance(commands[0],list):
            return commands
        return [commands]

    def actions(self,name):
        records=[]
        for argv in self.commands(name):
            _,err,_=self.command(argv)
            records.append(self.transaction(err))
        return records

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

    def keyset(self):
        return self.state.get('keys',{'instance':self.state['key']})

    def rawset(self,raw):
        return raw if 'entries' in self.config else {'instance':raw}

    def readset(self):
        return self.state.get('live_entries',{'instance':self.state.get('live_entry')})

    def raw(self):
        keys=self.keyset()
        encoded={name:self.encode('LedgerKey',key) for name,key in keys.items()}
        result,filename=self.rpc('getLedgerEntries',{'keys':list(encoded.values())})
        entries={e['key']:e for e in result.get('entries',[])}
        if set(entries)!=set(encoded.values()) or len(entries)!=len(result.get('entries',[])):
            raise Inconclusive('missing exact raw entries: '+filename)
        raw={}; found={}
        for name,key in encoded.items():
            entry=entries[key]
            if entry['liveUntilLedgerSeq']<result['latestLedger']:
                raise Inconclusive('raw entry is not live: '+name)
            raw[name]=self.decode('LedgerEntryData',entry['xdr'])
            data=raw[name]['contract_data']
            if {k:data[k] for k in ('contract','key','durability')}!=keys[name]['contract_data']:
                raise Inconclusive('decoded raw entry does not match ledger key')
            found[name]=entry
        self.state.setdefault('raw_observations',[]).append({'rpc_evidence':filename,'ledger':result['latestLedger'],'raw':raw})
        if 'entries' in self.config:
            return raw,found,result['latestLedger']
        return raw['instance'],found['instance'],result['latestLedger']

    def archive(self):
        if self.state:
            raise Inconclusive('refusing to deploy over existing campaign')
        network, _ = self.rpc('getNetwork')
        if network['passphrase'] != self.config['passphrase'] or network['protocolVersion'] != self.config['protocol']:
            raise Inconclusive('network/profile mismatch')
        out, err, _ = self.hook('deploy')
        try:
            self.state['id'] = json.loads(out)
        except ValueError:
            self.state['id'] = out.strip()
        if not re.fullmatch('C[A-Z2-7]{55}',self.state['id']):
            raise Inconclusive('invalid deployed contract id')
        self.state['deploy_transaction'] = self.transaction(err)
        if 'entries' in self.config:
            def expand(value):
                if isinstance(value,str):return value.replace('{id}',self.state['id'])
                if isinstance(value,dict):return {k:expand(v) for k,v in value.items()}
                if isinstance(value,list):return [expand(v) for v in value]
                return value
            self.state['keys']=expand(self.config['entries'])
            self.state['key']=next(iter(self.state['keys'].values()))
        else:
            self.state['key'] = {'contract_data':{'contract':self.state['id'],'key':'ledger_key_contract_instance','durability':'persistent'}}
        self.state['key_xdr'] = self.encode('LedgerKey',self.state['key'])
        for phase in ('setup',):
            if phase in self.config['hooks']:
                self.state[phase+'_transactions']=self.actions(phase)
        if 'mutate' in self.config['hooks']:
            self.public('initial')
            initial_raw,_,_=self.raw()
            compare(initial_raw,self.state['initial'],self.config['bindings'])
            self.state['raw_initial']=initial_raw
            if not same(self.state['initial'], self.config['expected_initial']):
                raise Violation('initial state does not equal declared X')
            self.state['mutation_transactions']=self.actions('mutate')
        self.public('before')
        if 'expected_before' in self.config and not same(self.state['before'], self.config['expected_before']):
            raise Violation('pre-archive state does not equal declared Y')
        raw, entry, ledger = self.raw()
        compare(raw,self.state['before'],self.config['bindings'])
        self.state.update(raw_before=raw, live_entry=entry,live_ledger=ledger)
        self.state['raw_before_evidence']=self.state['raw_observations'][-1]['rpc_evidence']
        if 'entries' in self.config:
            self.state['live_entries']=entry
        if self.config.get('mutation_required'):
            if same(self.state['initial'], self.state['before']) or self.state['raw_initial']==raw:
                raise Violation('mutable state did not change from X to Y')
        self.public('control')
        control_raw, _, _ = self.raw()
        compare(control_raw,self.state['control'],self.config['bindings'])
        if not same(self.state['control'], self.state['before']) or control_raw != raw:
            raise Violation('unchanged live control changed')
        self.state['control_result'] = 'PASS'
        self.save()
        cursor=ledger
        deadline=time.monotonic()+self.config.get('archive_timeout_seconds',240)
        while time.monotonic()<deadline:
            if 'advance' in self.config['hooks']:
                for argv in self.commands('advance'):
                    self.command(argv)
            result, filename = self.rpc('getLedgers',{'startLedger':cursor,'pagination':{'limit':200}})
            for item in result['ledgers']:
                seq=item['sequence']
                if seq != cursor:
                    raise Inconclusive('ledger metadata coverage gap')
                decoded=self.decode('LedgerCloseMeta',item['metadataXdr'])
                body=decoded.get('v2')
                if body is None or body['ledger_header']['header']['ledger_seq'] != seq or body['ledger_header']['hash'] != item['hash']:
                    raise Inconclusive('unsupported or inconsistent ledger metadata')
                for name,key in self.keyset().items():
                    if key in body['evicted_keys']:
                        if seq<=self.readset()[name]['liveUntilLedgerSeq']:
                            raise Inconclusive('eviction precedes recorded TTL')
                        self.state.setdefault('archive_witnesses',{})[name]={'ledger':seq,'hash':item['hash'],'rpc_evidence':filename,'key':key}
                if set(self.state.get('archive_witnesses',{}))==set(self.keyset()):
                    self.state['archive_witness']=next(iter(self.state['archive_witnesses'].values()))
                    self.state['phase']='archived'
                    self.save()
                    return
                cursor=seq+1
            time.sleep(3)
        raise Inconclusive('no actual Core eviction witness within bound')

    def receipt(self,tx):
        filename=tx['evidence']
        if Path(filename).name!=filename:
            raise Inconclusive('invalid receipt path')
        receipt=json.loads(json.loads((self.output/filename).read_text())['body'])['result']
        if receipt.get('status')!='SUCCESS' or receipt.get('txHash')!=tx['hash']:
            raise Inconclusive('missing successful raw restoration receipt')
        return receipt

    def restored_data(self,tx):
        receipt=self.receipt(tx)
        envelope=self.decode('TransactionEnvelope',receipt['envelopeXdr'])['tx']['tx']
        if len(envelope['operations'])!=1 or 'restore_footprint' not in envelope['operations'][0]['body']:
            raise Inconclusive('expected real RestoreFootprint transaction')
        meta=self.decode('TransactionMeta',receipt['resultMetaXdr'])['v4']
        restored=[change['restored']['data'] for op in meta['operations']
                  for change in op['changes'] if 'restored' in change]
        return restored,envelope['ext']['v1']['resources']['footprint'],receipt['ledger']

    def restore(self):
        self.validate_archive()
        transactions=self.actions('restore')
        first=not self.state.get('restore_transactions')
        restored=[]; targeted=[]
        for tx in transactions:
            data,footprint,ledger=self.restored_data(tx)
            restored.extend(data);targeted.extend(footprint['read_write'])
            if ledger<=max(w['ledger'] for w in self.witnesses().values()):
                raise Inconclusive('restore does not follow actual archival')
        if first:
            for name,key in self.keyset().items():
                if key not in targeted or self.rawset(self.state['raw_before'])[name] not in restored:
                    raise Inconclusive('metadata lacks exact restored contents: '+name)
        else:
            current,_,_=self.raw()
            if current!=self.state['raw_before']:
                raise Violation('restore retry changed raw state')
        self.state.setdefault('restore_transactions',[]).extend(transactions)
        self.state.setdefault('restore_batches',[]).append(transactions)
        self.state['phase']='restored-checkpoint'
        self.save()

    def witnesses(self):
        return self.state.get('archive_witnesses',{'instance':self.state.get('archive_witness')})

    def validate_archive(self):
        if not self.state.get('archive_witness'):
            raise Inconclusive('missing actual Core eviction witness')
        if set(self.witnesses())!=set(self.keyset()):
            raise Inconclusive('incomplete archive evidence')
        for name,witness in self.witnesses().items():
            filename=witness['rpc_evidence']
            if Path(filename).name!=filename:
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
                    or self.keyset()[name] not in meta['evicted_keys']
                    or witness['ledger']<=self.readset()[name]['liveUntilLedgerSeq']):
                raise Inconclusive('raw Core eviction witness does not match checkpoint')

    def verify_recovery(self):
        self.validate_archive()
        if 'entries' in self.config and not self.state.get('restore_transactions'):
            encoded=[self.encode('LedgerKey',key) for key in self.keyset().values()]
            result,filename=self.rpc('getLedgerEntries',{'keys':encoded})
            if result['latestLedger']<max(w['ledger'] for w in self.witnesses().values()):
                raise Inconclusive('RPC behind proven archive ledger')
            returned=result['entries']
            if any(e['key'] not in encoded or e['liveUntilLedgerSeq']>=result['latestLedger'] for e in returned):
                raise Inconclusive('negative state already restored or unexpected')
            self.state['negative_observation']={'rpc_evidence':filename,'ledger':result['latestLedger'],'required_keys_unavailable_live':encoded}
            self.save()
        require_recovered(self.state)
        compare(self.state['raw_after'],self.state['after'],self.config['bindings'])
        batch=self.state.get('restore_batches',[self.state['restore_transactions'][:1]])[0]
        restored=[]
        for tx in batch:
            data,_,ledger=self.restored_data(tx)
            if ledger<=max(w['ledger'] for w in self.witnesses().values()):
                raise Inconclusive('restoration does not follow actual archival')
            restored.extend(data)
        for name,value in self.rawset(self.state['raw_before']).items():
            if value not in restored:
                raise Inconclusive('raw receipt lacks restored contents: '+name)
        # Both named observation receipts are required; a retained pre-archive
        # observation cannot stand in for missing post-restore raw evidence.
        if 'entries' in self.config:
            for label in ('raw_before','raw_after'):
                filename=self.state[label+'_evidence']
                if Path(filename).name!=filename:
                    raise Inconclusive('invalid raw evidence path')
                record=json.loads((self.output/filename).read_text())
                result=json.loads(record['body'])['result']
                if label=='raw_after' and result['latestLedger']<max(tx['ledger'] for tx in batch):
                    raise Inconclusive('raw after evidence precedes restoration')
                actual={}
                for name,key in self.keyset().items():
                    encoded=self.encode('LedgerKey',key)
                    matches=[e for e in result['entries'] if e['key']==encoded]
                    if len(matches)!=1 or matches[0]['liveUntilLedgerSeq']<result['latestLedger']:
                        raise Inconclusive('required live raw observation missing: '+name)
                    actual[name]=self.decode('LedgerEntryData',matches[0]['xdr'])
                if actual!=self.rawset(self.state[label]):
                    raise Inconclusive('raw observation disagrees with checkpoint')

    def recover(self):
        self.restore()
        self.public('after')
        raw, entry, ledger = self.raw()
        compare(raw,self.state['after'],self.config['bindings'])
        self.state.update(raw_after=raw,restored_entry=entry,restored_ledger=ledger)
        self.state['raw_after_evidence']=self.state['raw_observations'][-1]['rpc_evidence']
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
        if 'schema' in config:
            from .scenario import validate
            validate(config)
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
    except (Inconclusive,OSError,ValueError,KeyError,TypeError,IndexError,AttributeError) as e:
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
