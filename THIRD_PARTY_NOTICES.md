# Third-party materials

The root MIT license applies to first-party Soroban StateCycle code and documentation.
It does not replace upstream notices or license retained application artifacts.

| Material | Pinned revision / origin | License / corresponding source |
| --- | --- | --- |
| Blend V2 pool and pool factory WASMs | `blend-capital/blend-contracts-v2`, `ba22b487b2c5057a4ecc28b05b5193c28e4bd117` | GNU AGPL v3; [license](docs/licenses/Blend-AGPL-3.0.txt), complete tracked corresponding source in `docs/vendor/blend/`, including its original license and build instructions. |
| Tansu WASM and source | `starboytiimz/tansu` (recorded checkout origin), `0524376d293be41ee3bf5ca102cee2a88bea3d03` | Original [license text](docs/experiments/stage2/sources/tansu/LICENSE) retained verbatim, including its DAO-related introductory terms and BSD conditions. Source in `docs/experiments/stage2/sources/tansu/`; build/provenance in the Stage 2 record. |
| Soroban Forge inspected candidate WASM | Soroban Forge inspected checkout, `63d5c1ff5002427e57dab46fbc0c3269da823ccc` (recorded source origin in Stage 2 evidence) | [MIT notice](docs/licenses/Forge-MIT.txt); candidate research artifact only, not a supported third archival scenario. |

The source trees include their own dependency declarations, copyright and notices.
Retaining them does not assert ownership by StateCycle. The pinned Quickstart image
and Stellar CLI are external prerequisites downloaded from their upstream publishers;
their executable distributions are not bundled here.
