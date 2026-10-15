import json,re
from capture import ROOT,run,rpc
from configure import dec
from provision import NET,SRC
p=json.loads((ROOT/'pilot-archived.json').read_text())
run('pilot-before-restore',['stellar','contract','read','--id',p['pool'],*NET,'--output','xdr'],check=False)
run('pilot-restore',['stellar','contract','restore','--id',p['pool'],*NET,*SRC],timeout=90)
r=json.loads((ROOT/'pilot-restore.json').read_text())
hashes=re.findall(r'(?:Signing transaction: |Transaction hash is )([a-f0-9]{64})',r['stderr'])
print(r['stdout'],r['stderr'],hashes)
for i,h in enumerate(hashes):
 t=rpc('pilot-restore-tx-'+str(i),'getTransaction',{'hash':h})
 assert t['status']=='SUCCESS'
 dec('pilot-restore-meta-'+str(i),'TransactionMeta',t['resultMetaXdr'])
 dec('pilot-restore-envelope-'+str(i),'TransactionEnvelope',t['envelopeXdr'])
values={}
for fn in ['get_config','get_admin']:
 values[fn]=json.loads(run('pilot-after-'+fn,['stellar','contract','invoke','--id',p['pool'],*NET,*SRC,'--send','yes','--',fn],timeout=90))
assert values==p['baseline'], (values,p['baseline'])
s=rpc('pilot-restored-state','getLedgerEntries',{'keys':[p['key_xdr']]})
raw=dec('pilot-restored-decoded','LedgerEntryData',s['entries'][0]['xdr'])
before=json.loads(json.loads((ROOT/'pilot-live-decoded.json').read_text())['stdout'])
assert raw==before
(ROOT/'pilot-recovery-result.json').write_text(json.dumps({'result':'PASS','scope':'single pilot archive/restore public and raw equality only; control/negative/retry/repetitions still pending','after':values,'raw_equal':True,'restored_ledger':s['latestLedger']},indent=2)+'\n')
print('Pilot public invariant and complete raw instance match')
