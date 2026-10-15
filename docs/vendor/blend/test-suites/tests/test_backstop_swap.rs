#![cfg(test)]

use backstop::BackstopClient;
use blend_contract_sdk::{backstop as v1_backstop, emitter};
use pool::{PoolClient, Request, RequestType, ReserveEmissionMetadata};
use pool_factory::{PoolFactoryClient, PoolInitMeta};
use sep_40_oracle::testutils::Asset;
use sep_41_token::testutils::MockTokenClient;
use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger},
    vec,
    xdr::Error,
    Address, BytesN, Env, String, Symbol, Val, Vec,
};
use test_suites::{
    assertions::{assert_approx_eq_abs, assert_approx_eq_rel},
    backstop::create_backstop,
    liquidity_pool::LPClient,
    oracle::create_mock_oracle,
    pool::{default_reserve_metadata, POOL_WASM},
    pool_factory::create_pool_factory,
    snapshot::{self, XLM_WHALE},
    test_fixture::SCALAR_7,
};

#[test]
fn test_v1_to_v2_backstop_swap() {
    let env = snapshot::env_from_snapshot();
    env.mock_all_auths();

    let frodo = Address::generate(&env);
    let samwise = Address::generate(&env);
    let merry = Address::generate(&env);

    // contracts shared between v1 and v2
    let blnd = Address::from_str(&env, snapshot::BLND_ID);
    let usdc = Address::from_str(&env, snapshot::USDC_ID);
    let backstop_token = Address::from_str(&env, snapshot::BLND_USDC_LP_ID);
    let emitter = Address::from_str(&env, snapshot::EMITTER_ID);
    let v1_backstop = Address::from_str(&env, snapshot::BACKSTOP_ID);
    let v1_pool = Address::from_str(&env, snapshot::V1_POOL_ID);

    let blnd_client = MockTokenClient::new(&env, &blnd);
    let usdc_client = MockTokenClient::new(&env, &usdc);
    let backstop_token_client = LPClient::new(&env, &backstop_token);
    let emitter_client = emitter::Client::new(&env, &emitter);
    let v1_backstop_client = v1_backstop::Client::new(&env, &v1_backstop);
    let v1_pool_client = PoolClient::new(&env, &v1_pool);

    // deploy v2 contracts
    let v2_backstop = Address::generate(&env);
    let v2_pool_factory = Address::generate(&env);

    let pool_hash = env.deployer().upload_contract_wasm(POOL_WASM);
    let pool_init_meta = PoolInitMeta {
        backstop: v2_backstop.clone(),
        pool_hash: pool_hash.clone(),
        blnd_id: blnd.clone(),
    };
    let v2_pool_factory_client = create_pool_factory(&env, &v2_pool_factory, true, pool_init_meta);

    let drop_list: Vec<(Address, i128)> = vec![
        &env,
        (frodo.clone(), 1_000_000 * 10i128.pow(7)),
        (samwise.clone(), 1_000_000 * 10i128.pow(7)),
        (v1_backstop.clone(), 1_000_000 * 10i128.pow(7)),
    ];
    let v2_backstop_client = create_backstop(
        &env,
        &v2_backstop,
        false,
        &backstop_token,
        &emitter,
        &blnd,
        &usdc,
        &v2_pool_factory,
        &drop_list,
    );

    // Backstop_v1 balance of BLND_USDC_LP tokens
    let mut to_match_v1 = backstop_token_client.balance(&v1_backstop);

    // Mint Merry LP tokens to deposit into v1 backstop
    mint_lp_tokens(&env, &backstop_token_client, &merry, 10_000 * SCALAR_7);
    v1_backstop_client.deposit(&merry, &v1_pool, &(10_000 * SCALAR_7));
    to_match_v1 += 10_000 * SCALAR_7;

    // Mint Merry USDC to deposit into v1 pool
    usdc_client.mint(&merry, &10_000_0000000);
    let requests: Vec<Request> = vec![
        &env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: usdc.clone(),
            amount: 10_000_0000000,
        },
    ];
    v1_pool_client.submit(&merry, &merry, &merry, &requests);

    // Mint LP tokens to frodo and samwise
    // -> mint frodo enough LP tokens to swap
    mint_lp_tokens(&env, &backstop_token_client, &frodo, to_match_v1);
    // -> mint samwise enough LP tokens to backstop a new pool on v2
    mint_lp_tokens(&env, &backstop_token_client, &samwise, 55_000 * SCALAR_7);

    // Create v2 pool
    let v2_pool_id = deploy_v2_pool(&env, &samwise, &v2_pool_factory_client, &v2_backstop_client);
    let v2_pool_client = PoolClient::new(&env, &v2_pool_id);
    // -> track deposit of 55k LP tokens into v2 backstop during deploy_v2_pool
    to_match_v1 -= 55_000 * SCALAR_7;

    // Test: Start backfilled emissions
    v2_backstop_client.distribute();

    // Time: pass 7 days
    jump(&env, 17280 * 7);

    // Test: Distribute and emit backfilled emissions
    emitter_client.distribute();
    // -> this fucntion was renamed on v2, just invoke v1 method directly
    env.invoke_contract::<Val>(
        &v1_backstop,
        &Symbol::new(&env, "gulp_emissions"),
        vec![&env] as Vec<Val>,
    );
    v1_pool_client.gulp_emissions();
    v2_backstop_client.distribute();
    v2_pool_client.gulp_emissions();

    // start backstop swap
    // -> in v1 merry earns about 200 LP tokens per week in emissions (~5 weeks of emissions)
    let approx_v1_new_lp_tokens = 200 * SCALAR_7 * 5;
    v2_backstop_client.deposit(
        &frodo,
        &v2_pool_id,
        &(to_match_v1 + approx_v1_new_lp_tokens),
    );
    emitter_client.queue_swap_backstop(&v2_backstop, &backstop_token);

    // Time: pass 7 days (7 days since swap)
    jump(&env, 17280 * 7);

    // -> distribute emissions
    emitter_client.distribute();
    env.invoke_contract::<Val>(
        &v1_backstop,
        &Symbol::new(&env, "gulp_emissions"),
        vec![&env] as Vec<Val>,
    );
    v1_pool_client.gulp_emissions();
    v2_backstop_client.distribute();
    v2_pool_client.gulp_emissions();

    // Test: Validate v1 still getting emissions and v2 cannot claim
    v1_backstop_client.claim(&merry, &vec![&env, v1_pool.clone()], &merry);
    v1_pool_client.claim(&merry, &vec![&env, 3], &merry);
    assert!(v2_backstop_client
        .try_claim(&samwise, &vec![&env, v2_pool_id.clone()], &0)
        .is_err());
    assert!(v2_pool_client
        .try_claim(&samwise, &vec![&env, 1, 3], &samwise)
        .is_err());

    // Time: pass 7 days (14 days since swap)
    jump(&env, 17280 * 7);
    // -> track 7 days worth of emissions for v1
    let v1_7_days_backstop = v1_backstop_client.claim(&merry, &vec![&env, v1_pool.clone()], &merry);
    assert!(v1_7_days_backstop > 0);
    let v1_7_days_pool = v1_pool_client.claim(&merry, &vec![&env, 3], &merry);
    assert!(v1_7_days_pool > 0);

    // Test: Validate v1 gets all emissions up until swap,
    //       the swap is successful, and v2 emits all backfilled emissions after swap
    // -> distribute emissions
    emitter_client.distribute();
    env.invoke_contract::<Val>(
        &v1_backstop,
        &Symbol::new(&env, "gulp_emissions"),
        vec![&env] as Vec<Val>,
    );
    v1_pool_client.gulp_emissions();
    v2_backstop_client.distribute();
    v2_pool_client.gulp_emissions();

    // Time: pass 7 days (21 days since swap)
    jump(&env, 17280 * 7);

    // -> distribute emissions
    emitter_client.distribute();
    env.invoke_contract::<Val>(
        &v1_backstop,
        &Symbol::new(&env, "gulp_emissions"),
        vec![&env] as Vec<Val>,
    );
    v1_pool_client.gulp_emissions();
    v2_backstop_client.distribute();
    v2_pool_client.gulp_emissions();

    // Time: pass 7 days (28 days since swap)
    jump(&env, 17280 * 7);
    // -> claim emissions
    v1_backstop_client.claim(&merry, &vec![&env, v1_pool.clone()], &merry);
    v1_pool_client.claim(&merry, &vec![&env, 3], &merry);

    // -> distribute emissions for v1
    emitter_client.distribute();
    env.invoke_contract::<Val>(
        &v1_backstop,
        &Symbol::new(&env, "gulp_emissions"),
        vec![&env] as Vec<Val>,
    );
    v1_pool_client.gulp_emissions();
    v2_backstop_client.distribute();
    v2_pool_client.gulp_emissions();

    // Time: pass 3 days (31 days since swap)
    jump(&env, 17280 * 3 + 1);

    // -> do swap
    emitter_client.swap_backstop();
    assert_eq!(emitter_client.get_backstop(), v2_backstop);
    assert_eq!(
        emitter_client.get_last_distro(&v2_backstop),
        env.ledger().timestamp()
    );

    // -> v1 emitter distribute run automatically
    // -> start v2 emissions after swap
    v2_backstop_client.distribute();

    // Test: Validate claim fails until gulp is run for v2
    assert!(v2_backstop_client
        .try_claim(&samwise, &vec![&env, v2_pool_id.clone()], &0)
        .is_err());
    assert!(v2_pool_client
        .try_claim(&samwise, &vec![&env, 1, 3], &samwise)
        .is_err());
    let blnd_balance_pre_drop = blnd_client.balance(&v2_backstop);
    assert_eq!(blnd_balance_pre_drop, 0);

    v2_backstop_client.drop();

    // gets 35d worth of tokens at 1 token per second, the 3d between pre-swap distribution
    // and the last distribution are lost, but this is expected
    let backfill_tokens_emitted = 35 * 24 * 60 * 60 * SCALAR_7;
    assert_eq!(blnd_client.balance(&v2_backstop), backfill_tokens_emitted);

    // Time: pass 4 days (4 days since swap)
    jump(&env, 17280 * 4);

    // -> claim backfilled emisions of 35d
    // -> frodo gets virtually all backstop emissions (70% of emissions)
    let pre_v2_backstop_claim_blnd = blnd_client.balance(&backstop_token);
    let pre_v2_backstop_balance = v2_backstop_client.user_balance(&v2_pool_id, &frodo).shares;
    let v2_tokens_minted = v2_backstop_client.claim(&frodo, &vec![&env, v2_pool_id.clone()], &0);
    let v2_backstop_claim_blnd = blnd_client.balance(&backstop_token) - pre_v2_backstop_claim_blnd;
    assert_approx_eq_rel(
        v2_backstop_claim_blnd,
        backfill_tokens_emitted
            .fixed_mul_floor(0_7000000, SCALAR_7)
            .unwrap(),
        0_0500000,
    );
    // shares are still 1 to 1
    let v2_backstop_claim_added_tokens =
        v2_backstop_client.user_balance(&v2_pool_id, &frodo).shares - pre_v2_backstop_balance;
    assert_eq!(v2_backstop_claim_added_tokens, v2_tokens_minted);

    // -> sawise gets all pool emissions as only user (30% of emissions)
    let v2_pool_claim = v2_pool_client.claim(&samwise, &vec![&env, 1, 3], &samwise);
    assert_approx_eq_rel(
        v2_pool_claim,
        backfill_tokens_emitted
            .fixed_mul_floor(0_3000000, SCALAR_7)
            .unwrap(),
        0_0100000,
    );

    emitter_client.distribute();
    // -> claim v1 emissions
    v1_backstop_client.claim(&merry, &vec![&env, v1_pool.clone()], &merry);
    v1_pool_client.claim(&merry, &vec![&env, 3], &merry);
    // -> distribute v1 emissions (distributes 3 days of emissions over the next 7 days)
    env.invoke_contract::<Val>(
        &v1_backstop,
        &Symbol::new(&env, "gulp_emissions"),
        vec![&env] as Vec<Val>,
    );
    v1_pool_client.gulp_emissions();
    // -> distribute v2 emissions ()
    v2_backstop_client.distribute();
    v2_pool_client.gulp_emissions();

    // Time: pass 7 days (11 days since swap)
    jump(&env, 17280 * 7);
    // -> claim v2 emissions and validate approx. 4 days worth of emissions are claimed
    let tokens_emitted: i128 = 4 * 24 * 60 * 60 * SCALAR_7;
    // -> frodo gets virtually all backstop emissions (0.7 tokens per second)
    let pre_v2_backstop_claim_blnd = blnd_client.balance(&backstop_token);
    let pre_v2_backstop_balance = v2_backstop_client.user_balance(&v2_pool_id, &frodo).shares;
    let v2_tokens_minted = v2_backstop_client.claim(&frodo, &vec![&env, v2_pool_id.clone()], &0);
    let v2_backstop_claim_blnd = blnd_client.balance(&backstop_token) - pre_v2_backstop_claim_blnd;
    assert_approx_eq_rel(
        v2_backstop_claim_blnd,
        tokens_emitted.fixed_mul_floor(0_7000000, SCALAR_7).unwrap(),
        0_0500000,
    );
    // shares are still 1 to 1
    let v2_backstop_claim_added_tokens =
        v2_backstop_client.user_balance(&v2_pool_id, &frodo).shares - pre_v2_backstop_balance;
    assert_eq!(v2_backstop_claim_added_tokens, v2_tokens_minted);

    // -> sawise gets all pool emissions as only user (0.3 tokens per second)
    let v2_pool_claim = v2_pool_client.claim(&samwise, &vec![&env, 1, 3], &samwise);
    assert_approx_eq_rel(
        v2_pool_claim,
        tokens_emitted.fixed_mul_floor(0_3000000, SCALAR_7).unwrap(),
        0_0500000,
    );

    // Time: pass 1 day (12 days since swap) to ensure emissions for pool 1 are over
    jump(&env, 17280);

    // -> claim v1 emissions and validate approx. 3 days worth of emissions are claimed.
    //    The wide bounds are due to multiple factors on the snapshot causing emissions to fluctuate
    //    between claim periods. Ultimately, this just validates that claim is working.
    let v1_backstop_claim = v1_backstop_client.claim(&merry, &vec![&env, v1_pool.clone()], &merry);
    assert_approx_eq_rel(
        v1_backstop_claim,
        v1_7_days_backstop
            .fixed_mul_floor(3_0000000, 7_0000000)
            .unwrap(),
        0_1000000,
    );
    let v1_pool_claim = v1_pool_client.claim(&merry, &vec![&env, 3], &merry);
    assert_approx_eq_rel(
        v1_pool_claim,
        v1_7_days_pool
            .fixed_mul_floor(3_0000000, 7_0000000)
            .unwrap(),
        0_1000000,
    );

    // Time: pass 1 day to validate v1 is not getting emissions
    jump(&env, 17280);

    // Test: validate emissons stop to v1
    // -> distribute v1 emissions
    emitter_client.distribute();
    assert!(env
        .try_invoke_contract::<Val, Error>(
            &v1_backstop,
            &Symbol::new(&env, "gulp_emissions"),
            vec![&env] as Vec<Val>
        )
        .is_err());
    assert!(v1_pool_client.try_gulp_emissions().is_err());
    // -> claim v1 emissions and validate no emissions are claimed
    assert_eq!(
        v1_backstop_client.claim(&merry, &vec![&env, v1_pool.clone()], &merry),
        0
    );
    assert_eq!(v1_pool_client.claim(&merry, &vec![&env, 3], &merry), 0);
}

