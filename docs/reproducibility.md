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

From the repository root:

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
```

The first runs all behavior tests and compilation. Some tests need Stellar CLI;
CI installs it explicitly, and the evidence verifier refuses to skip XDR replay.
The lightweight reproducibility check validates active scenarios, fixture hashes,
version, public documentation requirements and workflow structure without Docker.
The retained verifier makes no new live-network claim.

Fast hosted CI runs all these checks on Python 3.12. The separate manually triggered
integration workflow provisions the pinned network and runs both genuine campaigns,
uploading diagnostics even on failure. It has not been executed on a hosted runner.
Neither workflow substitutes mocked archival. First publication and hosted execution
require owner authorization; neither has occurred during local preparation.
