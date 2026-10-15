import os,subprocess,json
from capture import ROOT
for scenario,name in [('blend-config-admin','blend-live'),('tansu-project-commit','tansu-live')]:
 argv=['python3','-m','archival_regression','run','scenarios/'+scenario+'.json','--output',str(ROOT/name)]
 result=subprocess.run(argv,env={**os.environ,'PYTHONPATH':'src'},capture_output=True,text=True)
 record={'argv':argv,'exit_code':result.returncode,'stdout':result.stdout,'stderr':result.stderr}
 with (ROOT/(name+'-execution.json')).open('x') as f:json.dump(record,f,indent=2)
 print(name,result.returncode,result.stdout,result.stderr,flush=True)
 if result.returncode!=0:raise RuntimeError('Campaign did not pass; stop and retain diagnostics')