#[test]
fn test_v1_to_v2_backstop_swap_past_max_emissions() {
    let env = snapshot::env_from_snapshot();
    env.mock_all_auths();

    let frodo = Address::generate(&env);
    let samwise = Address::generate(&env);
    let merry = Address::generate(&env);

    // contracts shared between v1 and v2
    let blnd = Address::from_str(&env, snapshot::BLND_ID);
    let usdc = Address::from_str(&env, snapshot::USDC_ID);
    let xlm = Address::from_str(&env, snapshot::XLM_ID);
    let backstop_token = Address::from_str(&env, snapshot::BLND_USDC_LP_ID);
    let emitter = Address::from_str(&env, snapshot::EMITTER_ID);
    let v1_backstop = Address::from_str(&env, snapshot::BACKSTOP_ID);
    let v1_pool = Address::from_str(&env, snapshot::V1_POOL_ID);

    let blnd_client = MockTokenClient::new(&env, &blnd);
    let usdc_client = MockTokenClient::new(&env, &usdc);
    let backstop_token_client = LPClient::new(&env, &backstop_token);
    let emitter_client = emitter::Client::new(&env, &emitter);
    let v1_backstop_client = v1_backstop::Client::new(&env, &v1_backstop);
    let v1_pool_client = PoolClient::new(&env, &v1_pool);

    // deploy v2 contracts
    let v2_backstop = Address::generate(&env);
    let v2_pool_factory = Address::generate(&env);

    let pool_hash = env.deployer().upload_contract_wasm(POOL_WASM);
    let pool_init_meta = PoolInitMeta {
        backstop: v2_backstop.clone(),
        pool_hash: pool_hash.clone(),
        blnd_id: blnd.clone(),
    };
    let v2_pool_factory_client = create_pool_factory(&env, &v2_pool_factory, true, pool_init_meta);

    let drop_list: Vec<(Address, i128)> = vec![
        &env,
        (frodo.clone(), 1_000_000 * 10i128.pow(7)),
        (samwise.clone(), 1_000_000 * 10i128.pow(7)),
        (v1_backstop.clone(), 1_000_000 * 10i128.pow(7)),
    ];
    let v2_backstop_client = create_backstop(
        &env,
        &v2_backstop,
        false,
        &backstop_token,
        &emitter,
        &blnd,
        &usdc,
        &v2_pool_factory,
        &drop_list,
    );

    /*
     * Setup v1 user Merry
     * -> 10k LP token deposit
     * -> 10k USDC supply
     * -> 5k USDC borrow
     */

    // Mint Merry LP tokens to deposit into v1 backstop
    mint_lp_tokens(&env, &backstop_token_client, &merry, 10_000 * SCALAR_7);
    v1_backstop_client.deposit(&merry, &v1_pool, &(10_000 * SCALAR_7));

    // Mint Merry USDC to deposit into v1 pool
    usdc_client.mint(&merry, &10_000_0000000);
    mint_xlm(&env, &merry, &10_000_0000000);
    let requests: Vec<Request> = vec![
        &env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: usdc.clone(),
            amount: 10_000_0000000,
        },
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: xlm.clone(),
            amount: 10_000_0000000,
        },
    ];
    v1_pool_client.submit(&merry, &merry, &merry, &requests);

    /*
     * Setup v2 pool with samwise as the only user
     */
    mint_lp_tokens(&env, &backstop_token_client, &samwise, 55_000 * SCALAR_7);
    let v2_pool_id = deploy_v2_pool(&env, &samwise, &v2_pool_factory_client, &v2_backstop_client);
    let v2_pool_client = PoolClient::new(&env, &v2_pool_id);

    // bump drop list to max TTL so it doesn't expire
    env.as_contract(&v2_backstop, || {
        let key = Symbol::new(&env, "DropList");
        env.storage()
            .persistent()
            .extend_ttl(&key, 3110400, 3110400);
    });

    // Test: Start backfilled emissions
    v2_backstop_client.distribute();

    /*
     * Fill `MAX_BACKFILLED_EMISSIONS`. This takes 10,000,000 seconds, or
     * 2,000,000 blocks. Touch the data every week to ensure it doesn't get expried, and emissions keep
     * getting distributed.
     *
     * MAX_BACKFILL will be hit in ~16.5 weeks.
     *
     * Wait 1 more week after final distribute to ensure all v2 emissions have been distributed
     */
    // mint lp tokens to deposit dust to keep backstop rent paid
    mint_lp_tokens(&env, &backstop_token_client, &samwise, 1_0000000);
    let mut total_v2_dist = 0;
    for _ in 0..17 {
        // jump a week
        jump(&env, 17280 * 7);

        // distribute everything
        emitter_client.distribute();
        env.invoke_contract::<Val>(
            &v1_backstop,
            &Symbol::new(&env, "gulp_emissions"),
            vec![&env] as Vec<Val>,
        );
        v1_pool_client.gulp_emissions();
        total_v2_dist += v2_backstop_client.distribute();
        v2_pool_client.gulp_emissions();

        // touch positions for v2 pool
        let requests: Vec<Request> = vec![
            &env,
            Request {
                request_type: RequestType::SupplyCollateral as u32,
                address: xlm.clone(),
                amount: 10,
            },
            Request {
                request_type: RequestType::SupplyCollateral as u32,
                address: usdc.clone(),
                amount: 10,
            },
        ];
        v2_pool_client.submit(&samwise, &samwise, &samwise, &requests);
        v2_backstop_client.deposit(&samwise, &v2_pool_id, &10);

        // claim emissions v1 (causes rent bumps)
        v1_backstop_client.claim(&merry, &vec![&env, v1_pool.clone()], &merry);
        v1_pool_client.claim(&merry, &vec![&env, 1, 3], &merry);
    }
    assert_eq!(total_v2_dist, 10_000_000_0000000);

    // wait 1 more week to ensure v2 emissions are over
    jump(&env, 17280 * 7);
    // -> distribute everything
    emitter_client.distribute();
    env.invoke_contract::<Val>(
        &v1_backstop,
        &Symbol::new(&env, "gulp_emissions"),
        vec![&env] as Vec<Val>,
    );
    v1_pool_client.gulp_emissions();
    let v2_dist_result = v2_backstop_client.try_distribute();
    assert!(v2_dist_result.is_err());
    let v2_gulp_result = v2_pool_client.try_gulp_emissions();
    assert!(v2_gulp_result.is_err());
    // -> claim emissions v1 (causes rent bumps)
    let mut v1_backstop_weekly_claim =
        v1_backstop_client.claim(&merry, &vec![&env, v1_pool.clone()], &merry);
    let v1_pool_weekly_claim = v1_pool_client.claim(&merry, &vec![&env, 1, 3], &merry);

    /*
     * Start backstop swap
     */
    // Have frodo mint excess LP tokens required to start backstop swap, plus some buffer
    let to_exceed_v1 = backstop_token_client.balance(&v1_backstop)
        - backstop_token_client.balance(&v2_backstop)
        + 10_000_0000000;
    mint_lp_tokens(&env, &backstop_token_client, &frodo, to_exceed_v1);
    v2_backstop_client.deposit(&frodo, &v2_pool_id, &to_exceed_v1);
    emitter_client.queue_swap_backstop(&v2_backstop, &backstop_token);

    /*
     * Go through 31 day swap period. Each week, distribute emissions, claim for v1.
     *
     * Verify v2 is not getting any additional emissions, but positions can still be modified.
     */
    mint_lp_tokens(&env, &backstop_token_client, &frodo, 50_0000000);
    usdc_client.mint(&frodo, &50_0000000);
    mint_xlm(&env, &frodo, &50_0000000);

    // Time: pass 28 days (28 days since swap)
    for _ in 0..4 {
        // -> jump a week
        jump(&env, 17280 * 7);
        // -> distribute everything
        emitter_client.distribute();
        env.invoke_contract::<Val>(
            &v1_backstop,
            &Symbol::new(&env, "gulp_emissions"),
            vec![&env] as Vec<Val>,
        );
        v1_pool_client.gulp_emissions();
        let v2_dist_result = v2_backstop_client.try_distribute();
        assert!(v2_dist_result.is_err());
        let v2_gulp_result = v2_pool_client.try_gulp_emissions();
        assert!(v2_gulp_result.is_err());
        // -> claim emissions v1 (causes rent bumps)
        let v1_backstop_claim_amt =
            v1_backstop_client.claim(&merry, &vec![&env, v1_pool.clone()], &merry);
        assert_approx_eq_rel(v1_backstop_claim_amt, v1_backstop_weekly_claim, 0_1000000);
        v1_backstop_weekly_claim = v1_backstop_claim_amt;
        let v1_pool_claim_amt = v1_pool_client.claim(&merry, &vec![&env, 1, 3], &merry);
        assert_approx_eq_rel(v1_pool_claim_amt, v1_pool_weekly_claim, 0_0100000);

        // touch positions for v2 pool
        let requests: Vec<Request> = vec![
            &env,
            Request {
                request_type: RequestType::SupplyCollateral as u32,
                address: xlm.clone(),
                amount: 10,
            },
            Request {
                request_type: RequestType::SupplyCollateral as u32,
                address: usdc.clone(),
                amount: 10,
            },
        ];
        v2_pool_client.submit(&frodo, &frodo, &frodo, &requests);
        v2_backstop_client.deposit(&frodo, &v2_pool_id, &10);
    }

    // Time: pass 4 days and perform the swap
    jump(&env, 17280 * 4);

    // -> do swap
    emitter_client.swap_backstop();
    assert_eq!(emitter_client.get_backstop(), v2_backstop);

    // -> drop
    let blnd_balance_pre_drop = blnd_client.balance(&v2_backstop);
    let samwise_balance_pre_drop = blnd_client.balance(&samwise);
    let frodo_balance_pre_drop = blnd_client.balance(&frodo);
    v2_backstop_client.drop();
    assert_eq!(
        blnd_client.balance(&v2_backstop),
        blnd_balance_pre_drop + 10_000_000_0000000
    );
    assert_eq!(
        blnd_client.balance(&samwise),
        samwise_balance_pre_drop + 1_000_000_0000000
    );
    assert_eq!(
        blnd_client.balance(&frodo),
        frodo_balance_pre_drop + 1_000_000_0000000
    );

    // -> distribute v2 (emitter should be updated)
    let v2_dist_result_0 = v2_backstop_client.distribute();
    assert_eq!(v2_dist_result_0, 0);
    // -> this will still fail as no tokens have been distributed yet
    let v2_gulp_result = v2_pool_client.try_gulp_emissions();
    assert!(v2_gulp_result.is_err());

    /*
     * Claim backfilled emissions
     */
    let pre_backstop_claim_blnd = blnd_client.balance(&v2_backstop);
    v2_backstop_client.claim(&samwise, &vec![&env, v2_pool_id.clone()], &0);
    let v2_backfill_blnd_0 = pre_backstop_claim_blnd - blnd_client.balance(&v2_backstop);
    // -> backstop gets 70% of emissions, samwise is only backstop user (some rounding loss expected)
    assert_approx_eq_abs(v2_backfill_blnd_0, 7_000_000_0000000, 0_0001000);
    // -> pool gets 30% of emissions, samwise is only pool user (some rounding loss expected)
    let v2_backfill_pool_claim = v2_pool_client.claim(&samwise, &vec![&env, 1, 3], &samwise);
    assert_approx_eq_abs(v2_backfill_pool_claim, 3_000_000_0000000, 0_0001000);
    let v2_backfill_frodo_claim =
        v2_backstop_client.claim(&frodo, &vec![&env, v2_pool_id.clone()], &0);
    assert_eq!(v2_backfill_frodo_claim, 0);

    // Time: pass 3 days
    // -> 3 days after swap and start of v2 emissions
    // -> end of last full emission v1 period, 4 days left undistributed
    jump(&env, 17280 * 3);

    // -> distribute v1
    env.invoke_contract::<Val>(
        &v1_backstop,
        &Symbol::new(&env, "gulp_emissions"),
        vec![&env] as Vec<Val>,
    );
    v1_pool_client.gulp_emissions();

    // claim v1 emissions (are from last period for the full week)
    let v1_backstop_claim = v1_backstop_client.claim(&merry, &vec![&env, v1_pool.clone()], &merry);
    let v1_pool_claim = v1_pool_client.claim(&merry, &vec![&env, 1, 3], &merry);
    assert_approx_eq_rel(v1_backstop_claim, v1_backstop_weekly_claim, 0_1000000);
    assert_approx_eq_rel(v1_pool_claim, v1_pool_weekly_claim, 0_0100000);

    // Time: pass 4 days
    // -> 1 week after swap and start of v2 emission
    // -> 4/7 days of last partial v1 emissions
    jump(&env, 17280 * 4);

    // -> distribute everything
    emitter_client.distribute();
    // assert v1 distribute no longer works
    assert!(env
        .try_invoke_contract::<Val, Error>(
            &v1_backstop,
            &Symbol::new(&env, "gulp_emissions"),
            vec![&env] as Vec<Val>
        )
        .is_err());
    assert!(v1_pool_client.try_gulp_emissions().is_err());
    // validate v2 distribution amounts
    let v2_dist_result = v2_backstop_client.distribute();
    assert_approx_eq_abs(v2_dist_result, 17280 * 7 * 5 * SCALAR_7, 5_0000000);
    let v2_gulp_result = v2_pool_client.gulp_emissions();
    assert_approx_eq_abs(v2_gulp_result, 181_440 * SCALAR_7, 5_0000000);

    // Time: pass 3 days
    // -> final day of partial v1 emissions
    jump(&env, 17280 * 3);

    // claim partial v1 emissions
    let v1_backstop_claim = v1_backstop_client.claim(&merry, &vec![&env, v1_pool.clone()], &merry);
    let v1_pool_claim = v1_pool_client.claim(&merry, &vec![&env, 1, 3], &merry);
    assert_approx_eq_rel(
        v1_backstop_claim,
        v1_backstop_weekly_claim
            .fixed_mul_floor(4_0000000, 7_0000000)
            .unwrap(),
        0_1000000,
    );
    assert_approx_eq_rel(
        v1_pool_claim,
        v1_pool_weekly_claim
            .fixed_mul_floor(4_0000000, 7_0000000)
            .unwrap(),
        0_0100000,
    );

    // Time: pass 4 days
    // -> end of 1st week of distributed v2 emissions
    jump(&env, 17280 * 4);

    // -> validate v1 claim is zero now
    let v1_backstop_claim = v1_backstop_client.claim(&merry, &vec![&env, v1_pool.clone()], &merry);
    let v1_pool_claim = v1_pool_client.claim(&merry, &vec![&env, 1, 3], &merry);
    assert_eq!(v1_backstop_claim, 0);
    assert_eq!(v1_pool_claim, 0);

    // -> validate v2 claim is correct for positions held during the full backfill period
    let est_backstop_blnd_weekly = 423_360 * SCALAR_7;
    let frodo_backstop_balance = v2_backstop_client.user_balance(&v2_pool_id, &frodo).shares;
    let samwise_backstop_balance = v2_backstop_client
        .user_balance(&v2_pool_id, &samwise)
        .shares;
    let total_backstop_balance = frodo_backstop_balance + samwise_backstop_balance;

    let backstop_balance_pre = blnd_client.balance(&v2_backstop);
    v2_backstop_client.claim(&frodo, &vec![&env, v2_pool_id.clone()], &0);
    let backstop_balance_mid = blnd_client.balance(&v2_backstop);
    let v2_frodo_claim_blnd = backstop_balance_pre - backstop_balance_mid;
    assert_approx_eq_rel(
        v2_frodo_claim_blnd,
        frodo_backstop_balance
            .fixed_div_floor(total_backstop_balance, SCALAR_7)
            .unwrap()
            .fixed_mul_floor(est_backstop_blnd_weekly, SCALAR_7)
            .unwrap(),
        0_0500000,
    );

    v2_backstop_client.claim(&samwise, &vec![&env, v2_pool_id.clone()], &0);
    let backstop_balance_post = blnd_client.balance(&v2_backstop);
    let v2_samwise_claim_blnd = backstop_balance_mid - backstop_balance_post;
    assert_approx_eq_rel(
        v2_samwise_claim_blnd,
        samwise_backstop_balance
            .fixed_div_floor(total_backstop_balance, SCALAR_7)
            .unwrap()
            .fixed_mul_floor(est_backstop_blnd_weekly, SCALAR_7)
            .unwrap(),
        0_0500000,
    );
    assert_approx_eq_abs(
        v2_samwise_claim_blnd + v2_frodo_claim_blnd,
        423_360 * SCALAR_7,
        SCALAR_7,
    );

    let v2_pool_claim = v2_pool_client.claim(&samwise, &vec![&env, 1, 3], &samwise);
    assert_approx_eq_abs(v2_pool_claim, 181_440 * SCALAR_7, SCALAR_7);
}

