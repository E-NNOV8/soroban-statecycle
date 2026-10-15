# Stage 1 flagship findings

2026-10-15 host continuation: **PASS for the bounded configuration/admin continuity
campaign, with five fresh-state repetitions. Stage 1 is complete; stop here.**
Earlier INCONCLUSIVE infrastructure results are preserved below as historical.
No default-network, lending-position, production-safety or Stage 2 claim is made.

## Verified target and environment

- Blend V2 revision: `ba22b487b2c5057a4ecc28b05b5193c28e4bd117`.
  Agent verified the revision, empty tracked-source diff and artifact hashes;
  Blend build/tests passing are owner-proven prerequisites, not agent reruns.
- Optimized pool SHA256: `274d85836c4e9adf8995f9240c6452cb0ce2e4751e1b10baffffdbfb90319ba1`.
- Optimized factory SHA256: `e304170e67184748839dda5fd3b6dd5d8bf1e54055a07b536cede24da072c7a9`.
- Core v29.0.0, active protocol 28 / Host 28.0.0; Stellar CLI 28.1.0;
  exact bundled Host/XDR identities retained in `core-version.json`.
- Docker image ID: `sha256:4f8dfd693deba57fddef7f0bc722e4782c2e606c8675e3adc5438cebae249295`.
- Preexisting Quickstart `stellar-local`, loopback `http://localhost:8000/rpc`,
  passphrase `Standalone Network ; February 2017`. Host services were reached
  through local Codex CLI process escalation; no remote executor/public network.

Factory: `CA3JX42W32345O6AIUO3GJGBXHP2VZCAHQBZOC3RKJ6QAVSNG2VBKMYC`.
Original baseline pool: `CBTTZI3XSOH7UG3YRTRUOI2MMNDUPNNY76KNTMHSDWITU6PKWP7JKI7I`.
Factory deployment transaction:
`a757e0bfbe720b740235b49328e290d81adf61a12ce4834c06ae418ea29c59d4`.
Original factory-to-pool deployment transaction:
`d9a1fb4388de23824a6c3b15cab7e38e4aba20c5e2a47f21b6f8b2f92c09c145`.
The real unchanged factory supplied the unchanged pool hash and valid placeholder
account addresses. Initialization/getters do not exercise external dependencies.

## Invariant and exact raw keys

Observed `get_config`:

```json
{
  "bstop_rate": 1000000,
  "max_positions": 4,
  "min_collateral": "10000000",
  "oracle": "GDIGJ6OENHTHPP6TZA5C5NV2D5JD5TEHIZKWT6OYTUP7YSB4ZHVKU7HJ",
  "status": 6
}
```

Observed `get_admin`:
`GBPQSQQRS34EPU6QF3RLGHIHA7NQHNYQ4UUUSIYR6T4F5FLLIHQZMQEN`.

Both values reside in the same persistent contract-instance ledger entry:
`LedgerKey::ContractData(contract=<pool>, key=LedgerKeyContractInstance,
durability=Persistent)`, under instance symbols `Config` and `Admin`.
The instance executable references the exact pool hash. Code and TTL keys are
also retained; TTL key hash is SHA256 of the instance LedgerKey XDR. RPC disallows
direct TTL-key reads; per-entry TTL fields and transaction metadata are preserved.

For each fresh pool, repeated getters and raw-instance comparisons form the live
unchanged control. After real archival and RestoreFootprint, both public values
and the **entire decoded LedgerEntryData instance** match the pre-archive values.
This includes nonempty administrator, oracle, configuration and pool name.

## Legitimate Core archival and recovery

A real on-ledger consensus settings upgrade was verified at ledger **4457**:
minimum persistent TTL 120960 -> 64; minimum temporary TTL 720 -> 16;
starting eviction scan level 7 -> 1. Other archival parameters were preserved.
Quickstart already accelerates real sequential ledger closure to about one second.
No ledger jump, manual deletion, edited snapshot, altered Blend source, or patched
WASM was used. Existing entry TTLs were not shortened; the original baseline
instance remains on its long TTL (124987). Fresh pool instances use the new minimum.
The local short-TTL/scan profile remains applied at handoff.

