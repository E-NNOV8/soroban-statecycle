# Bounded second-target inspection

This inspection selects a validation target for the existing archival product;
it does not reopen project discovery. Sources are locally available tracked Git
revisions; no maintainer contact, fork, remote fetch or external writes occurred.

| Candidate | Actual storage/interfaces | Provisioning and reproduction | Archival fit | Decision |
|---|---|---|---|---|
| Soroban Forge marketplace royalties, `63d5c1ff5002427e57dab46fbc0c3269da823ccc` | `DataKey::Summary(Address)` stores cumulative sales/gross/royalties; `settle_sale` transfers real token funds before updating it; public summary getter. | Constructor-free WASM; collection/payer auth plus real SEP-41 token; unchanged isolated locked offline build succeeds. | All relevant writes call shared 30-day TTL extension. Core rejected attempted maxima 128 and 4096. No shortcut to archive is substituted. | Not selected for this bounded profile. Source/API fit is small general gap, but accelerated schedule is unresolved; no full campaign or compatibility PASS claimed. |
| StellarCred issuer registry | Persistent issuer/revocation/metadata and public admin operations. | Constructor admin; native dependency build needed. | Writes extend issuer keys to 120-day horizon. Same unproven maximum-TTL profile concern. | Not selected; source inspection only. |
| Tansu project/commit registry, `0524376d293be41ee3bf5ca102cee2a88bea3d03` | Persistent `ProjectKey::Key(Bytes)` contains project owner list/config; `LastHash(Bytes)` stores mutable last commit. Public authenticated `register`, `commit`, `get_commit`, `get_project`. Selected path does not extend TTL. | Constructor admin, genuine SAC collateral reference, unpause, register with real token collateral transfer, commit X then Y. Locked build initially lacked a cached crate; approved registry downloads enabled unchanged build. WASM retained with source/lock/hashes. | Natural network minimum TTL fits the verified existing Core profile. Exact instance/project/last-hash keys supplied in scenario; code restored as a dependency. | Selected independent project. Prospective SMALL GENERAL PRODUCT GAP: named keys, projections and setup/mutation/multi-action restore are all generic capabilities. |

Tansu is an application contract for project versioning/membership/governance,
not a counter/tutorial contract. The selected invariant is continuity of a
maintainer-authorized version record after real archival, including the project
entry that authorizes its owner and the recorded commit identifier. Two real Git
revision identifiers from Tansu's local history supply X and Y. The contract
validates identifier format; this campaign does not prove Git provenance or a
remote repository's contents on chain.

Source pointers in retained Tansu source:
`contracts/tansu/src/contract_tansu.rs` constructor/collateral configuration/pause;
`contract_versioning.rs` register at line 60, commit at 289, getter at 318;
`types.rs` ProjectKey at 190 and ContractRef at 8; `lib.rs` authorization and
contract-reference validation. No domain or NQG dependency is invoked by this
selected register/commit path. The SAC receives an actual collateral transfer.

Core's rejected settings are retained in `config-*`, `supported-config-*`, and
`config-validation-context.json`. Neither rejected setting was applied. Manual
close diagnostic routes rejected requests because the running configuration does
not enable manual close; no runtime configuration was changed and no ledger jump
occurred. No need to change Core timing arose for the selected Tansu path.

No new complete competing archival/invariant tool was discovered in this bounded
candidate source inspection. This is not an ecosystem-wide refreshed competitor
survey; the existing selection's competition/upstream risk remains unresolved at
its documented level. Native target tests and ordinary CLI restore primitives do
not themselves supply this complete real-Core campaign.
