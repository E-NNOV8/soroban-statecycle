"""Apply a genuine consensus settings upgrade on the isolated local network."""
import json
import time
from capture import ROOT, run, rpc
from provision import ADMIN, NET

def enc(name, typ, obj):
    return run(name,['stellar','xdr','encode','--type',typ],stdin=json.dumps(obj)).strip()

def dec(name, typ, xdr):
    return json.loads(run(name,['stellar','xdr','decode','--type',typ,'--input','single-base64',xdr]))

if __name__ == '__main__':
    keys = [enc('archival-config-key-v2','LedgerKey',{'config_setting':{'config_setting_id':'state_archival'}})]
    original = rpc('archival-config-entry','getLedgerEntries',{'keys':keys})
    entry = dec('archival-config-decoded','LedgerEntryData',original['entries'][0]['xdr'])
    settings = entry['config_setting']['state_archival']
    settings['min_persistent_ttl'] = 64
    settings['min_temporary_ttl'] = 16
    settings['starting_eviction_scan_level'] = 1
    upgrade = {'updated_entry':[{'state_archival':settings}]}
    (ROOT/'short-ttl-upgrade.json').write_text(json.dumps(upgrade,indent=2)+'\n')
    xdr = enc('short-ttl-upgrade-xdr','ConfigUpgradeSet',upgrade)
    account = json.loads(run('upgrade-admin-sequence',['curl','--noproxy','*','-sS','http://localhost:8000/accounts/'+ADMIN]))
    seq = account['sequence']
    output = run('settings-upgrade-transactions',['docker','exec','stellar-local','stellar-core','get-settings-upgrade-txs',ADMIN,seq,'Standalone Network ; February 2017','--xdr',xdr])
    lines = output.strip().splitlines()
    assert len(lines) in [7,9], lines
    for i in range(0,len(lines)-1,2):
        signed = run('upgrade-sign-'+str(i),['stellar','tx','sign',*NET,'--sign-with-key','archival-admin'],stdin=lines[i]).strip()
        run('upgrade-send-'+str(i),['stellar','tx','send',*NET],stdin=signed,timeout=90)
        result = rpc('upgrade-transaction-'+str(i),'getTransaction',{'hash':lines[i+1]})
        assert result['status'] == 'SUCCESS', result
    key=lines[-1]
    run('schedule-settings-upgrade',['docker','exec','stellar-local','curl','-sSG','http://localhost:11626/upgrades?mode=set&upgradetime=1970-01-01T00:00:00Z','--data-urlencode','configupgradesetkey='+key])
    for i in range(20):
        current=rpc('config-upgrade-check-'+str(i),'getLedgerEntries',{'keys':keys})
        actual=dec('config-upgrade-check-decoded-'+str(i),'LedgerEntryData',current['entries'][0]['xdr'])
        if actual['config_setting']['state_archival']==settings:
            print('Local consensus settings upgrade applied',current['latestLedger']); break
        time.sleep(1)
    else:
        raise RuntimeError('settings upgrade not observed')
