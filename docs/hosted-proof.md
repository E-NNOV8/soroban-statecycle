# Hosted proof and durable public evidence

**Historical hosted result: PASS**, produced by exact commit
[`c19751db48702fe075b24da749a65a611ba3d1b6`](https://github.com/mxrtins04/soroban-statecycle/commit/c19751db48702fe075b24da749a65a611ba3d1b6).
This is the original executable producer, not an assertion that every later HEAD
has run expensive integration. Subsequent verifier hardening may replay these bytes;
it does not change their producer or relabel local Stage 1/2 evidence as hosted.

| Proof | Original hosted run | Executed checks |
| --- | --- | --- |
| Fast | [38807089360](https://github.com/mxrtins04/soroban-statecycle/actions/runs/38807089360) | 57 tests, compilation, 2546 retained files, seven offline campaigns, checkout sanity: PASS. |
| Real Core | [38807146800](https://github.com/mxrtins04/soroban-statecycle/actions/runs/38807146800) | Fresh genuine Quickstart provisioning and both complete Blend/Tansu campaigns: PASS, with raw evidence upload. |

Runner: Ubuntu **24.04.5**, CPython **3.12.15**, runner **2.337.0**, image
`ubuntu-24.04` version **20261012.334.1**. Stellar CLI **28.1.0**, pinned Quickstart
Core **29.0.0**, active protocol **28**, minimum persistent TTL 64 / temporary 16 /
scan starting level 1. Raw runner/job logs and full environment records are retained.
Fast runs automatically for pushes/PRs; real Core integration is manual-only.

## What actually ran

| Scenario | Actual hosted observations |
| --- | --- |
| Blend config/admin | Real unchanged factory → pool `CAF62OKICOT4YEVBNZECEXKQHX7IYCYNENVKXTPKSDHM7L3UYOUOT5AD`; persistent instance Core eviction ledger **93**; restoration **96**, committed-stage retry **98**. Public config/admin and complete observed raw instance agree before/after. |
| Tansu project/commit | Contract `CDSLK6MGSKGSI2KDHZGFB5NGBBUFYXYTL7UHJYKTUPUSHLS3OW4RXB4D`; instance/project/commit evicted **168/174/175**; first restore batch **178/179/181/183**, retry **184/186/188/189**. Authenticated commit X → Y before archival; getter and raw LastHash remain Y after restoration. |

Tansu X: `7bd159a9660d5764fb288977ba97356b3f19c72d`.
Tansu Y: `0524376d293be41ee3bf5ca102cee2a88bea3d03`.

Both campaigns passed unchanged live control, genuine archival/positive recovery,
expected omitted-restore negative, separate-process retry after committed restoration,
and independent raw recovery verification. Runner exits are `[2,1,2,0,0]` for archive,
negative, restore checkpoint, recovery/retry and final verify. Successful empty-footprint
retry cannot replace the first actual restored contents. No in-flight atomic crash,
production-network behavior, exhaustive application coverage or automatic-restore
failure is claimed. The negative is explicit-restoration policy enforcement.

Original fixtures and inspected source identities are described in the unchanged
[Stage 1](stage1-flagship.md) and [Stage 2](stage2-reusability.md) reports. These
are **external-project compatibility proofs** against pinned unchanged application
artifacts using the same engine. They establish no production adoption, maintainer
endorsement, collaboration or complete application audit. Blend's tested getters do
not call its placeholder oracle/backstop contracts; Tansu uses a real SAC dependency.

## Retained artifact and raw evidence

Original [statecycle-core-evidence artifact, ID 12006608955](https://github.com/mxrtins04/soroban-statecycle/actions/runs/38807146800/artifacts/12006608955):
670915 bytes, GitHub SHA-256 digest:

```text
c17151b51a2de9f1c31304add986bd3fa243962bba8926d4b652e831124eebc5
```

The [retained diagnostic-redacted ZIP](hosted-proof/20261015/core-artifact.sanitized.zip),
[bundle manifest](hosted-proof/20261015/manifest.json),
[Core run metadata](hosted-proof/20261015/core-run.json),
[Core job results](hosted-proof/20261015/core-jobs.json),
[artifact metadata/digest](hosted-proof/20261015/core-artifacts.json),
[fast run metadata](hosted-proof/20261015/fast-run.json),
[fast job results](hosted-proof/20261015/fast-jobs.json),
[Core logs](hosted-proof/20261015/core-logs.zip),
[fast logs](hosted-proof/20261015/fast-logs.zip) and
[runner environment](hosted-proof/20261015/runner-environments.json) are now in Git.
Logs/API metadata were copied unchanged. The artifact copy omits only Quickstart's
standalone network root-key text from `setup/docker.log`; it is **not byte-identical
to the original ZIP**. The manifest records the original GitHub digest, the retained
ZIP digest, both diagnostic-member hashes and original hashes for every member.
All application RPC/XDR/checkpoint/receipt members remain byte-identical. This
explicit diagnostic redaction is not a rewritten application result. The downloaded
original remains unchanged locally; signing configuration is not included. Future
integration uploads redact this diagnostic line before publication.

Inside the artifact, `blend/` and `tansu/` contain configuration, checkpoints,
runner exits and numbered raw RPC/CLI records. Checkpoints reference exact ledger
hashes, transaction hashes, v2 eviction witnesses, v4 restored metadata and raw
before/after receipts. `setup/` contains original consensus-upgrade/readback,
artifact upload verification and Docker diagnostics. The retained hosted bytes remain zipped;
verification works in a temporary extraction and leaves them unchanged.

GitHub originally reports expiry **2027-01-13T21:05:17Z** for this artifact; workflow
logs also have finite service retention. The checked-in copies avoid a dependency
on those future downloads or private Codex notes. Hashes/provenance are relative to
the retained baseline and GitHub metadata, not a signed external attestation.

## Independently verify from a checkout

With the documented Python and Stellar CLI on PATH, from the checkout root:

```sh
python3 scripts/verify_hosted_evidence.py
```

No Docker, network, original workspace, private signer or old runner filesystem is
needed. The verifier checks the complete retained file set and hashes, consistent
producer/run/job/artifact identities and outcomes, safe ZIP extraction, full recorded
lifecycle sequence, expected negative, declared mutable X/Y, and the current engine's
raw recovery checks for each declared campaign. Missing or inconsistent evidence
exits 2 INCONCLUSIVE, not PASS. This offline command does not execute new live
control/negative/retry transactions. Those were executed by the attributed hosted run.

The separate `python3 scripts/verify_evidence.py` checks original local Stage 1/2
and preparation proof, preserving its 2546-file/seven-replay scope. Hosted proof is
not silently added to that historical corpus. See [architecture](architecture.md)
and [limitations](limitations.md) for exact trust and support boundaries.

## Hardened producer validation

The validation/classification/invariant hardening subsequently passed on exact
producer [`615522022c7a8fdcc957b111fb13073cba47094d`](https://github.com/mxrtins04/soroban-statecycle/commit/615522022c7a8fdcc957b111fb13073cba47094d):

- [Fast run 38813239421](https://github.com/mxrtins04/soroban-statecycle/actions/runs/38813239421): **68 tests**, compilation, original local
  proof, original hosted proof and checkout sanity PASS.
- [Real Core run 38813244977](https://github.com/mxrtins04/soroban-statecycle/actions/runs/38813244977): genuine fresh Blend and mutable Tansu
  control/archival/restore/negative/retry/raw verification PASS. Both downloaded raw
  campaigns also pass independent offline recovery replay.
- [Artifact 12009467151](https://github.com/mxrtins04/soroban-statecycle/actions/runs/38813244977/artifacts/12009467151):
  SHA-256 `69c28d4dba6e029539731aa8862e28d9d9e43f3cb514e1a6d8fed0d6bc51efc3`; downloaded ZIP matches GitHub's digest.

The [new bundle manifest](hosted-proof/hardening-20261015/manifest.json),
[original uploaded ZIP](hosted-proof/hardening-20261015/core-artifact.zip),
[Core run metadata](hosted-proof/hardening-20261015/core-run.json),
[Core job results](hosted-proof/hardening-20261015/core-jobs.json),
[artifact metadata](hosted-proof/hardening-20261015/core-artifacts.json),
[Core logs](hosted-proof/hardening-20261015/core-logs.zip),
[fast run metadata](hosted-proof/hardening-20261015/fast-run.json),
[fast job results](hosted-proof/hardening-20261015/fast-jobs.json),
[fast logs](hosted-proof/hardening-20261015/fast-logs.zip) and
[runner environment](hosted-proof/hardening-20261015/runner-environments.json) are
retained in Git. The producing workflow already omitted the standalone root-key
diagnostic, so this newer ZIP is **byte-identical** to the hosted upload; the manifest
records an identity mapping for that diagnostic, with no post-download edit.
Original c19751d evidence above remains separately attributed and unchanged.

Exact new contract IDs, eviction ledgers and first/retry restore ledgers:

```json
{
  "blend": {
    "contract_id": "CBP7OZLS4H6PTB4TWW7LZJP3FKEPCNCPQF4HVLTP2VMJV2WI3K3KWSZE",
    "evictions": {
      "instance": 94
    },
    "restore_ledgers": [
      [
        96
      ],
      [
        98
      ]
    ]
  },
  "tansu": {
    "contract_id": "CCUURCUGK3TS4VU7GANYYKKOUPRCAC6K4SXFCFXYNGJI5SKMA7XZ7QGI",
    "evictions": {
      "instance": 167,
      "project": 173,
      "commit": 175
    },
    "restore_ledgers": [
      [
        178,
        180,
        181,
        182
      ],
      [
        184,
        186,
        187,
        189
      ]
    ]
  }
}
```

Verify the newer bundle independently, without Docker, network or signing keys:

```sh
python3 scripts/verify_hosted_evidence.py --bundle docs/hosted-proof/hardening-20261015
```

The default command continues to verify the original c19751d bundle. Neither
historical producer is silently rewritten to a later documentation-only HEAD.
The follow-up evidence/documentation commit changes no executable inputs relative
to this tested hardened producer; fast CI still verifies its public checkout.
