#![no_std]

use soroban_sdk::{contract, contractimpl, token, vec, Address, Env, Symbol};

#[contract]
pub struct FlashLoanReceiverModifiedERC3156;

#[contractimpl]
impl FlashLoanReceiverModifiedERC3156 {
    /// Have the flash loan receiver attempt a re-entrant call with "pool".
    ///
    /// This will make `exec_op` call `submit` on the pool contract with a borrow request for
    /// `amount` of the token `token` from the `caller`. The flash loan will then return the tokens
    /// to the caller.
    pub fn set_re_entrant(env: Env, pool: Address) {
        env.storage()
            .instance()
            .set::<Symbol, Address>(&Symbol::new(&env, "pool"), &pool);
    }

    /// Do something to simulate a flash loan
    pub fn exec_op(env: Env, caller: Address, token: Address, amount: i128, _fee: i128) {
        // require the caller to authorize the invocation
        caller.require_auth();

        // perform a re-entrant call against the pool contract if set
        let key = Symbol::new(&env, "pool");
        if let Some(pool_id) = env.storage().instance().get::<Symbol, Address>(&key) {
            // call submit to borrow tokens to "caller" with "token" and "amount"
            let pool_client = pool::PoolClient::new(&env, &pool_id);
            pool_client.submit(
                &caller,
                &env.current_contract_address(),
                &caller,
                &vec![
                    &env,
                    pool::Request {
                        request_type: pool::RequestType::Borrow as u32,
                        address: token.clone(),
                        amount,
                    },
                ],
            );
        }

        // send tokens back
        let token_client = token::TokenClient::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &caller, &amount);
    }
}