Pilot pool `CDWTOBSFCURUQ3XLVEFUPNB7GKDRNLYKMRGGZ6W2ROFN3Y6KQIDTQJOC`:
Core ledger-close v2 metadata explicitly evicted its persistent instance at
**4582**, ledger hash
`7772fe213cbb2fa621d39ac2a1b36a3bb479afe179adcc7ea3224f18ace02c4b`.
Independently, direct Core history anchor ledger **4687** names hot-archive bucket
`d179d2a922cb49a316ae96ac132ae95e46d73616b9e9a7a2f03dbec19cce8c58`.
The copied immutable bucket's SHA256 matches that name, and its actual `archived`
XDR record contains the full instance equal to the live baseline.

Pilot RestoreFootprint transaction
`13531d6a97e4f11e2d4d92b2a5779da04a6fd615b0f60ed3c88bf5398e27180c`
at ledger **4725** has actual `restored` metadata for the instance and TTL.
Public getters and complete raw instance match after restoration.
RPC can retain archived contents with `liveUntilLedgerSeq: 0`; neither missing
RPC data nor TTL arithmetic is used as the proof of archival.

## Five fresh-state repetitions

One Stage 1 campaign, five distinct initialized pool instances, shared unchanged
factory/code and same local chain. No network reset or fixture substitution.
All five were reverified with the final verifier after execution.

| Run | Fresh pool ID | Live ledger / TTL | Exact Core eviction ledger | First restore ledger | Control / negative exit / recovery+retry |
|---|---|---|---|---|---|
| 1 | `CDXTNWGN2ZM2LMCH4ZTL74CBTD4DDKMBAIAD7UJPWY7DQLZAY72GRW3X` | 4844 / 4901 | 4902 | 4906 | PASS / 1 / PASS |
| 2 | `CCYKZIH7LTRXHPSLVUR3SQPL4ZFPTJAMWXIBSM6MHZ567W4OAOKFVY6H` | 4988 / 5045 | 5046 | 5050 | PASS / 1 / PASS |
| 3 | `CDCS6IS2ZGBDTZRFUHPBEWYUWXJUMCHLAUBAWMVSJUQ6J55LDTN5NFAJ` | 5067 / 5124 | 5125 | 5129 | PASS / 1 / PASS |
| 4 | `CBAOOZFZYCY2QMXOLMT5EQHXB4P5HWCFTNGVPTYUNJGRCBRH3V2IM3DI` | 5145 / 5202 | 5203 | 5208 | PASS / 1 / PASS |
| 5 | `CAUZLF2W3MGXHERIHYAMKNBZFJJUBV6YXAYFRK5ECI3MSTYV3IJRBE6S` | 5224 / 5282 | 5283 | 5287 | PASS / 1 / PASS |

For each run, deliberate omission of the required explicit restore phase produces
**FAIL exit 1**, with reason `required explicit restoration omitted`, after actual
Core archival is already proven. This is a lifecycle-policy negative; it does not
claim a Blend bug or failure of protocol automatic restoration. No post-archive
getter is invoked in this negative, so automatic restoration cannot mask omission.
A separate real host connection-refused probe produces **INCONCLUSIVE exit 2**
before any provisioning. Missing raw evidence also cannot pass the verifier.

The interruption boundary is a committed RestoreFootprint checkpoint before getter
recovery. That process exits; a new process resumes and retries restoration.
Stellar may submit a successful empty-footprint RestoreFootprint for an already-live
entry. The runner requires a previously proven restore and fresh unchanged live
raw state to accept it. This proves a bounded committed-stage retry, not an in-flight
crash, partial atomic transaction, or node-failure recovery.

## Implementation, checks and retained failures