/***** Test Helpers *****/

/// Jump the timestamp by "blocks" time, assuming each block is 5 seconds.
/// This does not actually jump the sequence number by "blocks", and instead
/// only increments it.
fn jump(env: &Env, blocks: u32) {
    let seconds_passed: u64 = (blocks as u64) * 5;
    env.ledger()
        .set_sequence_number(env.ledger().sequence() + 10);
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + seconds_passed);
}

/// Mint "amount" LP tokens to "user". This assumes the LP token is the starting
/// weights and no swaps have been made.
fn mint_lp_tokens(env: &Env, client: &LPClient, user: &Address, amount: i128) {
    let blnd = Address::from_str(&env, snapshot::BLND_ID);
    let usdc = Address::from_str(&env, snapshot::USDC_ID);
    let blnd_client = MockTokenClient::new(&env, &blnd);
    let usdc_client = MockTokenClient::new(&env, &usdc);
    // @ snapshot, LP shares are worth are ~0.09 USDC and ~4.401 BLND each
    let blnd_mint_amount = amount.fixed_mul_floor(4_40100000, SCALAR_7).unwrap();
    let usdc_mint_amount = amount.fixed_mul_floor(0_09000000, SCALAR_7).unwrap();
    blnd_client.mint(&user, &blnd_mint_amount);
    usdc_client.mint(&user, &usdc_mint_amount);
    client.join_pool(
        &amount,
        &vec![&env, blnd_mint_amount.clone(), usdc_mint_amount.clone()],
        &user,
    );
}

