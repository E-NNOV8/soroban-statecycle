# Host Stage 1 evidence

Execution uses this local Codex CLI host, Docker `stellar-local` and loopback RPC
`http://localhost:8000/rpc`. No public network was used. Earlier sandbox failures
remain historical; commands needing host services ran with local process approval.

`baseline.json` identifies the deployed unchanged Blend factory and original pool,
configuration/admin values and pinned artifact hashes. `*-tx-*.json` retains raw
RPC receipts. `baseline-entry-*.json` and `*-metadata-*.json` contain CLI-decoded XDR.

`archival-config-decoded.json` is the original on-ledger network configuration.
`short-ttl-upgrade.json`, its encoded XDR, transaction receipts, and
`active-config-decoded-*.json` prove the real consensus configuration change.
Changes: min persistent TTL 120960 -> 64; min temporary TTL 720 -> 16;
starting eviction scan level 7 -> 1. Other archival settings remain unchanged.
Quickstart already accelerates real ledger closure to approximately one second.
Existing TTLs were not reduced. Blend source and binaries were not patched.

The installed Core helper emitted restore/upload/deploy envelopes but omitted the
payload invocation despite its stderr labels. The missing upgrade entry and two
rejected schedules are retained. `finish_upgrade.py` explicitly invoked the real
upgrade contract's `write` method and successfully scheduled the resulting key.
The applied setting was read back from ledger state at ledger 4457. Encoding and
unsupported direct-TTL-query attempts, and read-only `dump-ledger` lock/path
failures, are also retained. No offline dump or snapshot was edited to fix them.

`pilot-archived.json`, `pilot-ledger-4582.json`, and the raw `pilot-ledgers-*.json`
prove eviction of the exact persistent instance. `core-history-archived.json`
anchors a direct copy of an immutable hot-archive bucket; its SHA256 equals its
Core bucket-list hash. `archive-bucket-decode.json` has an actual `archived`
record whose full contents equal the previously live instance.
`hot-archive-verification.json` records this comparison. `pilot-restore-*` has the
RestoreFootprint receipt and `restored` metadata, and `pilot-recovery-result.json`
records complete raw-instance and public-value equality after recovery.

`run-N-config.json` supplies ordinary CLI hooks and instance symbol bindings to
`archival_regression.campaign`. Each `run-N/` preserves the exact loaded config,
raw command/RPC/XDR evidence and checkpoint. Each fresh pool has its own salt,
contract ID, baseline, eviction and restoration. Shared factory, code and network
are intentional. These are five fresh pool states, not five reset networks.

For each run: archive phase performs before/unchanged control calls and validates
raw agreement, then observes contiguous Core ledger metadata for exact eviction.
It exits 2 at the archived checkpoint because recovery is incomplete. Verify
before restore exits 1 for `required explicit restoration omitted`. Restore exits
2 after a committed RestoreFootprint checkpoint, before application recovery.
A new process retries restoration, observes public/raw equality, and exits 0.
The already-live retry may have an empty footprint; it requires a prior actual
restore and a fresh raw-state equality check. It is not an assumed partial restore
or an in-flight transaction crash. Automatic invocation restoration cannot hide
the negative: the negative deliberately omits the entire explicit restore phase
and does not invoke a getter that could restore automatically. It checks the
campaign lifecycle policy; it does not assert that Blend or protocol auto-restore
is broken.

The first retry exposed the verifier's refusal of Stellar's valid empty-footprint
retry. Its INCONCLUSIVE report remains; the corrected continuation passes. Source
hashes in the final summary identify the final verifier; all retained successful
campaigns are reverified with it. No historical error was relabeled as product FAIL.

`unavailable-rpc-host/` is a real host connection-refused probe, classified
INCONCLUSIVE exit 2 before setup. Unit tests are verifier tests, not substitutes
for the live campaigns. `tests-final.log` retains their results.

Reproduction requires the recorded local network/artifact prerequisites and an
account identity named `archival-admin`. No secret key is copied into evidence.
Choose a new output folder and fresh deployment salt; existing evidence is refused.
The `archive`, `restore`, and `recover` phase commands use the same config/output:

```sh
PYTHONPATH=src python3 -m archival_regression.campaign \
  --config path/to/fresh-config.json --output path/to/fresh-output --phase archive
# Expected 2 at the independently proven archived checkpoint.
PYTHONPATH=src python3 -m archival_regression.campaign \
  --config path/to/fresh-config.json --output path/to/fresh-output --phase verify
# Expected 1: required explicit restoration omitted.
PYTHONPATH=src python3 -m archival_regression.campaign \
  --config path/to/fresh-config.json --output path/to/fresh-output --phase restore
# Expected 2 at the committed restore checkpoint.
PYTHONPATH=src python3 -m archival_regression.campaign \
  --config path/to/fresh-config.json --output path/to/fresh-output --phase recover
# Expected 0 only after retry, public invariant and independent raw comparison.
```

The local test profile remains applied at handoff. Restoration does not undo the
consensus settings upgrade. Default-network archival latency, lending positions,
real oracle/backstop calls, full Blend correctness and production safety are outside
this configuration/admin continuity proof. Stage 2 remains unauthorized.
