#![cfg(test)]
use pool::PoolClient;
use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    vec, Address, BytesN, String, Vec,
};
use test_suites::{
    create_fixture_with_data,
    test_fixture::{TokenIndex, SCALAR_7},
};

/// Test backstop RZ changes correctly handle emissions tracking
#[test]
fn test_backstop_rz_changes_handle_emissions() {
    let fixture = create_fixture_with_data(false);
    let bstop_token = &fixture.lp;
    let sam = Address::generate(&fixture.env);
    let frodo = &fixture.users[0];
    let pool_fixture = &fixture.pools[0];

    // Mint some backstop tokens
    // assumes Sam makes up 20% of the backstop after depositing (50k / 0.8 * 0.2 = 12.5k)
    //  -> mint 12.5k LP tokens to sam
    fixture.tokens[TokenIndex::BLND].mint(&sam, &(125_001_000_0000_0000_000_000 * SCALAR_7)); // 10 BLND per LP token
    fixture.tokens[TokenIndex::BLND].approve(&sam, &bstop_token.address, &i128::MAX, &99999);
    fixture.tokens[TokenIndex::USDC].mint(&sam, &(3_126_000_0000_0000_000_000 * SCALAR_7)); // 0.25 USDC per LP token
    fixture.tokens[TokenIndex::USDC].approve(&sam, &bstop_token.address, &i128::MAX, &99999);
    bstop_token.join_pool(
        &(12_500 * SCALAR_7),
        &vec![
            &fixture.env,
            125_001_000_0000_0000_000 * SCALAR_7,
            3_126_000_0000_0000_000 * SCALAR_7,
        ],
        &sam,
    );
    fixture
        .backstop
        .deposit(&sam, &pool_fixture.pool.address, &(12500 * SCALAR_7));
    fixture
        .backstop
        .queue_withdrawal(frodo, &pool_fixture.pool.address, &(45000 * SCALAR_7));

    fixture.jump(60 * 60 * 24 * 21);
    fixture.emitter.distribute();
    fixture.backstop.distribute();
    pool_fixture.pool.gulp_emissions();
    fixture
        .backstop
        .withdraw(frodo, &pool_fixture.pool.address, &(45000 * SCALAR_7));

    fixture.backstop.remove_reward(&pool_fixture.pool.address);

    let result = pool_fixture.pool.try_gulp_emissions();
    assert!(result.is_err());

    // claim 3 days later
    fixture.jump(60 * 60 * 24 * 3);
    let result = fixture.backstop.claim(
        &sam,
        &vec![&fixture.env, pool_fixture.pool.address.clone()],
        &0,
    );
    assert_eq!(result, 55_141_3083663);

    fixture.jump(60 * 60 * 24 * 4);
    let result = fixture.backstop.claim(
        &sam,
        &vec![&fixture.env, pool_fixture.pool.address.clone()],
        &0,
    );
    assert_eq!(result, 54_030_2461020);

    fixture.jump(1);
    let result = fixture.backstop.claim(
        &sam,
        &vec![&fixture.env, pool_fixture.pool.address.clone()],
        &0,
    );
    assert_eq!(result, 0);

    fixture
        .backstop
        .deposit(frodo, &pool_fixture.pool.address, &(50000 * SCALAR_7));

    fixture
        .backstop
        .add_reward(&pool_fixture.pool.address, &None);

    fixture.emitter.distribute();
    fixture.backstop.distribute();

    let result = pool_fixture.pool.gulp_emissions();

    // Emissions are distributed to the pool because the reward zone was empty when the backstop was added
    assert_eq!(result, 1814403000000); // (60 * 60 * 24 * 7 + 1) * 0.3
}

#[test]
fn test_backstop_full_rz_under_limits() {
    let fixture = create_fixture_with_data(true);
    let bstop_token = &fixture.lp;
    let sam = Address::generate(&fixture.env);
    let pool_fixture = &fixture.pools[0];

    // Mint some backstop tokens
    let per_pool_lp_deposit = 30_000 * SCALAR_7;
    fixture.tokens[TokenIndex::BLND].mint(&sam, &(125_001_000_0000_0000_000_000 * SCALAR_7)); // 10 BLND per LP token
    fixture.tokens[TokenIndex::BLND].approve(&sam, &bstop_token.address, &i128::MAX, &99999);
    fixture.tokens[TokenIndex::USDC].mint(&sam, &(3_126_000_0000_0000_000_000 * SCALAR_7)); // 0.25 USDC per LP token
    fixture.tokens[TokenIndex::USDC].approve(&sam, &bstop_token.address, &i128::MAX, &99999);
    bstop_token.join_pool(
        &(per_pool_lp_deposit * 60),
        &vec![
            &fixture.env,
            125_001_000_0000_0000_000 * SCALAR_7,
            3_126_000_0000_0000_000 * SCALAR_7,
        ],
        &sam,
    );

    // 1 Pool already in rz. Create 29 new pools.
    // They don't need reserves as we're not going to use them.
    fixture.emitter.distribute();
    fixture.backstop.distribute();
    let mut pools: Vec<Address> = vec![&fixture.env, pool_fixture.pool.address.clone()];
    for _ in 0..29 {
        let pool_address = fixture.pool_factory.deploy(
            &sam,
            &String::from_str(&fixture.env, "Teapot"),
            &BytesN::<32>::random(&fixture.env),
            &fixture.oracle.address,
            &0,
            &6,
            &0,
        );
        fixture
            .backstop
            .deposit(&sam, &pool_address, &per_pool_lp_deposit);
        fixture.backstop.add_reward(&pool_address, &None);
        pools.push_back(pool_address);
    }

    // check rz length
    let rz = fixture.backstop.reward_zone();
    assert_eq!(rz.len(), 30);

    // Run distribute w/ 30 pools
    fixture.jump_with_sequence(60 * 60 * 24 * 5);
    fixture.emitter.distribute();
    fixture.backstop.distribute();
    let dist_resources = fixture.env.cost_estimate().resources();
    assert!(dist_resources.instructions < 100000000);
    assert!(dist_resources.mem_bytes < 41943040 / 2);
    assert!(dist_resources.read_entries + dist_resources.write_entries < 100);
    assert!(dist_resources.write_entries < 50);
    assert!(dist_resources.read_bytes < 200000 / 2);
    assert!(dist_resources.write_bytes < 132096 / 2);

    // assert all pools can be gulped
    for pool in pools.iter() {
        let pool_client = PoolClient::new(&fixture.env, &pool);
        let result = pool_client.gulp_emissions();
        assert!(result > 0);
    }
}
