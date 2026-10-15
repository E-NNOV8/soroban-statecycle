# Campaign configuration

Run from the checkout root. A scenario is trusted JSON containing **argv arrays**,
not shell snippets. Hooks run with the caller's permissions and working directory.
Review external configurations before execution. Setup writes ready-to-run copies
of the two fixtures under `.local/statecycle/scenarios/`.

## Required fields (scenario-v2)

| Field | Meaning |
| --- | --- |
| `schema` | Exactly `archival-regression/scenario-v2`; this stable internal identifier predates the public name. |
| `rpc_url` | Local HTTP RPC endpoint, for example `http://localhost:8000/rpc`. |
| `passphrase` | Exact local network passphrase. |
| `protocol` | `28`, the only verified profile. |
| `entries` | Nonempty map of observation names to complete persistent ContractData LedgerKey JSON objects. |
| `bindings` | Nonempty map from public observation hook names to raw value projections. |
| `hooks` | Deployment, restoration and public observation commands. |

Each `entries` value has exactly `contract_data`, containing `contract`, `key` and
`durability: "persistent"`. Use Stellar XDR JSON for the key, including
`"ledger_key_contract_instance"` for instance storage. The engine encodes each key,
requires an independent eviction witness for each, and verifies restoration coverage.

Each binding specifies `entry` (one declared entry name), `path` (nonempty JSON path
to its SCVal in decoded LedgerEntryData), and optionally `value_path` (path within
the normalized SCVal). Paths contain object keys or array indices. Example:

```json
{
  "get_admin": {
    "entry": "instance",
    "path": ["contract_data", "val"],
    "value_path": ["storage", "Admin"]
  }
}
```

## Hooks and placeholders

| Hook | Contract |
| --- | --- |
| `deploy` | Required. Deploy a fresh contract; final command stdout must be its JSON string or plain contract ID. Use `{salt}` for a fresh 32-byte salt. |
| `setup` | Optional provisioning commands after deployment. |
| `mutate` | Optional legitimate application mutation, after initial observation. |
| Public binding name | Required for each binding. Return JSON representing the invariant value. |
| `restore` | Required. Real restoration command(s) covering the declared keys and required code dependencies. |

`deploy` and each public observation hook must be **one argv array**. Optional
`setup`/`mutate` and required `restore` accept one argv or a list executed sequentially.
Public observation names cannot be reserved phase names (`deploy`, `setup`, `mutate`,
`restore`, `advance`). Deployment/getters must each produce exactly one transaction
hash in CLI stderr and a successful receipt; use `--send yes` for getters, as the
fixtures do. A getter returning cached/simulated stdout alone is insufficient. `{id}` expands to the deployed contract
ID; `{salt}` expands before deployment. Other brace-containing JSON arguments remain
literal. Commands must terminate successfully; infrastructure/hook failures classify
INCONCLUSIVE. Include explicit signer, network and any local CLI config-directory
arguments. Setup-generated copies do this without altering the engine.

## Optional fields

| Field | Meaning |
| --- | --- |
| `name`, `source` | Descriptive provenance metadata; no engine dispatch by target name. |
| `archive_timeout_seconds` | 1–3600, default 240. A timeout is INCONCLUSIVE, not archival proof. |
| `expected_initial` | Complete map of public binding names to expected values before mutation; required whenever `mutate` is supplied; used only with that hook. |
| `expected_before` | Complete map of public binding names to expected values after mutation/before archival. |
| `mutation_required` | Require `mutate`, distinct `expected_initial` / `expected_before` and observed X → Y. |

Even without explicit expected values, public/raw values must agree before and
after recovery and preserve the pre-archive invariant. Mutation-required Tansu
also asserts the initial and changed commit values explicitly. JSON types matter:
integer `1` is different from boolean `true`, including inside nested values. Configuration is
validated before starting; existing campaign output directories are refused.

## Output and lifecycle

`run` writes a resolved `<output>.scenario.json` beside the campaign directory.
Inside it, `config.json` records configuration; `checkpoint.json` records lifecycle
state, invariant values, named keys, eviction witnesses and restore batches;
`evidence-NNNNN.json` records argv/stdout/stderr or raw RPC bodies; `runner.json`
records separate process exits. A completed run writes `campaign-result.json`.

Phases are archive → deliberate negative verification → committed restore checkpoint
→ recovery/retry in a new process → final offline verification. Exit 2 at an
intentional checkpoint is accepted only at the expected boundary; a final PASS
requires every phase's exact expected outcome and independent evidence. Failed or
incomplete campaigns retain diagnostics rather than inventing a success report.

The lower-level `python3 -m archival_regression.campaign --help` exposes checkpoint
phases for inspection/recovery. Prefer `run` for the complete expected-negative and
retry sequence. See the two JSON fixtures for full examples; no new target requires
an engine target-name branch.

`source` is descriptive provenance, not a generic executable WASM attestation.
The bundled setup verifies fixture hashes before upload and checks uploaded hashes;
custom setup hooks must supply their own artifact/deployment verification. Arbitrary
ledger keys need to be known in advance; unsupported SCVal/XDR shapes classify
INCONCLUSIVE rather than being silently omitted. There is no generic shell sandbox,
secret scrubber or automatic key discovery. See [trust boundaries](limitations.md).
