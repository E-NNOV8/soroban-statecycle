# Soroban StateCycle: Own-Contract Scenario Authoring Guide

This guide walks Soroban smart contract developers through translating their contract into an automated StateCycle regression test scenario (`scenario-v2`).

StateCycle tests persistent storage continuity against a genuine Stellar Core consensus node: verifying that your contract's state can be safely archived, reported as evicted by Core's eviction scanner, and fully recovered via `RestoreFootprintOp` transactions without silent state corruption or bricking.

---

## 1. The StateCycle Lifecycle Model

StateCycle executes a strict seven-phase verification pipeline:

```text
1. Deploy / Setup   → Deploys contract with fresh salt {salt}; records baseline state X.
2. Mutate (Optional)→ Executes authenticated state transition X → Y; captures live control.
3. Inactivity/Evict → Suspends calls until Core consensus evicts declared persistent keys.
4. Deliberate Neg.  → Verifies raw unavailability and proves un-restored calls fail (policy check).
5. RestoreFootprint → Submits real CLI restore transactions for all declared persistent keys.
6. Invariant Check  → Asserts raw SCVal equality and public getter equality (must match Y).
7. Retry / Re-verify→ Re-executes restoration against live state in a new process; re-verifies.
```

To run this automated lifecycle against your own contract, you author a single declarative JSON scenario file.

---

## 2. Fundamental Boundary: What StateCycle Can and Cannot Infer

A Soroban contract WASM binary contains exported interface specifications (the contract spec), describing method signatures, user-defined types, and error codes.

However, **a compiled WASM specification cannot infer runtime storage keys or signing context**:
1. **Dynamic Storage Keys**: In Soroban, persistent entries are created dynamically at runtime via `env.storage().persistent().set(&key, &val)`. The WASM specification does not enumerate which keys exist on the ledger or which entries are critical for continuity.
2. **Signing & Authorization Context**: Invoking state-modifying or restoring transactions requires explicit signer keys (`--source`), network passphrases, and transaction envelopes.
3. **Canonical Value Projections**: The relationship between a low-level `ContractData` SCVal (e.g., a custom struct in instance storage) and the public return value of a view function (`getter`) is application-specific.

Developers must supply these runtime keys, public getter hooks, and transaction credentials. StateCycle provides the execution engine, Core archival polling, and invariant validation.

---

## 3. Core Scenario Components

A valid `archival-regression/scenario-v2` file consists of five main sections:

### 3.1 Network & Target Configuration

```json
{
  "schema": "archival-regression/scenario-v2",
  "rpc_url": "http://localhost:8000/rpc",
  "passphrase": "Standalone Network ; February 2017",
  "protocol": 28,
  "archive_timeout_seconds": 240,
  "name": "my-custom-contract"
}
```

* `schema`: Must be exactly `"archival-regression/scenario-v2"`.
* `protocol`: Must be `28` (Stellar protocol 28 accelerated archival profile).
* `archive_timeout_seconds`: 1 to 3600 seconds (default: 240).

### 3.2 Identifying Persistent Storage Keys (`entries`)

StateCycle verifies archival and restoration for named persistent `ContractData` keys.

> **Rule:** StateCycle strictly requires `"durability": "persistent"`. Temporary storage cannot be restored once expired and will be rejected with an `INCONCLUSIVE` error.

#### Type A: Instance Storage
If your contract stores configuration, admin keys, or data in instance storage (`env.storage().instance()`):

```json
"entries": {
  "instance": {
    "contract_data": {
      "contract": "{id}",
      "key": "ledger_key_contract_instance",
      "durability": "persistent"
    }
  }
}
```

#### Type B: Named Persistent Data Keys
If your contract stores persistent entries using enum keys or tuples (e.g., `DataKey::Balance(Address)` or `DataKey::Record(BytesN<32>)`):

```json
"entries": {
  "record_entry": {
    "contract_data": {
      "contract": "{id}",
      "key": {
        "vec": [
          { "symbol": "Record" },
          { "bytes": "b87f8d993d6540af57a064ecba52be7bba2c641c342b3c470d7a3140393549b5" }
        ]
      },
      "durability": "persistent"
    }
  }
}
```

Format the `key` as canonical Soroban SCVal JSON (symbols, vectors, maps, addresses, or byte arrays).

### 3.3 Defining Public Getter Projections (`bindings`)

Bindings connect your contract's public read functions (`getters`) to the underlying raw ledger entries.

```json
"bindings": {
  "get_record": {
    "entry": "record_entry",
    "path": ["contract_data", "val"],
    "value_path": ["storage", "Record"]
  }
}
```

* `entry`: Must match one of the keys declared in `entries`.
* `path`: JSON path to locate the value within the decoded `LedgerEntryData` (typically `["contract_data", "val"]`).
* `value_path` (optional): Path within the normalized SCVal structure (e.g. if the entry is an instance storage map).
* **Important:** Every binding key name (e.g., `get_record`) **must have a matching hook** in `hooks`.