`src/archival_regression/campaign.py` implements declared argv hooks, raw RPC/XDR
capture, exact eviction observation, restoration receipt validation, public/raw
comparisons, checkpoint/resume, negative enforcement and distinct exit policies.
There is no Blend-specific engine branch. Blend details are ordinary experiment
inputs and symbol bindings. Actual integration classification:
**SMALL GENERAL PRODUCT GAP**, now implemented locally for this profile.

Final verification: **30 behavior tests PASS**, Python compilation PASS, and five
final retained-evidence reverifications exit 0. Verifier tests use retained real
observations; tests themselves are not archive proof. Lint remains NOT TESTED
(`ruff` unavailable). Build/test prerequisites for Blend are owner-proven; no new
independent Blend/Core build was executed. Project branch `main` remains unborn,
with no commit or remote; exact final source/test SHA256 values are in the summary.
Final campaign module SHA256:
`f62ecc2927d310f37af2d8591ba708d753c117923a5f83d631d6f7aa2c91d718`.

Preserved diagnostics include sandbox socket/loopback failures, unsupported TTL
query, XDR-key encoding failure, read-only dump lock/path failures, missing upgrade
payload/rejected activation attempts, and the first retry-verifier INCONCLUSIVE
result. The installed Core helper emitted restore/upload/deploy envelopes without
the payload invocation; the real upgrade contract was explicitly invoked to finish
the upgrade. No infrastructure failure was recast as an expected product negative.

Current evidence and reproduction:
[host guide](experiments/stage1/host-resume-20261015/README.md),
[final summary including exact ledgers/transactions/source hashes](experiments/stage1/host-resume-20261015/summary.json),
[manifest](experiments/stage1/manifest.json),
[tests](experiments/stage1/host-resume-20261015/tests-final.log).
Raw signed envelopes, RPC requests/responses, metadata XDR and decoded values,
Core archive bytes and file checksums are retained locally.

**Exit gate: PASS for this bounded Stage 1.** No public network, remote, push, PR,
release, outreach, other product edits, discovery changes or Stage 2 occurred.
Selection BUILD survives this narrow feasibility proof; broader practicality and
product demand are not established. Any later stage requires new authorization.

---

# Historical infrastructure-blocked findings (superseded by host continuation)

# Stage 1 flagship findings

> Resume 2026-10-15: the same Stage 1 was resumed with the corrected endpoint
> `http://localhost:8000/rpc`. Actual agent preflight still exited 2: Docker socket
> permission denied and loopback RPC Operation not permitted. CLI/Rust/target checks
> passed; Blend acquisition still failed DNS. See
> [resume findings](experiments/stage1/resume-preflight-20261015/result.json) and
> [actual preflight](experiments/stage1/resume-preflight-20261015/report.json).
> Earlier diagnostics below remain historical. No new campaign ran; Stage 1 is
> still INCONCLUSIVE. Implementation and its previous 15-test result are unchanged.


2026-10-15. **INCONCLUSIVE — stopped before Blend provisioning.**
Selection remains BUILD; this environment could not execute the required experiment.
No actual archival/restoration result, target compatibility or product maturity is claimed.

## Executed setup and earliest blocker

Created this isolated project and initialized local Git (`main`, unborn, no remote,
no commit). Preserved other product/research files. Updated the master status.

| Executed attempt | Actual result |
|---|---|
| Docker `info` | Exit 1: permission denied connecting to `/var/run/docker.sock`. |
| Rootless Podman with project-local XDG runtime/config, storage root/runroot and vfs | Exit 125: `newuidmap` cannot write `uid_map`: Operation not permitted; cannot set up namespace. Writable directory configuration did not resolve the execution restriction. |
| Native `stellar-core --version` | Executable unavailable; no Core found in inspected executable locations. |
| Local RPC `getHealth` at `127.0.0.1:8000/soroban/rpc` | URL open failed with Operation not permitted. This does not prove whether a server exists outside this execution environment. |
| Blend `git ls-remote` | Exit 128: could not resolve github.com. No exact revision acquired. |
| Blend source archive download | Temporary DNS resolution failure; no source archive or WASM acquired. |
| Core release acquisition | Temporary DNS resolution failure; no native binary acquired. |

