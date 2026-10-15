/**
 * Partial client for the pool cr
 */
use soroban_sdk::{contractclient, contracttype, Address, Env, Map};

#[derive(Clone)]
#[contracttype]
pub struct Positions {
    pub liabilities: Map<u32, i128>, // Map of Reserve Index to liability share balance
    pub collateral: Map<u32, i128>,  // Map of Reserve Index to collateral supply share balance
    pub supply: Map<u32, i128>,      // Map of Reserve Index to non-collateral supply share balance
}

#[allow(dead_code)]
#[contractclient(name = "PoolClient")]
pub trait Pool {
    /// Fetch the positions for an address
    ///
    /// ### Arguments
    /// * `address` - The address to fetch positions for
    fn get_positions(e: Env, address: Address) -> Positions;
}
