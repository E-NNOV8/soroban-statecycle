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
