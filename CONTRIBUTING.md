# Contributing to Soroban StateCycle

Start with [architecture](docs/architecture.md), [configuration](docs/scenarios.md)
and [supported scope](docs/limitations.md). This alpha already has a complete bounded
Core-backed lifecycle; passing tests alone cannot replace real archival proof.

From the checkout root with the documented CLI on PATH:

```sh
bash scripts/check.sh
python3 scripts/verify_evidence.py
python3 scripts/verify_hosted_evidence.py
python3 scripts/check_reproducibility.py
```

For executable validation/provenance/invariant changes, provision a new local node
and run both [existing campaigns](docs/reproducibility.md), or dispatch the existing
manual integration workflow as an authorized repository operator. Preserve exact
producer SHA, failures, RPC/XDR, receipts and artifact hashes. Keep original evidence
immutable; add separately attributed proof for later execution. Do not patch targets,
substitute missing data, weaken assertions or count infrastructure failure as FAIL.
Do not publish signing configuration or secrets. Retained upstream materials keep
their licenses; root MIT covers first-party material only.

## Legitimate future work (not promised or opened as issues)

- An additional protocol/XDR adapter is useful only with pinned real Core proof,
  version compatibility boundaries and rejection tests; a version-number edit is insufficient.
- A documented long-TTL integration profile could support real applications that
  extend their state beyond this short campaign window, without manual deletion or
  target patches. Rejected historical TTL probes are not a working profile.
- A recoverable ambiguous-submit/in-flight boundary could test failures beyond the
  already-proven committed-stage retry, with real transaction reconciliation and
  independently attributable receipts.

Each is independently useful additional scope, not an unfinished requirement of the
current bounded MVP. Prioritize an observed user need and bounded proof before
implementation. No manufactured backlog, new target, UI or broad integration is
required merely to appear mature. Documentation or maintenance fixes should identify
a concrete wrong claim, reproducibility failure or correctness defect.
