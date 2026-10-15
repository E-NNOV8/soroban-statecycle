"""Observe real Core eviction; absence and TTL arithmetic alone cannot pass."""
import json,time
from capture import ROOT,run,rpc
from configure import enc,dec
from provision import NET,SRC,ADMIN,ORACLE
b=json.loads((ROOT/'baseline.json').read_text())
pool=run('pilot-deploy',['stellar','contract','invoke','--id',b['factory'],*NET,*SRC,'--','deploy','--admin',ADMIN,'--name','Stage 1 archival pilot','--salt','16'*32,'--oracle',ORACLE,'--backstop_take_rate','1000000','--max_positions','4','--min_collateral','10000000'],timeout=90).strip().strip('"')
kobj={'contract_data':{'contract':pool,'key':'ledger_key_contract_instance','durability':'persistent'}}
k=enc('pilot-instance-key','LedgerKey',kobj)
baseline={}
for fn in ['get_config','get_admin']:
 baseline[fn]=json.loads(run('pilot-baseline-'+fn,['stellar','contract','invoke','--id',pool,*NET,*SRC,'--send','yes','--',fn],timeout=90))
state=rpc('pilot-live','getLedgerEntries',{'keys':[k]})
entry=state['entries'][0]
raw=dec('pilot-live-decoded','LedgerEntryData',entry['xdr'])
start=state['latestLedger']
checkpoint={'pool':pool,'key':kobj,'key_xdr':k,'baseline':baseline,'live_entry':entry,'live_ledger':start,'live_until':entry['liveUntilLedgerSeq']}
(ROOT/'pilot-checkpoint.json').write_text(json.dumps(checkpoint,indent=2)+'\n')
print('Live pool',pool,'ledger',start,'TTL',entry['liveUntilLedgerSeq'],flush=True)
next_ledger=start
witness=None
for i in range(80):
 time.sleep(3)
 latest=rpc('pilot-poll-'+str(i),'getLedgerEntries',{'keys':[k]})
 result=rpc('pilot-ledgers-'+str(i),'getLedgers',{'startLedger':next_ledger,'pagination':{'limit':200}})
 for ledger in result['ledgers']:
  seq=ledger['sequence']
  meta=dec('pilot-ledger-'+str(seq),'LedgerCloseMeta',ledger['metadataXdr'])
  body=next(iter(meta.values()))
  if kobj in body.get('evicted_keys',[]):
   witness={'ledger':seq,'ledger_hash':ledger['hash'],'key':kobj,'metadata_file':'pilot-ledger-'+str(seq)+'.json'}
  next_ledger=seq+1
 if witness:
  checkpoint['archive_witness']=witness
  checkpoint['archive_rpc_state']=latest
  (ROOT/'pilot-archived.json').write_text(json.dumps(checkpoint,indent=2)+'\n')
  print('ACTUAL CORE EVICTION',json.dumps(witness),flush=True)
  break
else:
 raise RuntimeError('INCONCLUSIVE: no actual Core eviction witness within bound')
