# Reproducing the local profile

## Versions

| Dependency | Profile |
| --- | --- |
| Python | Standard library only; Python 3.12 in hosted workflows. Public-only checkout checks also run locally under 3.12.3; host preparation uses 3.14.6. |
| Stellar CLI | 28.1.0, XDR 28.0.0; CLI version is enforced by setup. |
| Docker | Working Linux-container Docker Engine, socket access, network access for the first image download. No Docker API version-specific features beyond run/exec/ps. |
| Quickstart | `stellar/quickstart@sha256:4f8dfd693deba57fddef7f0bc722e4782c2e606c8675e3adc5438cebae249295` (Linux profile). |
| Core / protocol | Image contains Core 29.0.0; **active protocol 28**, verified from RPC. |
| Archival settings | Minimum persistent TTL 64, temporary TTL 16, starting eviction scan level 1; maximum TTL remains the image default 3110400. |
| Application artifacts | Retained hash-checked WASMs in `docs/experiments/stage2/artifacts/`; exact hashes enforced by setup. |

Use the [official Stellar CLI installation instructions](https://developers.stellar.org/docs/tools/cli/install-cli)
for other platforms. `bash scripts/install_stellar.sh .local/bin` installs the
checksum-pinned official Linux x86_64 release; add that directory to PATH. Install
Docker Engine using its platform instructions and verify `docker info` succeeds.
No `pip install` is required. Rust is unnecessary for running the retained artifacts;
rebuilding them requires the separate upstream locked build inputs documented in
the historical records and retained sources.

## Fresh checkout

Clone the public repository, install Python/Docker and the pinned CLI described
above, then run from the repository root (including when using `PYTHONPATH=src`):

```sh
git clone https://github.com/E-NNOV8/soroban-statecycle.git
cd soroban-statecycle
bash scripts/install_stellar.sh .local/bin
```

```sh
export PATH="$PWD/.local/bin:$PATH"  # if using the optional installer
python3 scripts/check_reproducibility.py
python3 scripts/setup_local.py
PYTHONPATH=src python3 -m archival_regression run \
  .local/statecycle/scenarios/tansu-project-commit.json --output outputs/tansu-1
PYTHONPATH=src python3 -m archival_regression run \
  .local/statecycle/scenarios/blend-config-admin.json --output outputs/blend-1
```

The setup starts a **new** local Quickstart container and waits for RPC and funding
services. It creates local signing identities, funds the admin through Friendbot,
uploads unchanged retained WASMs, deploys Blend's real factory and a protocol SAC
for Tansu, and submits an actual consensus archival-configuration upgrade. It reads
back the active configuration before reporting setup success. Setup success alone
makes no archival claim. Each subsequent campaign deploys a fresh target.

To coexist with another node:

```sh
python3 scripts/setup_local.py --port 18000 --container statecycle-secondary \
  --output .local/secondary
PYTHONPATH=src python3 -m archival_regression run \
  .local/secondary/scenarios/tansu-project-commit.json --output outputs/tansu-2
```

Use a new setup directory/container and a free port. Existing containers and output
directories are refused, never reset. Setup leaves failed attempts for inspection.
Inspect `setup-result.json` and `setup-evidence.json` in its output. Private key
material is stored separately under its `stellar/` subdirectory and omitted from
command output logs. Do not publish that directory. The standard `.local/` and
`outputs/` locations are ignored. If choosing custom locations, protect them too.

Campaigns wait for natural sequential ledgers and actual Core eviction; there is
no manual entry deletion, ledger jump, source patch or snapshot manipulation.
They normally take minutes. Timeouts and service failures are INCONCLUSIVE. The
node persists until explicitly stopped by its operator. This script is development
infrastructure and must not be pointed at public networks.

## Checks and automation

```sh
bash scripts/check.sh
python3 scripts/verify_evidence.py
python3 scripts/check_reproducibility.py
python3 scripts/verify_hosted_evidence.py
```

The first runs all behavior tests and compilation. Some tests need Stellar CLI;
CI installs it explicitly, and the evidence verifier refuses to skip XDR replay.
The lightweight reproducibility check validates active scenarios, fixture hashes,
version, public documentation requirements and workflow structure without Docker.
The retained verifier makes no new live-network claim.

Fast hosted CI runs these checks on Python 3.12 for pushes and pull requests,
including offline replay of the original hosted artifact. The separate manually
triggered integration workflow provisions the pinned network and runs both genuine
campaigns, uploading diagnostics even on failure. Both workflows actually passed
on producer `c19751db48702fe075b24da749a65a611ba3d1b6`; see
[hosted proof and retained raw evidence](hosted-proof.md). That proof does not imply
Core integration runs on every push or that a later SHA has been tested live.

This Linux-container profile is the supported environment. The installer uses
Linux x86_64 GNU binaries and requires `bash`, `curl`, `tar`, `sha256sum`, `install`
and `uname` (standard GNU/Linux utilities). macOS/Windows/ARM fresh setup is NOT
TESTED. No original workspace checkout, global signing alias, pre-funded identity
or private Codex note is needed: setup resolves retained WASMs from this checkout
and generates its own ignored CLI configuration. The standalone campaign command
assumes the documented CLI version; setup enforces 28.1.0 explicitly.

To rerun hosted integration as a repository operator with GitHub CLI permission:

```sh
gh workflow run local-integration.yml --repo E-NNOV8/soroban-statecycle --ref main
```

This dispatch is a real integration job, not an offline verifier. Retain its exact
`head_sha`, run/job metadata and artifacts before attributing new hosted proof.

## Independent reproduction handoff

The hosted campaigns below were executed by the project. Their retention and
offline replay are useful evidence, but neither is an independent reproduction
or review. The pinned public review targets are executable proof producer
[`615522022c7a8fdcc957b111fb13073cba47094d`](https://github.com/E-NNOV8/soroban-statecycle/commit/615522022c7a8fdcc957b111fb13073cba47094d);
the `main` documentation baseline is
[`9e54a67c896a6550e1b9e082efc004943a4a8228`](https://github.com/E-NNOV8/soroban-statecycle/commit/9e54a67c896a6550e1b9e082efc004943a4a8228).
The project-controlled Core run was
[38813244977](https://github.com/E-NNOV8/soroban-statecycle/actions/runs/38813244977),
with artifact [12009467151](https://github.com/E-NNOV8/soroban-statecycle/actions/runs/38813244977/artifacts/12009467151),
retained ZIP SHA-256
`69c28d4dba6e029539731aa8862e28d9d9e43f3cb514e1a6d8fed0d6bc51efc3`.
The separate `security/pin-actions` branch contains action-pinning changes and
is outside these pinned review targets; it does not change the producer or
`main` documentation baseline above.

### Checklist

Start with a clean clone at the executable producer above:

```sh
git clone https://github.com/E-NNOV8/soroban-statecycle.git
cd soroban-statecycle
git checkout 615522022c7a8fdcc957b111fb13073cba47094d
bash scripts/install_stellar.sh .local/bin
export PATH="$PWD/.local/bin:$PATH"
```

Record the actual environment; the demonstrated profile is Python 3.12+,
Stellar CLI 28.1.0 / XDR 28.0.0,
Docker Engine, `stellar/quickstart@sha256:4f8dfd693deba57fddef7f0bc722e4782c2e606c8675e3adc5438cebae249295`,
Core 29.0.0, and active protocol 28. No Python packages or Rust toolchain are
required for these checks and campaigns.

1. Record `git rev-parse HEAD` and confirm it is
   `615522022c7a8fdcc957b111fb13073cba47094d`. Run `python3 --version`,
   `stellar --version`, `docker --version`, and `docker info`; note that the
   configured container image digest and active protocol are recorded by setup.
2. Run the behavior/compilation gate and retained offline checks from the checkout
   root:

   ```sh
   bash scripts/check.sh
   python3 scripts/verify_evidence.py
   python3 scripts/verify_hosted_evidence.py \
     --bundle docs/hosted-proof/hardening-20261015
   python3 scripts/check_reproducibility.py
   ```

   The hosted-bundle command validates retained project-produced bytes and replays
   them offline. It does not run Core or count as independent live reproduction.
3. Provision a fresh local Core network, then run both campaigns into new output
   directories:

   ```sh
   python3 scripts/setup_local.py
   PYTHONPATH=src python3 -m archival_regression run \
     .local/statecycle/scenarios/blend-config-admin.json \
     --output outputs/blend-independent
   PYTHONPATH=src python3 -m archival_regression run \
     .local/statecycle/scenarios/tansu-project-commit.json \
     --output outputs/tansu-independent
   ```

   Setup may take several minutes. It must report the pinned image and protocol-28
   network, verify fixture WASM hashes and read back the accelerated archival
   settings. Do not reuse an existing container or output directory.
4. For each campaign, inspect `campaign-result.json`, `runner.json`,
   `checkpoint.json`, the resolved `*.scenario.json` sidecar, and every referenced
   `evidence-NNNNN.json`. The enclosing campaign command must exit 0 and report
   PASS. Expected runner phase exits are `[2, 1, 2, 0, 0]`: archive checkpoint,
   deliberate missing-restore negative, restore checkpoint, new-process retry,
   final verification. The negative must exit 1 with reason
   `required explicit restoration omitted`; this is the expected explicit-restore
   policy check, not an infrastructure failure or claim that protocol auto-restore
   is broken. The separate retry must occur after a committed restore checkpoint.
5. Confirm Blend's public `get_config` and `get_admin` values and declared raw
   instance state match before and after recovery. Confirm Tansu changes the public
   commit from X `7bd159a9660d5764fb288977ba97356b3f19c72d` to Y
   `0524376d293be41ee3bf5ca102cee2a88bea3d03`, that the instance/project/commit
   keys have actual Core eviction witnesses, and that RestoreFootprint metadata
   contains the prior LedgerEntry contents. After restoration, the getter and raw
   `LastHash` remain Y and the declared raw entries match the pre-archive state.
   Contract IDs, transaction hashes and ledger numbers vary between executions;
   compare behavior and recorded evidence, not those run-specific identifiers.
6. Preserve both campaign output directories and the two resolved scenario
   sidecars. Record setup's `setup-result.json` and sanitized `setup-evidence.json`,
   but never include `.local/statecycle/stellar/` or other signing configuration.
   Create a checksum list outside the directories being hashed:

   ```sh
   (
     cd outputs
     find blend-independent tansu-independent -type f -print0 | sort -z | \
       xargs -0 sha256sum
     sha256sum blend-independent.scenario.json tansu-independent.scenario.json
   ) > outputs/independent-review.SHA256SUMS
   ```

   Keep the raw files and their hashes together; checksums alone do not prove that
   the experiment was correct.

### Reviewer results record

Complete this record after the reproduction. Do not call a result independent
unless the reviewer ran or reviewed it independently; record attribution only
with their consent.

```markdown
## Independent StateCycle reproduction / review

- Reviewer name or requested attribution:
- Project / role and relevant experience:
- Attribution consent: yes / no / not requested
- Review type: source review / clean-checkout reproduction / both
- Repository URL:
- Exact source SHA (`git rev-parse HEAD`):
- Documentation revision reviewed:
- Date and host OS/architecture:
- Python version:
- Stellar CLI and XDR versions:
- Docker Engine version:
- Quickstart image digest:
- Stellar Core version and active protocol:

### Commands and exits

| Command | Exit code | Result / notes |
|---|---:|---|
| `bash scripts/check.sh` | | |
| `python3 scripts/verify_evidence.py` | | |
| `python3 scripts/verify_hosted_evidence.py --bundle docs/hosted-proof/hardening-20261015` | | |
| `python3 scripts/check_reproducibility.py` | | |
| `python3 scripts/setup_local.py` | | |
| Blend campaign | | |
| Tansu campaign | | |

### Campaign observations and evidence

- Expected negative: phase exit/reason and enclosing campaign result:
- Restore checkpoint and separate-process retry observed:
- Blend config/admin and raw-state result:
- Tansu X→Y, eviction witnesses, restored contents and raw-state result:
- Evidence directories and SHA256SUMS path:
- Any missing/corrupt raw response, receipt, or checksum:

### Review conclusion

- Claims independently confirmed:
- Discrepancies with source paths and reproduction steps:
- Limitations or untested behavior:
- Conclusion: confirmed / confirmed with discrepancies / inconclusive
- Adoption, integration, endorsement, production readiness, or program acceptance:
  not assessed by this review unless separately evidenced.
```