Exact argv, stdout, stderr and exceptions are preserved in
[initial probes](experiments/stage1/preflight-probes.json) and
[final executable preflight](experiments/stage1/preflight-final/report.json).
No privilege escalation, namespace bypass, Docker socket permission change or public
network deployment was attempted. No mock runtime was substituted.

Version probes actually executed: Stellar CLI 28.1.0
(`c0f4d0da891bbf214c08b8c5035ae6db80e9a3bd`), XDR 28.0.0; rustc/Cargo 1.95.0;
Python 3.14.6; Git 2.55.0. These are available tools, not a chosen coherent Core/Host
execution profile. Core, Host, protocol, network settings and ledger anchors are unknown.

The public web retrieval path could display Blend contract source and Core release
metadata, but cannot supply an acquired/pinned executable artifact or a runnable backend.
Rendered source availability does not resolve these execution blockers. See
[source inspection](experiments/stage1/source-inspection.json).

## Chosen invariant — not yet executed

For an initialized pool with no intervening configuration update:

```text
canonical(get_config_after) == canonical(get_config_before)
get_admin_after == get_admin_before
```

The decoded raw contract-instance configuration/admin must agree with the corresponding
public result before archival and after restoration. Compare oracle, minimum collateral,
backstop rate, status and maximum positions with their actually observed baseline values;
do not invent an expected configuration or accept missing fields as equal.

