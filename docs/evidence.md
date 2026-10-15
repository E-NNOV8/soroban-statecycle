# Retained evidence integrity

```sh
python3 scripts/verify_evidence.py
```

This command checks the complete immutable evidence file set and SHA-256 values in
`docs/evidence-integrity.json`, parses retained JSON, and checks checkpoint receipt
references. It then copies evidence into temporary directories and applies the
current engine's offline verifier to both Stage 2 campaigns and all five Stage 1
runs. Originals are never written. Missing files, modified bytes, missing CLI or
invalid receipts stop verification with exit 2 (INCONCLUSIVE).

`--hashes-only` performs integrity verification without CLI-based semantic replay;
it must not be described as the full evidence verification.

Stage 1 and Stage 2 reports and raw evidence retain their original names, absolute
paths, timestamps, commands and results. Historical `soroban-archival-regression`
paths identify the original execution directory, not a fresh-checkout requirement.
Historical source hashes identify the version used then. Stage 1 engine source is
in `docs/experiments/stage2/engine-before/`; the final Stage 2 source/tests/templates
and README are retained in `docs/history/stage2-source/`. The final Stage 2 workflow is also snapshotted. Their original SHA manifests
also contain then-current active source paths; they are not a checksum specification
for today's README or source tree. Raw evidence and source references are checked
against preserved files/snapshots; the former root README checksum is historical
metadata (the Stage 1 root README version was not retained), not executable proof. Do not regenerate those historical manifests to
make current files appear historical.

The current integrity manifest covers the reports, both raw corpora, source
snapshots, corresponding Blend source and third-party license files. Python caches
are excluded. It is a repository integrity baseline, not a signed attestation or
protection against a party changing both evidence and manifest. Full replay validates
Core eviction, RestoreFootprint receipts and raw/public invariants rather than merely
trusting saved PASS labels. Live repeatability is a separate setup/campaign check.

## Hosted proof is a separate retained corpus

[Hosted proof](hosted-proof.md) identifies the exact producer SHA, run/job metadata,
original archive and GitHub digest. `python3 scripts/verify_hosted_evidence.py`
checks that separate manifest, validates producer/run/artifact consistency, safely
extracts into a disposable directory, checks complete lifecycle records and applies
the current recovery verifier to both raw campaigns. Missing/corrupted inputs cannot
pass. This command needs Stellar CLI but neither Docker, network access nor keys.
The original `verify_evidence.py` retains its 2546-file/seven-replay scope; it does
not silently relabel the local Stage 1/2 corpus as hosted evidence.

The zipped hosted originals are never extracted over repository evidence. The
hosted manifest and GitHub digest establish byte identity and attribution relative
to recorded metadata; they are not a signed attestation against an adversary able
to replace both the bundle and its manifest. RPC responses come from the trusted
local test node, not a verified public-network consensus client. Archived proofs
remain valid historical observations even when their source SHA differs from HEAD;
they do not certify later executable changes without new integration.
