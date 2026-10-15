# Stage 1 evidence

Result: **INCONCLUSIVE**. This directory contains infrastructure attempts, not a
Blend archival campaign. See [the full findings](../../stage1-flagship.md).

- `manifest.json`: actual stage result, unknown revision/artifacts/network anchors,
  proposed invariant, commands, outcomes, build/test state and exit requirements.
- `preflight-probes.json`, `raw/*.json`: initial real runtime/RPC/source attempts.
- `preflight-run-1/`, `preflight-run-2/`, `preflight-final/`: versioned preflight
  reports; final source hashes are in the final report. Earlier reports retain
  their original source hashes. These are not fresh-state campaign repetitions.
- `source-inspection.json`: accessible rendered source versus failed revision API.
- `tests-final.log`, `python-compile-final.log`: final introduced-logic checks.
- `checksums.json`: integrity hashes of retained evidence (excluding itself).
- `verification.json`: local integrity/preservation check results.

No WASMs, contract IDs, ledger anchors, archive witnesses, restoration transactions,
application outputs, retry outcomes or five-run proof exist. Null/missing fields
are intentionally not replaced by synthetic values. No secrets were generated.
