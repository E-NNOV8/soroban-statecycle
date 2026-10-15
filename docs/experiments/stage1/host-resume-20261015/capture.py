"""Local experiment recorder. No command output implies archival by itself."""
import datetime
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent

def run(name, argv, *, stdin=None, timeout=60, check=True):
    path = ROOT / (name + '.json')
    if path.exists():
        raise FileExistsError(path)
    record = {'argv': argv, 'stdin': stdin, 'started_at': datetime.datetime.now(datetime.timezone.utc).isoformat()}
    try:
        p = subprocess.run(argv, input=stdin, capture_output=True, text=True, timeout=timeout)
        record.update(exit_code=p.returncode, stdout=p.stdout, stderr=p.stderr)
    except (OSError, subprocess.TimeoutExpired) as e:
        record.update(exit_code=None, error=str(e))
    path.write_text(json.dumps(record, indent=2) + '\n')
    if check and record['exit_code'] != 0:
        raise RuntimeError(f'{name}: {record}')
    return record.get('stdout', '')

def rpc(name, method, params=None):
    body = {'jsonrpc':'2.0', 'id':1, 'method':method}
    if params is not None:
        body['params'] = params
    out = run(name, ['curl','--noproxy','*','--max-time','20','-sS','http://localhost:8000/rpc','-H','Content-Type: application/json','-d',json.dumps(body)])
    data = json.loads(out)
    if 'error' in data:
        raise RuntimeError(data)
    return data['result']

if __name__ == '__main__':
    run(sys.argv[1], sys.argv[2:], check=False)
