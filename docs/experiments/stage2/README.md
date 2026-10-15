# Stage 2 evidence and reproduction

**Result: PASS.** Read [the report](../../stage2-reusability.md) and
[summary.json](summary.json) for exact values/IDs/ledgers/transactions/source hashes.
Stage 1 evidence remains unchanged under `../stage1/`.

## One command per campaign

Run from the project root, with Stellar CLI 28.1.0 and a working funded local
`archival-admin` identity:

```sh
PYTHONPATH=src python3 -m archival_regression run \
  scenarios/blend-config-admin.json --output /tmp/fresh-blend-output
PYTHONPATH=src python3 -m archival_regression run \
  scenarios/tansu-project-commit.json --output /tmp/fresh-tansu-output
```

The commands resolve fresh salts, preserve resolved configs, run all lifecycle
phases in separate processes, and return 0 only when control, real archival,
intended negative, committed-stage restore retry and public/raw verification pass.
A negative subphase exits 1 and is explicitly checked by the enclosing command.
Incomplete checkpoints and infrastructure/evidence errors exit 2. The enclosing
command does not treat an arbitrary exit 2 as proven archival: it checks the saved
archived phase before proceeding.

Actual executions used `blend-live/` and `tansu-live/`; see their `runner.json`,
`campaign-result.json`, checkpoint and raw `evidence-NNNNN.json` files. The parent
`*-execution.json` files retain the outer process's actual exit, stdout and stderr.
Resolved `*.scenario.json` files preserve the exact fresh salts used.

## Local prerequisites and artifacts

The supplied scenarios target the existing local Quickstart node and its previously
funded identity. Runtime: Docker image ID
`sha256:4f8dfd693deba57fddef7f0bc722e4782c2e606c8675e3adc5438cebae249295`,
Core 29.0.0 / active protocol 28, loopback RPC `http://localhost:8000/rpc`, passphrase
`Standalone Network ; February 2017`. Stage 1's legitimate consensus profile is
required for bounded execution: min persistent TTL 64, min temporary TTL 16,
starting scan level 1; other settings including maximum 3110400 are retained.
Profile/version/network/image records are preserved here. Default-network latency
is not established by these campaigns.

`artifacts/` retains the exact unchanged pool/factory and Tansu WASMs. Source IDs:

- Blend `ba22b487b2c5057a4ecc28b05b5193c28e4bd117`;
- Tansu `0524376d293be41ee3bf5ca102cee2a88bea3d03`.

The Blend fixture supplies its already-deployed Stage 1 factory ID. Tansu supplies
the already-deployed real SAC collateral reference from `sac-deploy.json`.
On another local chain, fund/configure the identity, upload the supplied WASMs,
deploy the genuine factory with the supplied pool hash and valid placeholder
addresses, and deploy the genuine custom-asset SAC; update those declared fixture
addresses. No engine changes are required. A reset node with missing prerequisites
is not equivalent to the demonstrated configured node and cannot pass by omission.

Example ordinary artifact preparation on a configured local node:

```sh
stellar contract upload --wasm docs/experiments/stage2/artifacts/tansu.wasm \
  --source archival-admin --rpc-url http://localhost:8000/rpc \
  --network-passphrase 'Standalone Network ; February 2017'
stellar contract asset deploy \
  --asset ARST2:GBPQSQQRS34EPU6QF3RLGHIHA7NQHNYQ4UUUSIYR6T4F5FLLIHQZMQEN \
  --source archival-admin --rpc-url http://localhost:8000/rpc \
  --network-passphrase 'Standalone Network ; February 2017'
```

The runtime command requires uploaded code; target build and fresh-network bootstrap
are explicit prerequisites rather than hidden engine behavior. Archived code can
be legitimately restored/uploaded with the normal CLI before deployment if needed.
No private key is retained; config/hooks use the existing account alias.

## Build/source reproducibility

Tansu's retained source/lockfile/license are under `sources/tansu/`. The locked
initial build missed a cached crate; the approved package-registry download/build
log and actual command are retained. An offline locked rebuild from retained source
with cached dependencies matched the deployed WASM byte-for-byte:

```sh
cargo build --offline --locked --release --target wasm32v1-none \
  --manifest-path docs/experiments/stage2/sources/tansu/Cargo.toml \
  --package tansu --target-dir .local/stage2/tansu-build
```

See `tansu-source-hashes.json`, `tansu-build-network.log`,
`tansu-retained-source-rebuild.json` and `tansu-rebuild-comparison.json`.
Original target checkouts were inspected/read, not modified. Blend artifacts are
those independently pinned and deployed in Stage 1; no new Blend build is claimed.

## Raw proof locations

For each campaign, `checkpoint.json` references the exact RPC evidence files for:

- public getter transaction receipts, setup and mutation transactions;
- before/control/after raw ledger state;
- every exact key's Core `evicted_keys` metadata witness and ledger hash;
- the raw unavailable-live-state negative observation;
- first RestoreFootprint receipts, footprints and actual `restored` contents;
- separately executed successful already-live restore retries.

Every evidence file preserves argv/stdin/stdout/stderr or RPC request and raw JSON
response. Base64 XDR remains available alongside its decoded CLI output. Missing
required raw receipts cannot be replaced with checkpoint fields. RPC-retained
archived contents with TTL 0 are not independent archive proof; the Core witnesses
must already be verified.

`reverify.py` revalidates both new campaigns and all five original Stage 1 records
on disposable copies, checks a real missing-evidence process exit 2, and verifies
that Stage 1 corpus bytes/paths were unchanged. `engine-before/` retains the exact
historical Stage 1 engine bytes. Stage 1 checksum records remain historical; Stage 2
has separate current-source hashes and checksums.

## Tests and CI

```sh
scripts/check.sh
```

`tests-final.log`: 53 local tests PASS and compilation PASS.
`hosted-suite-local.log`: configured hosted scenario/preflight subset, 30 tests PASS
locally. `.github/workflows/engine-tests.yml` is configured only; no hosted run,
remote, push or publication occurred. Real campaigns and CLI-XDR evidence replay
remain local checks.

`unavailable-host/` and `unavailable-host.log` retain a real host connection-refused
probe, exit 2 before deployment. `final-reverification.json` also records exit 2
when post-restore raw evidence is missing. Expected-negative exit 1 is distinct.

## Retained unsuccessful attempts and limits

Forge royalties was inspected and built, but no archival campaign was run for it.
Its 30-day extensions led to two attempted maximum-TTL consensus upgrades (128 and
4096). Both were rejected, and exact payloads, submitted upload/deploy transactions,
Core validation diagnostics and rejected activations remain here. The existing
working profile was never replaced. Manual-close diagnostic requests were rejected;
manual mode was not enabled and no ledger jump/configuration change occurred.
`prepare_host.py` and `apply_supported_profile.py` are retained failed experiment
scripts, **not a successful bootstrap recipe**. Tansu avoids those unresolved
long-extension requirements using its real unmodified persistent-state path.

Scope: explicit persistent ContractData keys, protocol 28/XDR v2/v4, trusted argv
hooks, supplied continuity assertions, configured local node and funded identity.
Tansu's project/last-commit ownership state is covered; its project indexes, remote
Git authenticity, DAO/NQG/membership and collateral balances/recovery are not.
Retry is after a committed restore checkpoint, not an in-flight transaction crash.
The negative checks explicit recovery policy and unavailable live state, not a
claim that protocol automatic restoration fails. No Stage 3 is authorized.