### 3.4 Declaring Command Hooks (`hooks`)

Hooks are structured `argv` arrays (never arbitrary shell strings) executed by the runner.

```json
"hooks": {
  "deploy": [
    "stellar", "contract", "deploy",
    "--wasm-hash", "YOUR_WASM_HASH",
    "--salt", "{salt}",
    "--rpc-url", "http://localhost:8000/rpc",
    "--network-passphrase", "Standalone Network ; February 2017",
    "--source", "archival-admin"
  ],
  "get_record": [
    "stellar", "contract", "invoke",
    "--id", "{id}",
    "--rpc-url", "http://localhost:8000/rpc",
    "--network-passphrase", "Standalone Network ; February 2017",
    "--source", "archival-admin",
    "--send", "yes",
    "--",
    "get_record"
  ],
  "restore": [
    "stellar", "contract", "restore",
    "--id", "{id}",
    "--rpc-url", "http://localhost:8000/rpc",
    "--network-passphrase", "Standalone Network ; February 2017",
    "--source", "archival-admin"
  ]
}
```

#### Hook Requirements:
1. **`deploy`** (Required): Returns the newly deployed contract ID (or JSON string containing it) on standard output. Use `{salt}` to ensure isolation across repeated test runs.
2. **Public Getters** (Required for each binding): Must use `--send yes`. StateCycle requires read transactions to be submitted to RPC to extract transaction hashes and receipts, proving that observation occurred on-ledger rather than in a local simulator cache.
3. **`restore`** (Required): Submits one or more `RestoreFootprint` operations covering all declared `entries`.
4. **`setup`** (Optional): Initialization call(s) executed immediately after deployment.
5. **`mutate`** (Optional): Authenticated transaction modifying the contract state from X to Y before the eviction window.

### 3.5 State Mutation & Assertions (`expected_initial` / `expected_before`)

If your scenario tests a mutable state transition (e.g., testing that state update `X → Y` persists across archival):

```json
"mutation_required": true,
"hooks": {
  "mutate": [
    "stellar", "contract", "invoke",
    "--id", "{id}",
    "--source", "archival-admin",
    "--send", "yes",
    "--",
    "update_record",
    "--new_value", "updated_state_value"
  ]
},
"expected_initial": {
  "get_record": "initial_state_value"
},
"expected_before": {
  "get_record": "updated_state_value"
}
```

* `expected_initial`: Expected getter output before `mutate` runs.
* `expected_before`: Expected getter output after `mutate` runs and before eviction.
* Both must cover every declared binding, and values must be strictly distinct.

---

## 4. Step-by-Step Walkthrough: Authoring for a Custom Contract

Let's author a scenario for a sample contract `CounterContract` with:
- Persistent instance storage holding an integer counter.
- A view method `get_count() -> u32`.
- A state-mutating method `increment() -> u32`.

### Step 1: Deploy & Inspect Initial Storage
Compile your contract:
```bash
cargo build --target wasm32-unknown-unknown --release
```

Deploy locally using `stellar contract deploy` and invoke `get_count`. Inspect the raw contract instance entry using:
```bash
stellar contract read --id <CONTRACT_ID> --durability persistent
```
Notice that instance storage lives under `ledger_key_contract_instance`.

### Step 2: Generate Initial Scenario Template
Use the StateCycle scaffolding utility:
```bash
python3 scripts/scaffold_scenario.py \
  --name counter-contract \
  --contract-id "{id}" \
  --bindings get_count \
  --has-mutation \
  --output scenarios/counter-contract.json
```

### Step 3: Populate Arguments and Expectations
Edit `scenarios/counter-contract.json` to configure:
1. The `deploy` argv with your WASM hash or binary path.
2. The `mutate` hook with `-- increment`.
3. Set `expected_initial` to `{"get_count": 0}` and `expected_before` to `{"get_count": 1}`.

### Step 4: Run Scenario Locally
Execute the regression test:
```bash
PYTHONPATH=src python3 -m archival_regression run \
  scenarios/counter-contract.json \
  --output outputs/counter-run-1
```

If any required eviction witness, restore footprint, or getter equality fails, StateCycle stops with a clear report and non-zero exit code.

---

## 5. Summary Checklist Before Running

- [ ] `schema` is `"archival-regression/scenario-v2"`.
- [ ] `protocol` is `28`.
- [ ] All `entries` have `"durability": "persistent"`.
- [ ] Every entry in `bindings` corresponds to an entry in `entries`.
- [ ] Every binding name has an identical hook name in `hooks`.
- [ ] All read getter hooks include `--send yes`.
- [ ] `restore` hook restores all declared persistent entries.
- [ ] If `mutation_required` is true, distinct `expected_initial` and `expected_before` are declared.
- [ ] Placeholder `{id}` is used for dynamic contract references in hooks.
- [ ] Placeholder `{salt}` is used in the `deploy` hook.
