# Stage 2 — reusable archival regression campaigns

**PASS.** One engine and one command ran a fresh Blend configuration/admin campaign
and an independent Tansu project/commit campaign against real local Core. Tansu's
mutable persistent commit record survived actual archival/restoration. Stage 1's
original corpus is unchanged. **Recommendation: CONTINUE TO MVP HARDENING; stop
at Stage 2 until separately authorized.**

The [machine summary](experiments/stage2/summary.json) contains exact source and
artifact hashes, contract IDs, ledger keys, ledgers, transaction hashes and evidence
references. The [evidence/reproduction guide](experiments/stage2/README.md) describes
prerequisites, commands and retained failures.

## Reusable interface and boundaries

```sh
PYTHONPATH=src python3 -m archival_regression run \
  scenarios/blend-config-admin.json --output /tmp/fresh-blend-campaign
PYTHONPATH=src python3 -m archival_regression run \
  scenarios/tansu-project-commit.json --output /tmp/fresh-tansu-campaign
```

These exact command forms ran successfully with project-local evidence outputs.
A fresh 32-byte deployment salt is resolved automatically. Existing outputs and
resolved configuration files are refused. Hooks use trusted local argv arrays,
without an engine shell or target-name dispatch.

| Layer | Actual responsibility | Files |
|---|---|---|
| Engine | Execute/capture hooks; validate local network; read explicit keys; decode SCVals; assert X/Y and live control; require exact Core eviction metadata for every key; verify RestoreFootprint receipts and restored contents; compare independent live raw before/after receipts; classify and checkpoint/retry. | `src/archival_regression/campaign.py` |
| Scenario model | Validate deployment/setup/mutation/observation/restore hooks, explicit persistent keys, raw projection bindings and bounded timing; resolve a fresh salt. | `src/archival_regression/scenario.py` |
| Reusable command | Invoke separate archive, negative, restore-checkpoint, resume/retry and final verification processes; enforce actual exits and checkpoint phases. | `src/archival_regression/__main__.py` |
| Target configuration | Artifact/source IDs, method/argument names, accounts, ledger-key SCVals, X/Y values, raw-field paths, restoration dependencies. | `scenarios/blend-config-admin.json`, `scenarios/tansu-project-commit.json` |
| Scenario-specific hook logic | Ordinary Stellar CLI deployment/invocation/restore commands supplied as argv. Neither selected campaign needed a bespoke executable hook. | The JSON `hooks` objects |
| Fixture generation/setup | Compute Tansu's declared Keccak project key, encode key arguments, build/upload unchanged code and prepare real SAC context. These are target input preparation, not lifecycle logic. | `docs/experiments/stage2/make_scenarios.py`, retained build/setup commands |

The engine contains no Blend, Tansu or Forge name/branch. Legacy instance-symbol
bindings remain supported for the original Stage 1 fixture. Instance storage is
one general representation; explicit complex persistent keys are another. Protocol
branches deal with ledger/XDR semantics and optional configuration capabilities.

A v2 scenario declares:

- `schema`, local `rpc_url`, `passphrase`, supported `protocol`, bounded timeout;
- `hooks.deploy`, optional `setup` and `mutate` lists, public getter hooks, and
  one or several `restore` actions;
- `entries`: named persistent ContractData LedgerKeys, with `{id}` for the
  deployed contract, including arbitrary supplied SCVal keys;
- `bindings`: public hook -> named raw entry -> JSON path to SCVal -> optional
  decoded-value path;
- optional `mutation_required`, `expected_initial` X and `expected_before` Y.

Invariant support is explicit value continuity and raw/public projections, with
X/Y checks when mutation is required. It is not an automatic invariant solver.

## Independent target inspection and selection

The [candidate inspection](experiments/stage2/candidate-inspection.md) records
actual storage, interfaces, deployment, TTL and build findings for Forge royalties,
StellarCred issuer registry and Tansu. This is bounded second-target integration
research, not a reopened project-discovery exercise.

Forge has meaningful cumulative settlement state, and its unchanged source built
offline. Its fixed 30-day TTL extensions need a different practical supported
profile. Core rejected two proposed maximum-TTL reductions (128 and 4096); neither
was applied or converted into a product FAIL. StellarCred also extends the inspected
persistent issuer paths for long periods. Neither project received an archival
campaign PASS, and neither was used as a trivial substitute.

**Selected:** Tansu project/commit registry, source revision
`0524376d293be41ee3bf5ca102cee2a88bea3d03`, SDK 27.0.4, unchanged WASM SHA256
`7962f42df518b6a24289d938b3f7e2f2a45d5d058a7781d04c9a1d1e86fc3cb6`.
Tansu is a project-versioning/membership/governance application. Its authenticated
`register`/`commit` path stores project ownership and last recorded commit in
persistent storage without extending these entries' TTLs. It fits the existing
verified short-minimum-TTL profile.

