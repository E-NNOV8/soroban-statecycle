# Current architecture and data flow

StateCycle has a Python standard-library campaign engine and a fixture-specific
local bootstrap. There is no target-name dispatch in `src/archival_regression/`.

| Boundary | Implementation and data |
| --- | --- |
| Scenario → runner | `scenario.py` validates v2 JSON, trusted argv hooks, named persistent ContractData keys, projections and optional X/Y assertions. `__main__.py` supplies a fresh salt and invokes separate phase processes. |
| Runner → application | `campaign.py` runs supplied CLI argv without an implicit shell, records stdout/stderr, obtains transaction hashes and requires successful RPC receipts. Application-specific methods/keys/dependencies live in JSON and `scripts/setup_local.py`. |
| Runner → RPC → Core | Plain loopback HTTP only, proxies disabled and redirects refused. RPC exposes Core ledger-close metadata and transaction metadata; the engine does not implement eviction or edit ledger state. |
| Evidence → verifier | Numbered immutable command/RPC records, checkpoint references, actual XDR and raw SCVal normalization back assertions. The checkpoint alone cannot replace mandatory raw witnesses. |

## Provisioning and live state

`scripts/setup_local.py` starts a new pinned Quickstart container, creates/funds
local identities and uploads hash-checked unchanged artifacts. It deploys the real
Blend factory and real SAC dependency used by Tansu. A real consensus upgrade sets
minimum persistent TTL 64, temporary TTL 16 and eviction scan level 1; raw readback
must match. Existing containers/output directories are refused. This script contains
target-specific fixture provisioning; the lifecycle engine does not.

Each campaign deploys a fresh target. Optional setup runs, then optional authenticated
mutation with public/raw X and Y observations. Before inactivity the engine records
public results and exact live LedgerEntryData. Repeated public/raw calls provide
the unchanged live control. Type-sensitive equality prevents boolean/integer aliasing.
Bindings project declared SCVals into public JSON; large integer SCVals remain strings.

## Genuine archival boundary

The engine polls RPC `getLedgers` from the recorded live ledger, requiring contiguous
sequence coverage. It decodes `LedgerCloseMeta` v2 using Stellar CLI and checks
ledger number/hash consistency. **Every declared persistent key must occur exactly
in Core's `evicted_keys` after its recorded live-until ledger.** Missing RPC entries,
expired TTL or a checkpoint phase cannot establish archival. Metadata retention gaps,
unsupported XDR or timeout classify INCONCLUSIVE. No application calls are made
while waiting, so the runner does not accidentally extend the observed state.

The Stage 1 pilot additionally retained a real hot-archive bucket record. Current
campaigns use Core eviction metadata and restoration metadata; they do not claim
that every repetition directly scans a hot-archive bucket.

## Negative, restore and retry

After proven archival, `verify` without restore observes raw unavailability without
a getter (which might auto-restore), then exits 1 for required explicit restoration
omission. This deliberate policy failure is required by the enclosing campaign.

Restore hooks submit real Stellar CLI restoration actions. Successful receipts must
decode to one `RestoreFootprint` operation and TransactionMeta v4. The first batch
must target all declared keys and contain their exact pre-archive LedgerEntryData
in actual `restored` changes, after the archive ledgers. Code dependencies can be
restored by earlier actions in the same declared batch.

The restore process saves `restored-checkpoint` and exits. A new process retries
restoration against now-live state, reads public results and independent raw live
entries, and compares them with Y/pre-archive state. Already-live successful empty
footprints are legitimate retries; they cannot substitute for the first batch's
real restored contents. This models interruption **after a committed stage**, not
an in-flight crash, unacknowledged submission or partially committed transaction.

## Recovery verification versus whole-campaign result

Offline `--phase verify` redecodes exact eviction witnesses and successful restore
receipts, checks restoration timing, validates before/after raw RPC records and
compares raw/public recovery invariants. It verifies recovery, not independently
re-execution of every historical getter or cryptographic authentication of RPC.
The enclosing `run` checks the complete phase sequence and writes final PASS only
after all expected process exits/checkpoints, control, negative and retry succeed.
The retained-hosted verifier additionally checks producer/digest/lifecycle records
before reusing that recovery verifier in a temporary copy.

| Result | Meaning |
| --- | --- |
| PASS / 0 | Required proof and assertions for the executed command succeeded. A recovery-only verifier is narrower than a full live campaign. |
| FAIL / 1 | An invariant or explicit-restoration policy was violated. Expected negative exit 1 is checked separately inside a successful campaign. |
| INCONCLUSIVE / 2 | Required infrastructure/evidence/schema support is unavailable, or the command intentionally stopped at an incomplete lifecycle checkpoint. |
| NOT TESTED | Descriptive coverage status for behavior never executed; not a campaign-success exit code. |

`runner.json` records the full sequence with expected exits `[2,1,2,0,0]`.
`config.json` preserves exact bytes and resume refuses configuration changes.
`checkpoint.json` references evidence files and stores raw/public values and batches;
`campaign-result.json` exists only on complete enclosing success. Reports for repeated
phase checks get distinct names. Preserve the complete directory and its resolved
scenario, not only the PASS label. See [configuration](scenarios.md),
[evidence integrity](evidence.md) and [limitations](limitations.md).
