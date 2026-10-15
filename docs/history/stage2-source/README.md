# Core-backed Soroban Archival Regression Tester

Run a declared persistent-state scenario through real local Core archival,
restoration, retry and invariant verification. The runner requires raw Core eviction
metadata and restoration receipts; TTL expiry and missing RPC data cannot pass.

**Stage 2 passed:** the same engine ran unchanged Blend configuration/admin continuity
and independent Tansu project/commit continuity. Tansu's authenticated X -> Y mutation
survived actual archival, with independent raw-state corroboration. Stage 1's original
proof remains unchanged. [Stage 2 report](docs/stage2-reusability.md) ·
[raw evidence and reproduction](docs/experiments/stage2/README.md).

From a configured local node with the recorded profile, uploaded artifacts, fixture
addresses and funded `archival-admin` identity:

```sh
PYTHONPATH=src python3 -m archival_regression run \
  scenarios/blend-config-admin.json --output /tmp/fresh-blend-campaign
PYTHONPATH=src python3 -m archival_regression run \
  scenarios/tansu-project-commit.json --output /tmp/fresh-tansu-campaign
scripts/check.sh
```

Each JSON scenario supplies ordinary deployment/setup/mutation/getter/restore argv
hooks, named persistent ledger keys, raw-value bindings and expected values. The
engine has no target-name branches. It records actual transactions/RPC/XDR, validates
all named archive/restore witnesses and raw observations, and uses separate processes
for the committed restore checkpoint and retry. Fresh salts are generated automatically;
existing output directories are refused.

Exit 0 means the supported campaign passed; exit 1 means a declared assertion failed;
exit 2 means infrastructure/evidence is missing or a lifecycle checkpoint is incomplete.
The enclosing `run` command checks its deliberate negative's exact exit and reason.

Supported scope is the pinned local protocol-28 profile and declared ContractData
continuity assertions. Network/bootstrap/artifact preparation is explicit. This is
not universal invariant discovery, production/default-network archival proof or full
Blend/Tansu coverage. Retry covers a committed-stage boundary. The negative enforces
explicit restoration policy; it does not assert that protocol automatic restore fails.

53 local tests pass. Hosted engine-test coverage is configured but has not run.
No public deployment, release or Stage 3 occurred. Further work requires authorization.