Build ran from an isolated archive of the pinned local Git revision. The locked
offline build initially lacked a cached dependency; approved registry downloads
allowed the unchanged build. A subsequent locked offline rebuild from retained
source, with cached dependencies, produced **byte-identical WASM**. Source, lockfile,
license, build diagnostics and artifact are retained. Original target repositories
were not modified. No maintainer endorsement/adoption is claimed.

Actual integration classification: **CONFIGURATION ONLY with the implemented v2
model**. Moving from Stage 1 required a **SMALL GENERAL PRODUCT GAP**: named ledger
entries, raw projections, setup/mutation phases and restoration action lists.
Those additions are target-independent and serve both scenarios.

## Mutable-state proof

Tansu contract: `CAIGXHTQTXLO2ZVQQYZTWWKXEEBGHV2WLGY3DUOMLLE25MKLCGWJ7RSG`.

Setup used the real constructor/admin configuration, a real Stellar Asset Contract
collateral reference, unpause, project registration and the first authenticated
commit call. Registration performed an actual collateral token transfer; no token
mock or edited fixture was substituted.

```text
initial get_commit X = 7bd159a9660d5764fb288977ba97356b3f19c72d
  -> authenticated public commit mutation
mutated get_commit Y = 0524376d293be41ee3bf5ca102cee2a88bea3d03
  -> exact Core eviction of instance/project/LastHash entries
  -> real Stellar restoration
post-restore get_commit = Y
raw LastHash entry = Y
complete observed instance/project/LastHash LedgerEntryData equal pre-archive Y state
```

X and Y are actual revision identifiers in Tansu's local Git history. This proves
preservation of the application's supplied version record, not remote Git authenticity.
The raw project entry also corroborates the maintainer list used for authorization.

## Actual Core archival and restoration

Runtime: local Quickstart, Core 29.0.0, active protocol/Host 28, Stellar CLI 28.1.0.
The preexisting Stage 1 profile remains: persistent minimum 64, temporary minimum
16, scan starting level 1, maximum TTL 3110400. Real sequential ledger closure was
used. **No Stage 2 network configuration change was applied.** No manual close was
enabled, no ledger sequence jumped, no snapshot edited, no entry deleted, and no
target TTL/source/WASM patched.

| Tansu observed key | Exact Core eviction ledger | Ledger hash |
|---|---:|---|
| Contract instance | 11720 | `859cf92652f006fb762b9a6e9390ac096bc3dea5ca4b624df1878889672e3be7` |
| `ProjectKey::Key(project_key)` | 11727 | `55626acf202ba8dfc125aaf9d7cc385ec3899b042f6742b4a59ce6cf576737fc` |
| `ProjectKey::LastHash(project_key)` | 11729 | `fba323309aff3d5d78c2a741f82f1f62aed919a74743feb7e3eed63affb59cda` |

Raw Core ledger-close v2 XDR explicitly contains each exact persistent key in
`evicted_keys`. The engine requires contiguous metadata coverage while observing
archival and independently redecodes the retained witnesses during verification.
TTL expiry, missing RPC entries, checkpoint flags or synthetic test data cannot
replace these witnesses.

Tansu's first restore batch contains real RestoreFootprint transactions at ledgers
**11734, 11737, 11739, 11741**, respectively restoring the WASM dependency, instance,
project and last-commit entries. Exact transaction hashes, footprints and raw v4
`restored` contents are retained in the summary/checkpoint and RPC receipts. Every
observed raw entry must occur with its exact contents in the restoration metadata.

The fresh Blend instance `CAX5YJSWW2MZBKFM3FTQ2OLT7M2RE43KGVMN3Z42NYYXWU75JU4IGCKT`
archived at **11636** and restored at **11641**, through the same v2 engine/model.
Its original nonempty configuration/admin invariant and complete raw instance match.

## Control, positive, negative and retry

| Case | Executed result |
|---|---|
| Control | Both scenarios repeated their public calls and independently read raw live state before inactivity. Values and complete declared raw entries were unchanged. |
| Positive | Exact Core eviction -> real restoration metadata -> post-restore getter equality -> independent live raw-receipt equality: PASS for both. |
| Negative | After proven eviction, omit explicit restoration; perform a real raw RPC availability observation without a getter. Required keys were unavailable as live entries. Verification exited **1**, `required explicit restoration omitted`. |
| Infrastructure distinction | A real host connection-refused probe exited **2 INCONCLUSIVE** before deployment; a verifier process missing post-restore raw evidence also exited 2. |
| Retry/interruption | Exit after the committed restore checkpoint, before getter recovery; a new process retries the declared restore actions, then checks getters/raw state. Both pass. All four Tansu retries have successful empty footprints, the real already-live no-op behavior. |

