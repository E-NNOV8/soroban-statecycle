# Local public-repository preparation — 2026-10-15

Existing Blend and Tansu scenarios were rerun on a new pinned Quickstart network
using `scripts/setup_local.py --port 18001 --container statecycle-repro-2 --output .local/repro-check-2`.
Both complete campaigns passed, including real eviction/restoration, expected
negative, retry and raw equality. See `summary.json` and `campaigns/` for original
commands, RPC/XDR, transaction hashes, ledger numbers and receipts. This repeats
existing coverage to validate bootstrap; it adds no new target/product feature.

The first setup attempt (`--port 18000 --container statecycle-repro --output .local/repro-check`)
stopped INCONCLUSIVE with HTTP 502 from Friendbot after RPC was healthy. Its sanitized
command log is retained. The setup now waits for the funding service. The second
attempt succeeded through genuine consensus configuration readback. Private key
files are excluded; key-generation output was omitted when captured. Signed public
transactions and public addresses remain reproducible evidence.

The optional CLI installer downloaded the official v28.1.0 Linux archive and checked
SHA256 `c1680deee94301d33ada7a17f98411e642a4248c727afbd2e43050d345746462`.
A public-only export was copied into the local Quickstart container and checked
under its Python 3.12.3 with this verified CLI: 57 tests, compilation, seven offline
campaign replays and checkout sanity passed. Host Python 3.14.6 also passed.
This local Linux-container check is **not hosted CI execution**.

The 1715 original Stage 1/2 historical file identities were checked against a private
preparation baseline; no bytes changed. Bytecode caches remain local and ignored.
The public integrity manifest separately covers all retained source/evidence files.
No GitHub repository, remote, push, release or hosted run was created.

A separate public-only checkout at `/tmp/statecycle-public-checkout` then ran
`python3 scripts/setup_local.py --port 18002 --container statecycle-fresh-checkout
--output .local/fresh`, followed by the documented Tansu campaign using its
generated scenario and `--output outputs/fresh-tansu`. Both passed without private
workspace dependencies. Full setup and campaign receipts are retained under
`fresh-public-checkout/`. This additionally checks that fixture paths are resolved
from the checkout rather than the original workspace location.
