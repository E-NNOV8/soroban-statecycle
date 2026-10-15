#![cfg(test)]

use pool::{PoolClient, Request, RequestType};
use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::{testutils::Address as _, vec, Address, Error, String};
use test_suites::{
    assertions::assert_approx_eq_abs,
    pool::default_reserve_metadata,
    test_fixture::{TestFixture, TokenIndex, SCALAR_7},
};

#[test]
fn test_backstop_inflation_attack() {
    let mut fixture = TestFixture::create(false);

    let whale = Address::generate(&fixture.env);
    let sauron = Address::generate(&fixture.env);
    let pippen = Address::generate(&fixture.env);

    // create pool with 1 new reserve
    fixture.create_pool(String::from_str(&fixture.env, "Teapot"), 0, 6, 0);

    let xlm_config = default_reserve_metadata();
    fixture.create_pool_reserve(0, TokenIndex::XLM, &xlm_config);
    let pool_address = fixture.pools[0].pool.address.clone();

    // setup backstop and update pool status
    fixture.tokens[TokenIndex::BLND].mint(&whale, &(5_001_000 * SCALAR_7));
    fixture.tokens[TokenIndex::USDC].mint(&whale, &(121_000 * SCALAR_7));
    fixture.lp.join_pool(
        &(400_000 * SCALAR_7),
        &vec![&fixture.env, 5_001_000 * SCALAR_7, 121_000 * SCALAR_7],
        &whale,
    );

    // execute inflation attack against pippen
    let starting_balance = 200_000 * SCALAR_7;
    fixture.lp.transfer(&whale, &sauron, &starting_balance);
    fixture.lp.transfer(&whale, &pippen, &starting_balance);

    // 1. Attacker deposits a small amount as the initial depositor
    let sauron_deposit_amount = 100;
    let sauron_shares = fixture
        .backstop
        .deposit(&sauron, &pool_address, &sauron_deposit_amount);

    // 2. Attacker tries to send a large amount to the backstop before the victim can perform a deposit
    let inflation_amount = 10_000 * SCALAR_7;
    fixture
        .lp
        .transfer(&sauron, &pool_address, &inflation_amount);

    // contract correctly mints share amounts regardless of the token balance
    let deposit_amount = 100;
    let pippen_shares = fixture
        .backstop
        .deposit(&pippen, &pool_address, &deposit_amount);
    assert_eq!(pippen_shares, 100);
    assert_eq!(sauron_shares, pippen_shares);

    // 2b. Attacker tries to donate a large amount to the backstop before the victim can perform a deposit
    //    #! NOTE - Contract will stop a random address from donating. This can ONLY come from the pool.
    //              However, authorizations are mocked during intergation tests, so this will succeed.
    fixture.lp.approve(
        &sauron,
        &fixture.backstop.address,
        &inflation_amount,
        &fixture.env.ledger().sequence(),
    );
    fixture
        .backstop
        .donate(&sauron, &pool_address, &inflation_amount);

    // contracts stop any zero share deposits
    let bad_deposit_result = fixture
        .backstop
        .try_deposit(&pippen, &pool_address, &deposit_amount);
    assert_eq!(
        bad_deposit_result.err(),
        Some(Ok(Error::from_contract_error(1005)))
    );
}

#[test]
fn test_backstop_interest_auction_inflation_attack() {
    let mut fixture = TestFixture::create(false);

    let whale = Address::generate(&fixture.env);
    let sauron = Address::generate(&fixture.env);
    let pippen = Address::generate(&fixture.env);

    // create pool with 1 new reserve
    fixture.create_pool(String::from_str(&fixture.env, "Teapot"), 0, 6, 0);

    let xlm_config = default_reserve_metadata();
    fixture.create_pool_reserve(0, TokenIndex::XLM, &xlm_config);
    let pool_address = fixture.pools[0].pool.address.clone();
    let pool_client = PoolClient::new(&fixture.env, &pool_address);
    pool_client.set_status(&3);

    // mint LP tokens, ~ 10 BLND and 0.25 USDC per share
    fixture.tokens[TokenIndex::BLND].mint(&whale, &(10_100_000 * SCALAR_7));
    fixture.tokens[TokenIndex::USDC].mint(&whale, &(251_000 * SCALAR_7));
    fixture.lp.join_pool(
        &(1_000_000 * SCALAR_7),
        &vec![&fixture.env, 10_100_000 * SCALAR_7, 251_000 * SCALAR_7],
        &whale,
    );

    // send tokens to sauron and pippen
    let starting_balance = 500_000 * SCALAR_7;
    fixture.lp.transfer(&whale, &sauron, &starting_balance);
    fixture.lp.transfer(&whale, &pippen, &starting_balance);
    let xlm_balance = 1_000_000 * SCALAR_7;
    fixture.tokens[TokenIndex::XLM].mint(&sauron, &xlm_balance);

    // 1. Attacker deposits a small amount as the initial depositor
    let sauron_deposit_amount = 100;
    fixture
        .backstop
        .deposit(&sauron, &pool_address, &sauron_deposit_amount);

    // 2. Attacker tries to force an interest auction to occur to inflate the backstop share value
    let inflation_amount = xlm_balance;
    fixture.tokens[TokenIndex::XLM].transfer(&sauron, &pool_address, &inflation_amount);

    // -> verify that gulp cannot be called until sufficient backstop deposits are present
    let gulp_result = pool_client.try_gulp(&fixture.tokens[TokenIndex::XLM].address);
    assert_eq!(
        gulp_result.err(),
        Some(Ok(Error::from_contract_error(1206)))
    );

    // 3. Attacker enables borrowing on the pool and fills interest auction to cause share inflation
    let remaining_to_threshold = 21_000 * SCALAR_7;
    fixture
        .backstop
        .deposit(&sauron, &pool_address, &remaining_to_threshold);
    pool_client.update_status();
    pool_client.gulp(&fixture.tokens[TokenIndex::XLM].address);

    // -> start and fill interest auction
    pool_client.new_auction(
        &2,
        &fixture.backstop.address,
        &vec![&fixture.env, fixture.lp.address.clone()],
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::XLM].address.clone(),
        ],
        &100,
    );
    fixture.jump_with_sequence(201 * 5);
    fixture.lp.approve(
        &sauron,
        &fixture.backstop.address,
        &inflation_amount,
        &fixture.env.ledger().sequence(),
    );
    let fill_requests = vec![
        &fixture.env,
        Request {
            request_type: RequestType::FillInterestAuction as u32,
            address: fixture.backstop.address.clone(),
            amount: 100,
        },
    ];
    pool_client.submit(&sauron, &sauron, &sauron, &fill_requests);

    // -> check new backstop share value
    let backstop_data = fixture.backstop.pool_data(&pool_address);
    let shares_to_tokens = backstop_data
        .tokens
        .fixed_div_floor(backstop_data.shares, SCALAR_7)
        .unwrap();

    // 4. Victim uses pool with inflated backstop share value
    let pippen_deposit_amount = SCALAR_7;
    let pippen_shares = fixture
        .backstop
        .deposit(&pippen, &pool_address, &pippen_deposit_amount);

    // -> verify the victim did not any meaningful amount of funds due to rounding
    let pippen_tokens = pippen_shares
        .fixed_mul_floor(shares_to_tokens, SCALAR_7)
        .unwrap();
    assert_approx_eq_abs(pippen_tokens, pippen_deposit_amount, 10);
}
