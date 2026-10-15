"""Stage 1 local provisioning experiment, unchanged Blend binaries."""
import hashlib
import json
import re
from pathlib import Path
from capture import ROOT, run, rpc

BLEND = Path.cwd().parent / 'blend-contracts-v2'
ART = BLEND / 'target/wasm32-unknown-unknown/optimized'
POOL_HASH = '274d85836c4e9adf8995f9240c6452cb0ce2e4751e1b10baffffdbfb90319ba1'
FACTORY_HASH = 'e304170e67184748839dda5fd3b6dd5d8bf1e54055a07b536cede24da072c7a9'
ADMIN = 'GBPQSQQRS34EPU6QF3RLGHIHA7NQHNYQ4UUUSIYR6T4F5FLLIHQZMQEN'
ORACLE = 'GDIGJ6OENHTHPP6TZA5C5NV2D5JD5TEHIZKWT6OYTUP7YSB4ZHVKU7HJ'
BACKSTOP = 'GDTR7BTCCRJOKFNVR4UARUUITZQM7MPVFQTOXO233ZD2TZPZOYUVXBKL'
NET = ['--rpc-url','http://localhost:8000/rpc','--network-passphrase','Standalone Network ; February 2017']
SRC = ['--source','archival-admin']

if __name__ == '__main__':
    revision = run('blend-revision',['git','-C',str(BLEND),'rev-parse','HEAD']).strip()
    assert revision == 'ba22b487b2c5057a4ecc28b05b5193c28e4bd117'
    run('blend-diff',['git','-C',str(BLEND),'diff','HEAD','--'])
    for name, expected in [('pool',POOL_HASH),('pool_factory',FACTORY_HASH)]:
        assert hashlib.sha256((ART / (name+'.wasm')).read_bytes()).hexdigest() == expected
    run('artifact-hashes',['sha256sum',str(ART/'pool.wasm'),str(ART/'pool_factory.wasm')])
    run('cli-version',['stellar','--version'])
    run('core-version',['docker','exec','stellar-local','stellar-core','version'])
    run('image',['docker','inspect','stellar-local','--format','{{json .Image}}'])
    rpc('network','getNetwork')
    rpc('health','getHealth')
    run('core-info',['docker','exec','stellar-local','stellar-core','http-command','info'])
    run('core-sorobaninfo',['docker','exec','stellar-local','stellar-core','http-command','sorobaninfo'])
    meta = json.dumps({'pool_hash':POOL_HASH,'backstop':BACKSTOP,'blnd_id':ORACLE})
    factory = run('factory-deploy',['stellar','contract','deploy','--wasm',str(ART/'pool_factory.wasm'),*NET,*SRC,'--','--pool_init_meta',meta],timeout=120).strip()
    assert re.fullmatch('C[A-Z2-7]{55}',factory), factory
    pool = run('pool-deploy',['stellar','contract','invoke','--id',factory,*NET,*SRC,'--','deploy','--admin',ADMIN,'--name','Stage 1 baseline','--salt','15'*32,'--oracle',ORACLE,'--backstop_take_rate','1000000','--max_positions','4','--min_collateral','10000000'],timeout=120).strip().strip('"')
    assert re.fullmatch('C[A-Z2-7]{55}',pool), pool
    baseline = {}
    for fn in ['get_config','get_admin']:
        baseline[fn] = json.loads(run('baseline-'+fn,['stellar','contract','invoke','--id',pool,*NET,*SRC,'--send','yes','--',fn],timeout=120))
    run('raw-instance',['stellar','contract','read','--id',pool,*NET,'--output','xdr'])
    for name in ['factory-deploy','pool-deploy','baseline-get_config','baseline-get_admin']:
        record = json.loads((ROOT/(name+'.json')).read_text())
        hashes = re.findall(r'\b[0-9a-f]{64}\b',record['stderr'])
        for i,tx in enumerate(hashes):
            rpc(name+'-tx-'+str(i),'getTransaction',{'hash':tx})
    (ROOT/'baseline.json').write_text(json.dumps({'revision':revision,'factory':factory,'pool':pool,'pool_hash':POOL_HASH,'factory_hash':FACTORY_HASH,'baseline':baseline},indent=2)+'\n')
    print(json.dumps({'factory':factory,'pool':pool,'baseline':baseline},indent=2))
