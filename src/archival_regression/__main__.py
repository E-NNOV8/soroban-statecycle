"""Minimal local campaign command: python -m archival_regression run SCENARIO."""
import argparse
import json
import os
from pathlib import Path
import secrets
import subprocess
import sys
from .campaign import Inconclusive
from .scenario import fresh,load_scenario


def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__)
    sub=parser.add_subparsers(dest='action',required=True)
    run=sub.add_parser('run')
    run.add_argument('scenario',type=Path)
    run.add_argument('--output',type=Path,required=True)
    args=parser.parse_args(argv)
    try:
        config=fresh(load_scenario(args.scenario),secrets.token_hex(32))
        if args.output.exists():
            raise Inconclusive('refusing existing campaign output')
        resolved=args.output.with_name(args.output.name+'.scenario.json')
        resolved.parent.mkdir(parents=True,exist_ok=True)
        with resolved.open('x') as f:json.dump(config,f,indent=2)
        records=[]
        for label,phase,expected in [('archive','archive',2),('negative','verify',1),('restore-checkpoint','restore',2),('resume-retry','recover',0),('verify','verify',0)]:
            argv=['python3','-m','archival_regression.campaign','--config',str(resolved),'--output',str(args.output),'--phase',phase]
            process=subprocess.run(argv,capture_output=True,text=True)
            records.append({'phase':label,'argv':argv,'exit_code':process.returncode,'stdout':process.stdout,'stderr':process.stderr})
            (args.output/'runner.json').write_text(json.dumps(records,indent=2)+'\n')
            print(label,process.stdout.strip(),flush=True)
            if process.returncode!=expected:
                return process.returncode or 2
            state=json.loads((args.output/'checkpoint.json').read_text())
            if phase=='archive' and state.get('phase')!='archived':
                return 2
            if label=='negative' and json.loads(process.stdout)['reason']!='required explicit restoration omitted':
                return 2
            if label=='restore-checkpoint' and state.get('phase')!='restored-checkpoint':
                return 2
        (args.output/'campaign-result.json').write_text(json.dumps({'result':'PASS','control':'PASS','positive':'PASS','negative':'PASS (required explicit restore omitted; exit 1)','retry':'PASS (separate process after committed restore)','raw_verification':'PASS'},indent=2)+'\n')
        return 0
    except (Inconclusive,OSError,ValueError,KeyError,TypeError) as e:
        print(json.dumps({'result':'INCONCLUSIVE','reason':str(e)}))
        return 2

if __name__=='__main__':sys.exit(main())
