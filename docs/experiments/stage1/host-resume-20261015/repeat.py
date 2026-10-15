"""Five fresh-state executions of the general campaign; no new network/target."""
import json,os,subprocess
from pathlib import Path
from capture import ROOT
from provision import NET,SRC,ADMIN,ORACLE
b=json.loads((ROOT/'baseline.json').read_text())
for i in range(2,6):
 config={'rpc_url':'http://localhost:8000/rpc','passphrase':'Standalone Network ; February 2017','protocol':28,'archive_timeout_seconds':240,'bindings':{'get_config':'Config','get_admin':'Admin'},'hooks':{
  'deploy':['stellar','contract','invoke','--id',b['factory'],*NET,*SRC,'--','deploy','--admin',ADMIN,'--name','Stage 1 repetition '+str(i),'--salt',('%02x'%(32+i))*32,'--oracle',ORACLE,'--backstop_take_rate','1000000','--max_positions','4','--min_collateral','10000000'],
  'get_config':['stellar','contract','invoke','--id','{id}',*NET,*SRC,'--send','yes','--','get_config'],
  'get_admin':['stellar','contract','invoke','--id','{id}',*NET,*SRC,'--send','yes','--','get_admin'],
  'restore':['stellar','contract','restore','--id','{id}',*NET,*SRC]}}
 path=ROOT/f'run-{i}-config.json'
 with path.open('x') as f:json.dump(config,f,indent=2)
 out=ROOT/f'run-{i}'
 # Separate child processes provide actual checkpoint/resume boundaries.
 for label,phase,expected in [('archive','archive',2),('omission-negative','verify',1),('restore-checkpoint','restore',2),('resume-retry','recover',0),('final-verification','verify',0)]:
  argv=['python3','-m','archival_regression.campaign','--config',str(path),'--output',str(out),'--phase',phase]
  p=subprocess.run(argv,env={**os.environ,'PYTHONPATH':'src'},capture_output=True,text=True)
  record={'argv':argv,'exit_code':p.returncode,'stdout':p.stdout,'stderr':p.stderr,'expected_exit':expected}
  (ROOT/f'run-{i}-{label}.json').write_text(json.dumps(record,indent=2)+'\n')
  print(i,label,p.returncode,p.stdout.strip(),p.stderr[-1000:],flush=True)
  if p.returncode!=expected:raise RuntimeError('Unexpected campaign outcome; stop')
