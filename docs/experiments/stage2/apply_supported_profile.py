import base64,json,time
from capture import ROOT,run,rpc
from prepare_host import NET,SRC,encode,decode
old=json.loads(json.loads((ROOT/'config-before-decoded.json').read_text())['stdout'])['config_setting']['state_archival']
old['max_entry_ttl']=4096
xdr=encode('supported-config-upgrade-xdr','ConfigUpgradeSet',{'updated_entry':[{'state_archival':old}]})
uk=json.loads(json.loads((ROOT/'config-upgrade-key-decoded.json').read_text())['stdout'])
content=run('supported-config-upload',['stellar','contract','invoke','--id',uk['contract_id'],*NET,*SRC,'--','write','--xdr_bytes',base64.b64decode(xdr).hex()],timeout=90).strip().strip('"')
key=encode('supported-config-key','ConfigUpgradeSetKey',{'contract_id':uk['contract_id'],'content_hash':content})
for i in range(10):
 o=run('supported-config-activate-'+str(i),['docker','exec','stellar-local','curl','-sSG','http://localhost:11626/upgrades?mode=set&upgradetime=1970-01-01T00:00:00Z','--data-urlencode','configupgradesetkey='+key])
 if 'Error' not in o:break
 time.sleep(1)
else:raise RuntimeError('Core rejected supported profile')
lk=json.loads((ROOT/'config-key.json').read_text())['stdout'].strip()
for i in range(10):
 e=rpc('supported-config-active-'+str(i),'getLedgerEntries',{'keys':[lk]})
 d=decode('supported-config-decoded-'+str(i),'LedgerEntryData',e['entries'][0]['xdr'])
 if d['config_setting']['state_archival']==old:break
 time.sleep(1)
else:raise RuntimeError('supported profile not observed')
info={'forge_revision':'63d5c1ff5002427e57dab46fbc0c3269da823ccc','forge_wasm':'docs/experiments/stage2/artifacts/soroban_forge_marketplace_royalties.wasm','forge_hash':json.loads((ROOT/'forge-upload.json').read_text())['stdout'].strip(),'sac':json.loads((ROOT/'sac-deploy.json').read_text())['stdout'].strip(),'profile':old,'profile_verified_ledger':e['latestLedger']}
(ROOT/'host.json').write_text(json.dumps(info,indent=2)+'\n');print(json.dumps(info,indent=2))
