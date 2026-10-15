"""Local artifact/dependency and consensus profile setup; not archive proof."""
import base64,hashlib,json,time
from pathlib import Path
from capture import ROOT,run,rpc
NET=['--rpc-url','http://localhost:8000/rpc','--network-passphrase','Standalone Network ; February 2017']
SRC=['--source','archival-admin']
ADMIN='GBPQSQQRS34EPU6QF3RLGHIHA7NQHNYQ4UUUSIYR6T4F5FLLIHQZMQEN'

def encode(name,typ,obj):return run(name,['stellar','xdr','encode','--type',typ],stdin=json.dumps(obj)).strip()
def decode(name,typ,xdr):return json.loads(run(name,['stellar','xdr','decode','--type',typ,'--input','single-base64',xdr]))
if __name__=='__main__':
 run('host-container',['docker','ps','--no-trunc'])
 run('core-version',['docker','exec','stellar-local','stellar-core','version'])
 rpc('network','getNetwork');rpc('health','getHealth')
 # Upload code under the existing long maximum TTL; shorten only new application
 # persistent extensions via a real consensus upgrade afterward.
 artifact=Path('.local/stage2/forge-build/wasm32v1-none/release/soroban_forge_marketplace_royalties.wasm')
 dest=ROOT/'artifacts';dest.mkdir(exist_ok=True)
 (dest/artifact.name).write_bytes(artifact.read_bytes())
 h=hashlib.sha256(artifact.read_bytes()).hexdigest()
 out=run('forge-upload',['stellar','contract','upload','--wasm',str(artifact),*NET,*SRC],timeout=90).strip();assert out==h,(out,h)
 # SAC is a genuine protocol token dependency, not a fabricated token contract.
 sac=run('sac-deploy',['stellar','contract','asset','deploy','--asset','ARST2:'+ADMIN,*NET,*SRC],timeout=90).strip()
 key=encode('config-key','LedgerKey',{'config_setting':{'config_setting_id':'state_archival'}})
 orig=rpc('config-before','getLedgerEntries',{'keys':[key]})
 cfg=decode('config-before-decoded','LedgerEntryData',orig['entries'][0]['xdr'])['config_setting']['state_archival']
 cfg['max_entry_ttl']=128
 xdr=encode('config-upgrade-xdr','ConfigUpgradeSet',{'updated_entry':[{'state_archival':cfg}]})
 account=json.loads(run('admin-sequence',['curl','--noproxy','*','-sS','http://localhost:8000/accounts/'+ADMIN]))
 lines=run('config-upgrade-transactions',['docker','exec','stellar-local','stellar-core','get-settings-upgrade-txs',ADMIN,account['sequence'],'Standalone Network ; February 2017','--xdr',xdr]).strip().splitlines()
 for i in range(0,len(lines)-1,2):
  signed=run('config-sign-'+str(i),['stellar','tx','sign',*NET,'--sign-with-key','archival-admin'],stdin=lines[i]).strip()
  run('config-send-'+str(i),['stellar','tx','send',*NET],stdin=signed,timeout=90)
  tx=rpc('config-tx-'+str(i),'getTransaction',{'hash':lines[i+1]});assert tx['status']=='SUCCESS'
 # Detect what the installed helper actually emitted; upload missing payload
 # via its unmodified protocol upgrade contract if necessary.
 upgrade_key=decode('config-upgrade-key-decoded','ConfigUpgradeSetKey',lines[-1])
 data_key=encode('config-payload-key','LedgerKey',{'contract_data':{'contract':upgrade_key['contract_id'],'key':{'bytes':upgrade_key['content_hash']},'durability':'temporary'}})
 payload=rpc('config-payload-read','getLedgerEntries',{'keys':[data_key]})
 if not payload['entries']:
  run('config-upload-payload',['stellar','contract','invoke','--id',upgrade_key['contract_id'],*NET,*SRC,'--','write','--xdr_bytes',base64.b64decode(xdr).hex()],timeout=90)
 for i in range(10):
  result=run('config-activate-'+str(i),['docker','exec','stellar-local','curl','-sSG','http://localhost:11626/upgrades?mode=set&upgradetime=1970-01-01T00:00:00Z','--data-urlencode','configupgradesetkey='+lines[-1]])
  if 'Error' not in result:break
  time.sleep(1)
 else:raise RuntimeError('INCONCLUSIVE: Core config rejected')
 for i in range(15):
  active=rpc('config-active-'+str(i),'getLedgerEntries',{'keys':[key]})
  value=decode('config-active-decoded-'+str(i),'LedgerEntryData',active['entries'][0]['xdr'])['config_setting']['state_archival']
  if value==cfg:break
  time.sleep(1)
 else:raise RuntimeError('INCONCLUSIVE: profile not verified')
 info={'forge_revision':'63d5c1ff5002427e57dab46fbc0c3269da823ccc','forge_wasm':str(dest/artifact.name),'forge_hash':h,'sac':sac,'asset':'ARST2:'+ADMIN,'admin':ADMIN,'network':NET,'profile':cfg,'profile_verified_ledger':active['latestLedger']}
 (ROOT/'host.json').write_text(json.dumps(info,indent=2)+'\n');print(json.dumps(info,indent=2))
