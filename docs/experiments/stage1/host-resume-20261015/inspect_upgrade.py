import json
from capture import ROOT,rpc,run
from configure import dec,enc
lines=json.loads((ROOT/'settings-upgrade-transactions.json').read_text())['stdout'].strip().splitlines()
key=dec('upgrade-key-decoded','ConfigUpgradeSetKey',lines[-1])
print(key)
k=enc('upgrade-data-key','LedgerKey',{'contract_data':{'contract':key['contract_id'],'key':{'bytes':key['content_hash']},'durability':'temporary'}})
x=rpc('upgrade-data-read','getLedgerEntries',{'keys':[k]})
print(x)
for e in x['entries']:
 data=dec('upgrade-data-decoded','LedgerEntryData',e['xdr'])
 print(data)
run('logs-inventory',['docker','exec','stellar-local','ls','-lt','/var/log/stellar-core'])
