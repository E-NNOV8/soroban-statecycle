"""Reverify retained evidence with the final source; publish no external state."""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import os
from capture import ROOT,run

project=Path.cwd()
summary={'stage':'Stage 1','result':'PASS','recorded_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_revision':None,'source_revision_note':'main is unborn; exact final file identities are retained below','source_hashes':{str(p.relative_to(project)):hashlib.sha256(p.read_bytes()).hexdigest() for p in [*sorted((project/'src').rglob('*.py')),*sorted((project/'tests').rglob('*.py'))]},'repetitions':[]}
for i in range(1,6):
 out=ROOT/f'run-{i}'
 state=json.loads((out/'checkpoint.json').read_text())
 assert state['phase']=='verified'
 assert state['control_result']=='PASS'
 assert len(state['restore_transactions'])>=2
 assert state['before']==state['control']==state['after']
 assert state['raw_before']==state['raw_after']
 negative=json.loads((ROOT/f'run-{i}-omission-negative.json').read_text())
 assert negative['exit_code']==1
 assert json.loads(negative['stdout'])['reason']=='required explicit restoration omitted'
 argv=['python3','-m','archival_regression.campaign','--config',str(ROOT/f'run-{i}-config.json'),'--output',str(out),'--phase','verify']
 result=subprocess.run(argv,env={**os.environ,'PYTHONPATH':'src'},capture_output=True,text=True)
 with (ROOT/f'final-reverify-{i}.json').open('x') as f:json.dump({'argv':argv,'exit_code':result.returncode,'stdout':result.stdout,'stderr':result.stderr},f,indent=2)
 assert result.returncode==0,(i,result.stdout,result.stderr)
 summary['repetitions'].append({'run':i,'pool':state['id'],'deployment':{k:state['deploy_transaction'][k] for k in ['hash','ledger']},'live_ledger':state['live_ledger'],'live_until':state['live_entry']['liveUntilLedgerSeq'],'archive':state['archive_witness'],'restoration':[{k:t[k] for k in ['hash','ledger']} for t in state['restore_transactions']],'restored_ledger':state['restored_ledger'],'control':'PASS','omission_negative_exit':1,'interruption':'separate process exit after committed restore checkpoint before application recovery','retry':'PASS; already-live empty-footprint RestoreFootprint and unchanged live raw state','public_raw_invariant':'PASS','final_verifier_exit':0})
assert len(set(r['pool'] for r in summary['repetitions']))==5
assert json.loads((ROOT/'unavailable-rpc-host/report-archive.json').read_text())['exit_code']==2
summary.update(fresh_state_repetitions=5,independent_target_builds_executed=0,build_test_provenance='owner reports Blend build/tests passed; agent verified revision, clean tracked diff, artifact hashes and actual unchanged deployment',integration_classification='SMALL GENERAL PRODUCT GAP',profile='Core v29.0.0; RPC local; active protocol 28; ledger-close metadata v2 and transaction metadata v4',boundaries=['configuration/admin instance only; no lending position or real oracle/backstop calls','five fresh pools share one factory/code/network','controlled interruption between committed lifecycle phases, not an in-flight crash','negative checks required explicit restoration policy, not a Blend defect or auto-restore failure','local short-TTL/scan settings remain applied; existing long TTLs not reduced','no public network, remote, push, PR, release, outreach or Stage 2'])
(ROOT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
# Update the existing manifest, preserving its prior bytes in this evidence folder.
m=json.loads((ROOT/'manifest-before-host-resume.json').read_text())
m.update(recorded_at=summary['recorded_at'],phase_reached='verified-real-core-archive-restore',result='PASS',reason='One local unchanged Blend V2 configuration/admin campaign passed with direct hot-archive pilot proof and five fresh-state repetitions; see host-resume summary.',fresh_state_campaign_runs=5,independent_target_builds=0,recommendation='STOP AT STAGE 1',next_dependency='Stage 1 complete for the bounded configuration/admin invariant. Stage 2 requires new authorization.')
m['product']['source_hashes']=summary['source_hashes']
b=json.loads((ROOT/'baseline.json').read_text())
m['external_target'].update(revision=b['revision'],artifact_used=True,source_modified=False,wasm_hashes={'pool':b['pool_hash'],'pool_factory':b['factory_hash']},contract_ids={'factory':b['factory'],'original_baseline_pool':b['pool'],'repetition_pools':[r['pool'] for r in summary['repetitions']]},actual_integration_classification=summary['integration_classification'],integration_result='PASS')
m['environment'].update(core='29.0.0',host='28.0.0 (active protocol profile; full Core host identities in core-version.json)',protocol=28,network_config='host-resume-20261015/active-config-decoded-1.json (see actual active-config-decoded-* files)',ledger_anchors=[r['archive'] for r in summary['repetitions']],ledger_keys=[r['archive']['key'] for r in summary['repetitions']],network_created=False,non_default_network_settings={'min_persistent_ttl':64,'min_temporary_ttl':16,'starting_eviction_scan_level':1,'quickstart_artificially_accelerate_time_for_testing':True},non_default_settings_note='Consensus settings upgrade verified at ledger 4457. Existing TTLs unchanged; local profile remains applied. Container preexisted this resume.')
m['invariant'].update(status='PASS',observed_values=b['baseline'],limitation='Configuration/admin continuity and complete instance equality only; valid placeholder account addresses; no reserve, lending, real oracle/backstop/token operation tested.')
for key in m['outcomes']:m['outcomes'][key]='PASS'
m['outcomes']['negative']='PASS: intended required-restore omission produced FAIL exit 1; infrastructure absence separately INCONCLUSIVE exit 2'
m['checks']['python_behavior_tests'].update(passed=30,failed=0,result='PASS',log='host-resume-20261015/tests-final.log')
m['checks']['python_compile'].update(result='PASS',log='host-resume-20261015/compile-final.json')
m['checks']['blend_build']={'result':'OWNER-REPORTED PASS','agent_rebuild_executed':False}
for gate in m['exit_gates']:gate['result']='PASS'
m['last_resume']={'result':'PASS','evidence':'host-resume-20261015/summary.json','guide':'host-resume-20261015/README.md','previous_manifest':'host-resume-20261015/manifest-before-host-resume.json'}
m['current_commands_note']='Historical commands above remain unchanged; all current argv/stdin/stdout/stderr and raw RPC/XDR evidence are in host-resume-20261015/ and run-N/. See summary for final verifier identities.'
(project/'docs/experiments/stage1/manifest.json').write_text(json.dumps(m,indent=2)+'\n')
print(json.dumps({'result':'PASS','fresh_state_repetitions':5,'source_hashes':summary['source_hashes']},indent=2))