The negative enforces the campaign's explicit recovery policy and detects unavailable
live state. It does not claim that a getter with protocol automatic restoration would
fail. RPC may retain archived XDR with TTL 0; that response is not used as the archive
proof. The independent Core witnesses are mandatory before the negative.

Retry is deliberately bounded to a committed lifecycle-stage boundary. No in-flight
crash, unacknowledged transaction, partial atomic restore or application mutation replay
is claimed. Successful replay of the already-live restore cannot replace the required
first `restored` metadata or unchanged raw observations.

## Tests, automation and evidence preservation

**53 local behavior tests PASS**, Python compilation PASS. Existing 30 tests remain;
new tests cover scenario validation, hook failures, independent target representations,
mutable-value mismatch, all-key evidence completeness, missing restoration/raw evidence
and classification. Retained real Core/transaction evidence is used for offline verifier
tests; unit-test synthetic scalars test parsing only and are not archival proof.

Local CI command: `scripts/check.sh`. A hosted GitHub Actions workflow is configured
for the dependency-free scenario/preflight tests. Its **30-test subset also passed
locally**. **Hosted CI has not run**; no remote/push/publication was authorized.
Live Core campaigns and CLI-XDR evidence tests are local gates.

The final verifier rechecked both fresh Stage 2 campaigns and all five original Stage 1
records. Original records were read/reverified in disposable copies. The entire prior
Stage 1 evidence corpus and flagship report remain byte-for-byte unchanged, with no
new files in the original corpus. The original Stage 1 engine bytes are preserved under
`experiments/stage2/engine-before/` with their historical SHA256. Stage 1's source-hash
records remain historical; current source hashes are recorded separately for Stage 2.

Exact final source hashes are in `summary.json`; Git `main` remains unborn, with no
commit or remote. The live execution source evolved with general evidence-index and
exception-guard fixes afterward; all records were reverified with the final source.
No historical failure or source identity was relabeled.

## Exit gate and final answers

| Requested finding | Outcome |
|---|---|
| 1. Stage 2 | **PASS** within the declared local profile/invariant scope. |
| 2. Reusable interface | One v2 JSON campaign model and `python -m archival_regression run SCENARIO --output DIR`. |
| 3. Target-specific remainder | Only configuration, supplied invariant values/keys, ordinary CLI hooks and artifact/setup prerequisites. |
| 4. Second target | Independent unchanged Tansu project/commit registry. |
| 5. Mutable state | Authenticated X -> Y mutation; public and raw state preserve Y after recovery. |
| 6. Real archival | Exact instance/project/last-hash eviction metadata at 11720/11727/11729. |
| 7. Restore | Real RestoreFootprint receipts and exact restored contents, four-action batch. |
| 8. Control | PASS for both fresh scenarios. |
| 9. Positive | PASS for both fresh scenarios. |
| 10. Negative | Expected unavailable-live-state/lifecycle failure, actual exit 1. |
| 11. Retry | Separate committed-stage processes; real already-live restore retries PASS. |
| 12. Raw verification | All named raw entries/receipts and restored contents independently checked; missing evidence fails closed. |
| 13. Tests/CI | 53 local tests and compilation PASS; 29 hosted-suite tests passed locally; hosted workflow configured, not run. |
| 14. Integration classification | CONFIGURATION ONLY after small general-purpose engine additions. |
| 15. Reusable tool? | **YES**, two applications use the same engine and model; not a universal/production-ready archival platform. |
| 16. Recommendation | **CONTINUE TO MVP HARDENING**, requiring new authorization; no automatic Stage 3. |

All 13 requested Stage 2 exit requirements pass in this supported scope. The engine
provides multi-entry lifecycle/evidence assertions, mutable-state verification and
failure/retry policy beyond calling the existing restore command. No complete competing
workflow was discovered in bounded candidate inspection; this does not establish global
novelty or remove the previously documented upstream displacement risk.

Limitations: fixed local protocol/XDR profile; trusted commands and funded signing
identity; declared keys and dependencies; equality/projection invariants; prerequisite
factory/artifact/SAC setup; no broad Tansu DAO/membership/NQG or collateral recovery;
no production/default-network schedule proof; no hosted campaign/adoption/endorsement.
Forge's long-extension accelerated profile remains unresolved. The local Stage 1 test
profile remains applied. No other product/research directory was changed, and no
public Stellar deployment, push, PR, release, outreach or Stage 3 occurred.
