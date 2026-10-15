# Skeptical MVP hardening audit

Audited starting revision: `c19751db48702fe075b24da749a65a611ba3d1b6`.
Scope: current source/tests, bootstrap, workflow definitions, public docs, original
Stage 1/2 manifests and downloaded original hosted proof. This is a bounded sanity
review, not an independent security audit or adoption assessment.

| Finding | Evidence and substantive response |
| --- | --- |
| High public-evidence gap | README/reproduction guide still denied hosted execution. Hosted ZIP/log/provenance lived only in ignored local storage and an expiring GitHub artifact. Retain hosted application proof and provenance in `docs/hosted-proof/20261015`, publish exact SHA/run/digest attribution, and add offline bundle verification to fast CI. |
| Medium outcome-classification bug | A v2 key with `contract_data: null` reached `.get` in scenario validation and the public runner exited 1 with a traceback before hooks. Validate nested field shapes explicitly; behavior test now requires JSON INCONCLUSIVE/2 and no provisioning. |
| Medium invariant-comparison bug | Raw `u32: 1` was accepted against public JSON `true` because Python compares `1 == True`. Make public/raw and declared invariant comparisons recursively type-sensitive; regression test rejects scalar and nested aliases. |
| Medium interface overclaim | Docs allowed command lists for deploy/public hooks, but `Campaign.hook` executes one argv and requires one transaction. Document and validate the actual restriction; lists remain supported for setup/mutate/restore. |
| Medium diagnostic disclosure | Artifact scan found Quickstart standalone root-key text in Docker logs. Preserve original download; redact only that line in the public diagnostic copy, retain original/member/retained hashes, and redact the same line before future CI uploads. Application RPC/XDR bytes remain unchanged. |
| Documentation gap | No current architecture/trust-boundary map or honest contributor surface. Describe actual Core/RPC/XDR flow, recovery-only versus full campaign verification, checkpoint retry, supported shapes and independently useful optional future work. |

Existing campaign logic already requires exact all-key Core eviction, actual restored
contents and independent raw live receipts. Target-specific data remain in fixtures
and bootstrap; engine contains no target-name branch. Shell argv is trusted executable
input; it is not a sandbox. General hooks can leak secrets if authored unsafely; the
bootstrap omits generated key output and CI excludes actual signing config. Checkpoint
basenames/config-byte checks and retained hashes protect the intended local workflow,
not an attacker-controlled multi-user filesystem or coordinated evidence forgery.
These boundaries are disclosed rather than disguised as security guarantees.

No new target, target patch, frontend, product breadth, fake backlog or source-history
rewrite was needed. Original Stage 1/2 reports/manifests/raw files are untouched.
Source metadata remains descriptive; bootstrap verifies bundled artifact hashes.
Other protocol/network shapes, long-TTL campaigns and in-flight retry remain
unsupported additional scope, not unfinished mandatory work for the documented MVP.

Validation required for this change: all fast checks and boundary regressions,
original local evidence replay, original hosted bundle replay, fresh-checkout checks,
then real Core integration on the new executable producer. Evidence parsing and
invariant validation changed, so documentation-only reuse of old hosted proof is
insufficient to certify the new SHA. New run results must remain separately attributed.
The original hosted producer stays c19751d; future hardening CI does not rewrite it.

## Executed validation and file responsibilities

All local fast checks pass (68 tests, compilation, shell/YAML sanity, links, public
archive secret scan and public-only checkout). Original 2546 retained files and
seven offline runs still verify; all 1715 original historical file identities are
unchanged. Original hosted raw evidence verifies under the hardened engine.
Hardened producer `615522022c7a8fdcc957b111fb13073cba47094d` passed hosted fast run 38813239421 and genuine Core run
38813244977; both hosted raw campaigns independently replay successfully. The
uploaded Docker diagnostic contains the omission marker and no Stellar secret key.
No checks/assertions/archival semantics were weakened, and no target source changed.

The execution commit changes six code/CI/test files: `campaign.py` (type-sensitive
invariants), `scenario.py` (input classification and actual hook constraints),
`verify_hosted_evidence.py` (safe retained proof verification), `test_hardening.py`
(boundary/integrity regressions), `engine-tests.yml` (public proof fast gate), and
`local-integration.yml` (redact diagnostic before upload). Active documentation is
README, CONTRIBUTING, architecture, evidence, hosted-proof, limitations, this audit,
reproducibility and scenario reference; each corrects a documented gap above.

Each of the two public proof directories contains ten files: artifact ZIP (raw
application proof), artifact metadata (GitHub ID/digest), Core run/job metadata
(producer and execution outcomes), Core log ZIP (execution trace), fast run/job
metadata (producer and fast outcomes), fast log ZIP (actual test/verifier output),
runner environment JSON (exact OS/image/Python), and manifest (hashes, attribution
and diagnostic mapping). The original ZIP has a disclosed diagnostic-only retention
redaction; the newer uploaded ZIP is retained byte-identically. The second commit
adds only captured evidence and documentation, without executable changes. It does
not require another expensive Core run or claim that the doc HEAD produced earlier
measurements. No mandatory blocker remains for the explicitly bounded alpha MVP;
adoption/endorsement and unsupported profiles remain unproven.
