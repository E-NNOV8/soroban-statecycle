import hashlib
import json
from capture import ROOT, run, rpc
from configure import enc,dec
b=json.loads((ROOT/'baseline.json').read_text())
k=json.loads((ROOT/'instance-key.json').read_text())['stdout'].strip()
code=enc('pool-code-key-v2','LedgerKey',{'contract_code':{'hash':b['pool_hash']}})
ttl=enc('instance-ttl-key-v2','LedgerKey',{'ttl':{'key_hash':hashlib.sha256(__import__('base64').b64decode(k)).hexdigest()}})
state=rpc('baseline-ledger-entries-v2','getLedgerEntries',{'keys':[k,code]})
for i,e in enumerate(state['entries']):
    dec('baseline-entry-'+str(i),'LedgerEntryData',e['xdr'])
for name in ['factory-deploy','pool-deploy','baseline-get_config','baseline-get_admin']:
    candidates=list(ROOT.glob(name+'-tx-*.json'))
    for j,path in enumerate(candidates):
        data=json.loads(json.loads(path.read_text())['stdout'])['result']
        if data.get('status')=='SUCCESS':
            dec(name+'-metadata-'+str(j),'TransactionMeta',data['resultMetaXdr'])
print('Captured exact instance, TTL, code keys and decoded transaction metadata')
