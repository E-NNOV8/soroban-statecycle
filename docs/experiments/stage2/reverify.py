"""Offline final-source validation; Stage 1 corpus is never written."""
import hashlib,json,os,subprocess,tempfile,shutil
from pathlib import Path
from capture import ROOT
PROJECT=Path.cwd()
results=[]
for name in ['blend-live','tansu-live']:
 argv=['python3','-m','archival_regression.campaign','--config',str(ROOT/(name+'.scenario.json')),'--output',str(ROOT/name),'--phase','verify']
 p=subprocess.run(argv,env={**os.environ,'PYTHONPATH':'src'},capture_output=True,text=True)
 record={'argv':argv,'exit_code':p.returncode,'stdout':p.stdout,'stderr':p.stderr}
 (ROOT/(name+'-final-reverify.json')).write_text(json.dumps(record,indent=2)+'\n')
 assert p.returncode==0,record
 results.append({'name':name,'result':'PASS'})
# Revalidate all five original Stage 1 records in disposable folders. The original
# config, checkpoint, raw evidence and reports remain byte-for-byte untouched.
old=PROJECT/'docs/experiments/stage1/host-resume-20261015'
for i in range(1,6):
 with tempfile.TemporaryDirectory() as folder:
  dest=Path(folder)/'evidence'
  shutil.copytree(old/f'run-{i}',dest,ignore=shutil.ignore_patterns('__pycache__'))
  cfg=old/f'run-{i}-config.json'
  argv=['python3','-m','archival_regression.campaign','--config',str(cfg),'--output',str(dest),'--phase','verify']
  p=subprocess.run(argv,env={**os.environ,'PYTHONPATH':'src'},capture_output=True,text=True)
  assert p.returncode==0,(i,p.stdout,p.stderr)
  results.append({'stage1_run':i,'exit_code':p.returncode,'stdout':p.stdout,'stderr':p.stderr,'note':'disposable copy; original corpus unchanged'})
# An actual process with one missing post-restore raw receipt must classify 2.
with tempfile.TemporaryDirectory() as folder:
 dest=Path(folder)/'missing-evidence';shutil.copytree(ROOT/'tansu-live',dest)
 state=json.loads((dest/'checkpoint.json').read_text())
 (dest/state['raw_after_evidence']).unlink()
 argv=['python3','-m','archival_regression.campaign','--config',str(ROOT/'tansu-live.scenario.json'),'--output',str(dest),'--phase','verify']
 p=subprocess.run(argv,env={**os.environ,'PYTHONPATH':'src'},capture_output=True,text=True)
 assert p.returncode==2,(p.stdout,p.stderr)
 results.append({'case':'missing raw post-restore evidence','exit_code':p.returncode,'stdout':p.stdout,'stderr':p.stderr})
preserved=json.loads((ROOT/'stage1-preservation-before.json').read_text())
changed=[name for name,h in preserved.items() if not (PROJECT/name).exists() or hashlib.sha256((PROJECT/name).read_bytes()).hexdigest()!=h]
new=set(str(f.relative_to(PROJECT)) for f in (PROJECT/'docs/experiments/stage1').rglob('*') if f.is_file())-set(preserved)
assert not changed and not new,(changed,new)
(ROOT/'final-reverification.json').write_text(json.dumps({'result':'PASS','results':results,'stage1_preserved_file_count':len(preserved),'stage1_changed_files':changed,'stage1_added_files':sorted(new)},indent=2)+'\n')
print('Final verifier: both fresh Stage 2 scenarios + 5/5 Stage 1 records PASS; missing evidence exit 2; Stage 1 bytes unchanged')