Why: persisted configuration and authorized administrator are meaningful continuity
properties that do not require lending, price changes, interest accounting or a populated
reserve graph. The owner explicitly permits this smaller invariant. It is available via
[Blend's public pool source](https://github.com/blend-capital/blend-contracts-v2/blob/main/pool/src/contract.rs),
where the getter delegates to stored configuration. That mutable source is inspectable,
not an immutable used Blend revision. Initialization requirements must still be verified
against the actual acquired artifact; no assumption that arbitrary dependency addresses
are sufficient is implemented.

Public operation for the minimum first campaign: read the initialized configuration and
administrator before disruption and again after legitimate restoration. No application
values, instance keys or contract IDs have actually been obtained.

## Actual implementation and tests

Only `src/archival_regression/preflight.py` is implemented. It:

- captures actual process status/stdout/stderr and missing-executable/timeout errors;
- checks backend availability output rather than treating empty/malformed output as ready;
- probes a plain loopback RPC health URL with proxying/redirects refused;
- refuses existing output directories to preserve prior evidence;
- reports setup errors as INCONCLUSIVE and leaves every campaign phase NOT TESTED;
- never returns campaign PASS or exits zero merely because tools/health are available.

No campaign schema, generic deployment hook, invariant comparator, archive/XDR classifier,
restore executor, raw-state verifier or recovery controller was built after the first
runtime dependency failed. Building an unexecutable platform would not establish the
flagship. There is no Blend-specific product branch.

```sh
# From soroban-archival-regression/
PYTHONPATH=src python3 -W error::ResourceWarning -m unittest discover -s tests -v
python3 -m compileall -q src tests
PYTHONPATH=src python3 -m archival_regression.preflight \
  --stellar-cli /tmp/runbook-tools/stellar \
  --output docs/experiments/stage1/preflight-new
```

Final results: **15 behavior tests passed**, zero failures; Python compilation exit 0.
Tests cover actual subprocess success/failure, missing program, timeout classification,
malformed/missing fields, unhealthy/malformed RPC evidence, local-only URL enforcement,
redirect rejection, evidence overwrite prevention and setup retries never counting as
recovery proof. Synthetic health responses are used solely for preflight parsing tests;
they are not fabricated protocol evidence. Unit-test stdout includes a deliberately
empty mocked inventory and its INCONCLUSIVE report, not another real machine probe.

Final real preflight exit: **2, INCONCLUSIVE**, as intended when execution prerequisites
are unavailable. No assertion/invariant/archive classifier is claimed tested when it
was not implemented. `ruff` is not installed; lint is NOT TESTED. Blend/Core build is
NOT TESTED. Logs: [tests](experiments/stage1/tests-final.log),
[compilation](experiments/stage1/python-compile-final.log).

## Stage 1 outcome matrix

| Required report item | Actual finding |
|---|---|
| 1. Stage 1 | **INCONCLUSIVE**, not PASS. |
| 2. Exact Blend revision | **Unavailable**; git/source acquisition failed. No real Blend artifact used. |
| 3. Exact invariant | Configuration/admin equality and raw-instance agreement defined above; **NOT TESTED**, no values recorded. |
| 4. Archival mechanism exercised | **None**. Intended real Core persistent eviction to hot archive, not a ledger jump or deletion. |
| 5. Proof archival was real | **NOT TESTED**. No Core metadata/buckets or archive witness exists. |
| 6. Restoration mechanism | **NOT TESTED**. Legitimate RestoreFootprint/protocol restoration path must be selected after pinning actual protocol. |
| 7. Control | **NOT TESTED**; provisioning did not start. |
| 8. Positive flagship | **NOT TESTED**. |
| 9. Meaningful archival negative | **NOT TESTED**. Docker/Podman/source errors are not a product negative. |
| 10. Retry/interruption | **NOT TESTED**. Repeating preflight is not a recovery retry. |
| 11. Raw verification | **NOT TESTED**; only infrastructure diagnostics captured. |
| 12. Five fresh-state repetitions | **0/5 campaigns, 0 target builds**. Multiple preflight attempts do not count. |
| 13. Actual integration classification | **NOT TESTED / unclassified**. None of YES, CONFIGURATION ONLY, SMALL GENERAL PRODUCT GAP or LARGE PRODUCT GAP is supported by implementation evidence yet. |
| 14. Tests/build/lint | 15 preflight tests PASS; Python compile PASS; Blend/Core build and lint NOT TESTED. |
| 15. Main limitations | No runnable Core, permitted loopback RPC or acquired independent target; campaign machinery remains unimplemented. |
| 16. Original BUILD assumptions survived? | **Undetermined**. Infrastructure inability neither validates nor falsifies local archival practicality, small integration or product distinctness. |
| 17. Recommendation | **RESEARCH MORE**, narrowly to unblock execution prerequisites and resume this same Stage 1. No new discovery cycle or Stage 2. |

Prior selection assumption SMALL GENERAL PRODUCT GAP is retained as an assumption only.
Do not label an unimplemented integration YES or LARGE PRODUCT GAP because the current
execution environment denied access. All 15 exit requirements are recorded separately
in [manifest.json](experiments/stage1/manifest.json); only the introduced behavior-test
requirement has passed. Checksums establish retained bytes, not archival correctness.

## Continuation boundary

The first unmet dependency is a permitted runnable local Core backend/local RPC plus
provenance-verifiable Blend source/artifact access. After that, resume **Stage 1**:
pin an unchanged actual target, inspect initialization, provision and establish the
public invariant before implementing the archive/restore campaign.

No network was created and no Core test profile was applied. There are no non-default
network settings to report; that absence does not imply a successful default-profile
experiment. Future acceleration must preserve real eviction, record every setting,
and account for Blend's own TTL extensions. RPC absence alone must never count as archive
proof. Real negative/retry cases must respect transaction atomicity and automatic restore.
Do not claim that an assumed partial restoration has already been shown possible.

The selected project remains the active BUILD, with Stage 1 stopped at an environmental
blocker. No automatic continuation to MVP or Stage 2 is justified by these results.