/// Mint XLM by transfering from a whale as explicit token minting is not supported
fn mint_xlm(env: &Env, user: &Address, amount: &i128) {
    let client = MockTokenClient::new(&env, &Address::from_str(&env, snapshot::XLM_ID));
    client.transfer(&Address::from_str(&env, XLM_WHALE), user, amount);
}

/// Deploy a basic v2 pool, enable borrowing, and have deployer setup positions
///
/// Creator must have 55k LP tokens to enable borrowing.
fn deploy_v2_pool(
    env: &Env,
    creator: &Address,
    pool_factory_client: &PoolFactoryClient,
    backstop_client: &BackstopClient,
) -> Address {
    let xlm = Address::from_str(&env, snapshot::XLM_ID);
    let usdc = Address::from_str(&env, snapshot::USDC_ID);

    // create an oracle for USDC and XLM pool
    let (oracle_id, mock_oracle_client) = create_mock_oracle(&env);
    mock_oracle_client.set_data(
        &creator,
        &Asset::Other(Symbol::new(&env, "USD")),
        &vec![
            &env,
            Asset::Stellar(xlm.clone()),
            Asset::Stellar(usdc.clone()),
        ],
        &7,
        &300,
    );
    mock_oracle_client.set_price_stable(&vec![
        &env, 0_4200000, // xlm
        1_0000000, // usdc
    ]);

    // create a pool
    let pool_id = pool_factory_client.deploy(
        &creator,
        &String::from_str(&env, "V2"),
        &BytesN::<32>::random(&env),
        &oracle_id,
        &0_1000000,
        &4,
        &0,
    );
    let pool_client = PoolClient::new(&env, &pool_id);

    // setup reserves
    let mut xlm_config = default_reserve_metadata();
    xlm_config.c_factor = 0_750_0000;
    xlm_config.l_factor = 0_750_0000;
    xlm_config.util = 0_500_0000;
    xlm_config.max_util = 0_900_0000;
    pool_client.queue_set_reserve(&xlm, &xlm_config);
    pool_client.set_reserve(&xlm);

    let mut usdc_config = default_reserve_metadata();
    usdc_config.c_factor = 0_900_0000;
    usdc_config.l_factor = 0_950_0000;
    usdc_config.util = 0_800_0000;
    pool_client.queue_set_reserve(&usdc, &usdc_config);
    pool_client.set_reserve(&usdc);

    // split emissions equally between XLM and USDC supplying
    let reserve_emissions = vec![
        &env,
        ReserveEmissionMetadata {
            res_index: 0, // XLM
            res_type: 1,  // b_token
            share: 1_0000000,
        },
        ReserveEmissionMetadata {
            res_index: 1, // USDC
            res_type: 1,  // b_token
            share: 1_0000000,
        },
    ];
    pool_client.set_emissions_config(&reserve_emissions);

    // setup backstop and enable borrowing
    backstop_client.deposit(&creator, &pool_id, &(55_000 * SCALAR_7));
    backstop_client.add_reward(&pool_id, &None);
    pool_client.set_status(&3);
    pool_client.update_status();

    // creator adds liquidity to the pool
    let usdc_client = MockTokenClient::new(&env, &usdc);
    // xlm does not allow minting - take from a whale
    mint_xlm(&env, &creator, &10_000_0000000);
    usdc_client.mint(&creator, &5_000_0000000);

    let requests: Vec<Request> = vec![
        &env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: xlm.clone(),
            amount: 10_000_0000000,
        },
        Request {
            request_type: RequestType::Borrow as u32,
            address: xlm.clone(),
            amount: 5_000_0000000,
        },
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: usdc.clone(),
            amount: 5_000_0000000,
        },
        Request {
            request_type: RequestType::Borrow as u32,
            address: usdc.clone(),
            amount: 3_000_0000000,
        },
    ];
    pool_client.submit(&creator, &creator, &creator, &requests);

    pool_id
}
