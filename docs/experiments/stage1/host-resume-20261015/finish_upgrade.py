import base64,json,time
from capture import ROOT,run,rpc
from configure import dec
from provision import NET,SRC
key=json.loads(json.loads((ROOT/'upgrade-key-decoded.json').read_text())['stdout'])
xdr=json.loads((ROOT/'short-ttl-upgrade-xdr.json').read_text())['stdout'].strip()
run('upload-settings-payload',['stellar','contract','invoke','--id',key['contract_id'],*NET,*SRC,'--','write','--xdr_bytes',base64.b64decode(xdr).hex()],timeout=90)
lines=json.loads((ROOT/'settings-upgrade-transactions.json').read_text())['stdout'].strip().splitlines()
for i in range(10):
 output=run('activate-settings-'+str(i),['docker','exec','stellar-local','curl','-sSG','http://localhost:11626/upgrades?mode=set&upgradetime=1970-01-01T00:00:00Z','--data-urlencode','configupgradesetkey='+lines[-1]])
 if 'Error' not in output:break
 time.sleep(1)
else:raise RuntimeError('Core rejected uploaded upgrade')
k=json.loads((ROOT/'archival-config-key-v2.json').read_text())['stdout'].strip()
for i in range(20):
 e=rpc('active-config-'+str(i),'getLedgerEntries',{'keys':[k]})
 d=dec('active-config-decoded-'+str(i),'LedgerEntryData',e['entries'][0]['xdr'])
 if d['config_setting']['state_archival']['min_persistent_ttl']==64:
  print('Actual on-ledger short TTL settings verified at ledger',e['latestLedger']); break
 time.sleep(1)
else:raise RuntimeError('upgrade not observed')
