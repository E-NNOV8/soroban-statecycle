# Supported scope, limitations and trust boundaries

Version 0.1.0-alpha is a bounded local regression tool, not production infrastructure
or a security audit. Linux x86_64 / Python 3.12+ / CLI 28.1.0 / pinned Quickstart
Core 29.0.0 with active protocol 28 is the proven profile. Protocol 29, other XDR
versions, public testnet/mainnet, remote RPC, macOS/Windows/ARM and other archival
settings are NOT TESTED or unsupported; a failed environment is INCONCLUSIVE.
Setup enforces CLI 28.1.0; direct campaign execution assumes that prerequisite.

## Application and state boundaries

Only explicitly declared persistent ContractData keys, supported SCVal projections
and continuity/X/Y assertions are covered. There is no key/invariant discovery,
temporary-state restoration, universal nested-type support or economic correctness
solver. Deploy/public hooks require single receipt-producing argv commands; setup,
mutation and restore may be sequences. Unknown shapes are not silently skipped.
Custom commands must satisfy the documented JSON/receipt contract.

The short minimum TTL and early eviction scan are legitimate accelerated local
consensus settings; they do not measure default-network archival latency or cost.
Long application TTL extensions may not archive within the bounded timeout. Core
must expose contiguous retained ledger metadata; stale or incomplete metadata cannot
be treated as successful archival. Different settings require separate validation.

Blend proof covers initialized pool configuration/admin continuity through its real
factory, using valid placeholder addresses for dependencies not called on that
path. It covers no lending balances, reserves, liquidations or economic safety.
Tansu proof covers project/last-commit persistent state and its authenticated
mutation using a real SAC dependency. It does not verify remote Git authenticity,
full DAO/membership/NQG behavior or collateral-balance recovery. Forge was inspected,
not validated by an archival campaign. No maintainer adoption or endorsement,
production use or exhaustive application coverage is established.

Retry covers a separate process after committed restoration, not in-flight/partial
atomic failure or arbitrary node recovery. Negative enforces explicit restoration
and checks unavailable live state; it does not claim automatic restore universally
fails. Offline recovery replay alone is not a new live control/negative/retry run.

## Execution, files and secrets

Scenarios/hooks are **trusted executable input**, run with the caller's permissions.
The engine uses argv without an implicit shell, but a hook can deliberately invoke
one or any other executable. This is not a sandbox for untrusted scenarios or RPC.
Only the campaign RPC client refuses proxies/redirects; bootstrap uses the operator's
local networking environment. Run against operator-controlled development services.

Use dedicated fresh output directories. Existing outputs/config sidecars are refused,
receipts require local basenames, and resume requires unchanged configuration bytes.
Do not run concurrent writers on one output or supply attacker-controlled filesystem
symlinks. The engine is not a secure multi-user evidence service. The hosted verifier
checks archive paths, symlinks, duplicate members and an uncompressed-size bound,
extracting only into a disposable directory. Signed transactions/public addresses
are evidence; they are not private signing keys.

Setup generates private CLI identities under ignored `.local/` and omits their
key-generation output from logs. General hooks are **not automatically secret
scrubbed**: use signing aliases/config paths, never secrets in argv/stdin/stdout.
Review a custom hook's outputs before publishing evidence; protect custom output
locations separately. `.gitignore` is not access control. Hosted integration uploads
campaign/setup diagnostics but excludes the actual signing configuration directory.

## Provenance and retention

Manifest hashes detect absent/changed retained bytes relative to the checked-in
baseline; they are not signed attestations against coordinated replacement of
manifest and evidence. RPC/XDR is read from a trusted local node, not independently
consensus-verified as a public network light client. Source metadata in scenarios is
descriptive; bundled setup separately verifies fixture artifact/upload hashes.

Historical proof is attributed to its exact producer SHA and profile. A later source
commit needs affected tests and, for behavior/verification changes, new live Core
integration. Fast CI performs offline evidence replay on each push; expensive Core
integration is manual. GitHub artifacts/logs expire and workflows can be deleted;
hosted application proof, logs and metadata are now retained in Git with checksums
and an offline verifier; one standalone root-key diagnostic is explicitly redacted. See [hosted proof](hosted-proof.md) for scope and expiry dates.
