# Soroban Statecycle Archival Regression Campaign Guide

`soroban-statecycle` verifies smart contract state lifecycle transitions across live, archived, and restored states on Stellar Soroban networks.

---

## 1. Lifecycle Phases Verified

| Phase | Description | Preconditions |
|---|---|---|
| **Archive** | Verifies contract entries reach end of TTL and get evicted to historical archival storage. | Storage key live_until_ledger < current ledger |
| **Restore** | Re-activates evicted entries via state restoration transactions with valid restoration footprint. | Valid archive witness & restoration fee paid |
| **Recover** | Validates that contract execution succeeds identically pre- and post-restoration. | Monotonic TTL & conserved contract storage state |

---

## 2. CLI Usage

```bash
# Execute statecycle archival campaign
python3 -m archival_regression.campaign --config scenarios/vault.json --output run_vault/ --phase archive

# Resume and verify restored execution
python3 -m archival_regression.campaign --config scenarios/vault.json --output run_vault/ --phase recover

# Verify phase timings export
python3 -m archival_regression.campaign --config scenarios/vault.json --output run_vault/ --phase verify
```
