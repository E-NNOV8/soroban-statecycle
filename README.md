# Soroban StateCycle

Archival and restoration regression testing for Soroban applications.

Soroban persistent state can leave the live ledger and require restoration. A
successful restore transaction alone does not establish that an application's
state and public behavior survived. StateCycle runs declared invariants against
both contract calls and independent raw ledger evidence, and refuses PASS when
required evidence is missing.

```text
live state → real Core archival → restoration → invariant verification
           → negative / retry verification → retained RPC, XDR and receipts
```

SDK-only TTL tests do not execute Core's eviction scanner, hot archive or actual
RestoreFootprint transactions. StateCycle requires Core eviction metadata and
restoration receipts for every declared key; expired TTL or missing RPC data alone
cannot establish archival.

For Soroban contract developers and reviewers who need a repeatable test of
persistent-state continuity, the scenario supplies application calls, explicit
ledger keys and assertions; the engine handles the lifecycle and raw proof.

This is a **0.1.0-alpha local developer tool**, with a deliberately narrow profile.
[Stage 1](docs/stage1-flagship.md) passed on Blend V2, including five fresh-state
repetitions. [Stage 2](docs/stage2-reusability.md) passed on Blend and independent
Tansu using the same engine. Tansu's authenticated commit mutation X → Y survived
real archival/restoration, corroborated by raw state. These are bounded regression
proofs, not audits or complete coverage of either application.

## Run locally

Prerequisites: Python 3.12 or later, Stellar CLI **28.1.0**, Docker Engine with a
working daemon, and `curl`/`tar` for the optional CLI installer. The pinned Linux
Quickstart image runs Core 29.0.0 with active **protocol 28**. Allow several minutes
and sufficient Docker memory/disk. See [versions and setup](docs/reproducibility.md).
No Python package dependencies are required.

From the checkout root, install the pinned Linux x86_64 CLI if needed:

```sh
bash scripts/install_stellar.sh .local/bin
export PATH="$PWD/.local/bin:$PATH"
```

Provision a new local network and both fixture dependencies, then run a campaign:

```sh
python3 scripts/setup_local.py
PYTHONPATH=src python3 -m archival_regression run \
  .local/statecycle/scenarios/tansu-project-commit.json \
  --output outputs/tansu-1
```

Use `.local/statecycle/scenarios/blend-config-admin.json` for Blend. Use a fresh
output directory for each run. Setup refuses existing containers/output directories;
use `--port`, `--container` and `--output` to coexist with another local network.
It changes only its new development network's archival settings through a real
consensus upgrade. Signing identities remain in ignored local files. Root
`scenarios/` files are the original fixture templates; setup supplies fresh addresses.

## Results and evidence

| Result | Meaning |
| --- | --- |
| PASS (exit 0) | All required observations and assertions passed for this campaign/profile. |
| FAIL (exit 1) | A tested assertion or required explicit-restoration policy was violated. |
| INCONCLUSIVE (exit 2) | Infrastructure, evidence or a completed lifecycle boundary is missing. |
| NOT TESTED | A behavior outside the executed scenario; no success or failure claim. |

The runner deliberately expects the missing-restore negative to exit 1 and requires
its exact reason. It then commits restoration, starts a separate recovery process,
retries restoration and verifies public/raw equality. A final
`campaign-result.json` reports the enclosing result; `checkpoint.json`,
`runner.json` and numbered evidence files preserve commands, RPC responses, ledger
numbers and transaction metadata. INCONCLUSIVE must never be treated as product FAIL.

[Scenario reference](docs/scenarios.md) documents hooks, keys, bindings and outputs.
Hooks execute trusted local commands: review a scenario before running it.

## Verify the checkout

```sh
bash scripts/check.sh
python3 scripts/verify_evidence.py
python3 scripts/check_reproducibility.py
```

Evidence verification hashes the retained corpus and replays both Stage 2 campaigns
and all five Stage 1 records offline in disposable copies. It needs the pinned CLI
for XDR decoding, but no running node. [Evidence integrity](docs/evidence.md) explains
historical paths and manifests.

Hosted proof **passed on producer
`c19751db48702fe075b24da749a65a611ba3d1b6`**:
[fast checks](https://github.com/mxrtins04/soroban-statecycle/actions/runs/38807089360)
(57 tests, compilation, retained replay) and
[real Core integration](https://github.com/mxrtins04/soroban-statecycle/actions/runs/38807146800)
(actual Blend and mutable Tansu campaigns). [Durable hosted evidence](docs/hosted-proof.md)
retains application proof bytes, logs, provenance and an explicit diagnostic redaction in this repository.
The hardened producer
`615522022c7a8fdcc957b111fb13073cba47094d` also passed [68-test fast CI](https://github.com/mxrtins04/soroban-statecycle/actions/runs/38813239421)
and [both real Core campaigns](https://github.com/mxrtins04/soroban-statecycle/actions/runs/38813244977); its separate
[durable bundle and replay command](docs/hosted-proof.md#hardened-producer-validation)
are retained. Documentation-only commits do not change either producer attribution.

Fast CI runs on pushes and pull requests. Expensive Core integration runs **only
when manually dispatched**; a green fast check alone is not new live archival proof.
The hosted retention verifier is also runnable offline:

```sh
python3 scripts/verify_hosted_evidence.py
```

[Architecture](docs/architecture.md) explains the actual Core/RPC boundary and
checkpoint model. [Limitations and trust boundaries](docs/limitations.md) and
[contributing](CONTRIBUTING.md) describe supported use and substantive future work.

## Limits

Only the pinned local protocol-28 profile and declared persistent ContractData
keys/projection-equality assertions are supported. The accelerated development TTL
profile is not a production-network configuration. No target discovery, arbitrary
invariant inference, production deployment or universal restoration guarantee is
provided. Retry covers a committed restoration checkpoint across processes, not
an interrupted atomic transaction. The negative detects unavailable live state and
enforces explicit restoration; it does not claim Stellar automatic restore fails.

First-party code is [MIT licensed](LICENSE). Retained target artifacts and source
have their own licenses; see [third-party notices](THIRD_PARTY_NOTICES.md).
