use soroban_sdk::{contract, contractimpl, contracttype, map, Address, Env, Map};

const ONE_DAY_LEDGERS: u32 = 17280; // assumes 5s a ledger
const LEDGER_THRESHOLD: u32 = ONE_DAY_LEDGERS * 90;
const LEDGER_BUMP: u32 = ONE_DAY_LEDGERS * 120;

#[derive(Clone)]
#[contracttype]
pub struct Positions {
    pub liabilities: Map<u32, i128>, // Map of Reserve Index to liability share balance
    pub collateral: Map<u32, i128>,  // Map of Reserve Index to collateral supply share balance
    pub supply: Map<u32, i128>,      // Map of Reserve Index to non-collateral supply share balance
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Positions(Address),
}

#[contract]
pub struct MockPool;

#[contractimpl]
impl MockPool {
    /// Set positions for a given address
    ///
    /// # Arguments
    /// * 'address' - The address to set positions for
    /// * 'positions' - The positions to set
    pub fn set_positions(e: Env, address: Address, positions: Positions) {
        e.storage()
            .instance()
            .extend_ttl(LEDGER_THRESHOLD, LEDGER_BUMP);
        let key = DataKey::Positions(address);
        e.storage()
            .persistent()
            .set::<DataKey, Positions>(&key, &positions);
        e.storage()
            .persistent()
            .extend_ttl(&key, LEDGER_THRESHOLD, LEDGER_BUMP);
    }

    /// Fetch the positions for an address
    ///
    /// # Arguments
    /// * 'address' - The address to fetch positions for
    pub fn get_positions(e: Env, address: Address) -> Positions {
        let key = DataKey::Positions(address);
        match e.storage().persistent().get::<DataKey, Positions>(&key) {
            Some(positions) => positions,
            None => Positions {
                liabilities: map![&e],
                collateral: map![&e],
                supply: map![&e],
            },
        }
    }
}
