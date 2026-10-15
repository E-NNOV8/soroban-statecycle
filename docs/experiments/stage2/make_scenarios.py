"""Target-specific declared inputs only; no campaign lifecycle implementation."""
import json
from pathlib import Path
from Crypto.Hash import keccak
from capture import ROOT
from prepare_host import NET,SRC,ADMIN
p=Path('scenarios');p.mkdir(exist_ok=True)
base={'schema':'archival-regression/scenario-v2','rpc_url':'http://localhost:8000/rpc','passphrase':'Standalone Network ; February 2017','protocol':28,'archive_timeout_seconds':240}
old=json.loads(Path('docs/experiments/stage1/host-resume-20261015/run-1-config.json').read_text())
blend={**base,'name':'blend-config-admin','source':{'revision':'ba22b487b2c5057a4ecc28b05b5193c28e4bd117'},'hooks':old['hooks'],'entries':{'instance':{'contract_data':{'contract':'{id}','key':'ledger_key_contract_instance','durability':'persistent'}}},'bindings':{name:{'entry':'instance','path':['contract_data','val'],'value_path':['storage',symbol]} for name,symbol in old['bindings'].items()}}
# A contract-instance SCVal is projected through its supported storage map.
blend['hooks']['deploy'][blend['hooks']['deploy'].index('--salt')+1]='{salt}'
(p/'blend-config-admin.json').write_text(json.dumps(blend,indent=2)+'\n')
hash_record=ROOT/'tansu-upload-final.json'
if not hash_record.exists():
 print('Blend scenario generated; Tansu upload still pending');raise SystemExit(0)
hash_=json.loads(hash_record.read_text())['stdout'].strip()
sac=json.loads((ROOT/'sac-deploy.json').read_text())['stdout'].strip()
x=json.loads((ROOT/'tansu-parent.json').read_text())['stdout'].strip()
y=json.loads((ROOT/'tansu-revision.json').read_text())['stdout'].strip()
project='ArchivalStageTwo'
k=keccak.new(digest_bits=256);k.update(project.encode());key=k.hexdigest()
def invoke(fn,*args):return ['stellar','contract','invoke','--id','{id}',*NET,*SRC,'--send','yes','--',fn,*args]
entries={'instance':{'contract_data':{'contract':'{id}','key':'ledger_key_contract_instance','durability':'persistent'}}}
for name,variant in [('project','Key'),('commit','LastHash')]:
 entries[name]={'contract_data':{'contract':'{id}','key':{'vec':[{'symbol':variant},{'bytes':key}]},'durability':'persistent'}}
# Observe exact project/last-hash values and the instance they depend on. The
# project index/list and collateral balances are setup context, outside invariant.
tansu={**base,'name':'tansu-project-commit','source':{'revision':y,'wasm_hash':hash_},'mutation_required':True,'entries':entries,'bindings':{'commit':{'entry':'commit','path':['contract_data','val']}},'expected_initial':{'commit':x},'expected_before':{'commit':y},'hooks':{
 'deploy':['stellar','contract','deploy','--wasm-hash',hash_,'--salt','{salt}',*NET,*SRC,'--','--admin',ADMIN],
 'setup':[
  invoke('set_collateral_contract','--admin',ADMIN,'--collateral_contract',json.dumps({'address':sac,'wasm_hash':None})),
  invoke('pause','--admin',ADMIN,'--paused','false'),
  invoke('register','--maintainer',ADMIN,'--name',project,'--maintainers',json.dumps([ADMIN]),'--url','https://example.invalid/archival-stage-two.git','--ipfs','stage2-project-metadata','--min_voting_period','null','--execute_delay','null'),
  invoke('commit','--maintainer',ADMIN,'--project_key',key,'--hash',x)],
 'mutate':[invoke('commit','--maintainer',ADMIN,'--project_key',key,'--hash',y)],
 'commit':invoke('get_commit','--project_key',key),
 'restore':[
  ['stellar','contract','restore','--wasm-hash',hash_,*NET,*SRC],
  ['stellar','contract','restore','--id','{id}',*NET,*SRC],
  *[['stellar','contract','restore','--id','{id}','--key-xdr','{'+name+'_scval_xdr}',*NET,*SRC] for name in ['project','commit']]]}}
# Encode public ledger key SCVals once; these are supplied target data, not engine
# knowledge of Tansu's key variants. The engine gets ordinary argv strings.
import subprocess
for name in ['project','commit']:
 encoded=subprocess.run(['stellar','xdr','encode','--type','ScVal'],input=json.dumps(entries[name]['contract_data']['key']),capture_output=True,text=True,check=True).stdout.strip()
 for command in tansu['hooks']['restore']:
  command[:]=[a.replace('{'+name+'_scval_xdr}',encoded) for a in command]
(p/'tansu-project-commit.json').write_text(json.dumps(tansu,indent=2)+'\n')
(ROOT/'tansu-declared-inputs.json').write_text(json.dumps({'project_name':project,'project_key_keccak256':key,'initial_commit':x,'mutated_commit':y,'sac':sac,'wasm_hash':hash_},indent=2)+'\n')
print('Both scenarios generated')
