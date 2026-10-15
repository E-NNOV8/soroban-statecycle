#![cfg(test)]
use backstop::{BackstopDataKey, PoolBalance};
use cast::i128;
use pool::{AuctionData, FlashLoan, PoolDataKey, Request, RequestType, ReserveConfig};
use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::{
    map,
    testutils::{Address as AddressTestTrait, Events},
    vec, Address, Env, Error, FromVal, IntoVal, Symbol, TryFromVal, Val, Vec,
};
use test_suites::{
    assertions::{assert_approx_eq_abs, assert_approx_eq_rel},
    create_fixture_with_data,
    moderc3156::create_flashloan_receiver,
    test_fixture::{TokenIndex, SCALAR_7},
};

fn assert_fill_auction_event_no_data(
    env: &Env,
    event: (Address, Vec<Val>, Val),
    pool_address: &Address,
    auction_user: &Address,
    auction_type: u32,
    filler: &Address,
    fill_pct: i128,
) {
    let (event_pool_address, topics, data) = event;
    assert_eq!(event_pool_address, pool_address.clone());

    assert_eq!(topics.len(), 3);
    assert_eq!(
        Symbol::from_val(env, &topics.get_unchecked(0)),
        Symbol::new(env, "fill_auction")
    );
    assert_eq!(u32::from_val(env, &topics.get_unchecked(1)), auction_type);
    assert_eq!(
        Address::from_val(env, &topics.get_unchecked(2)),
        auction_user.clone()
    );

    let event_data = Vec::<Val>::from_val(env, &data);
    assert_eq!(event_data.len(), 3);
    assert_eq!(
        Address::from_val(env, &event_data.get_unchecked(0)),
        filler.clone()
    );
    assert_eq!(i128::from_val(env, &event_data.get_unchecked(1)), fill_pct);
    assert!(AuctionData::try_from_val(env, &event_data.get_unchecked(2)).is_ok());
}

#[test]
fn test_liquidations() {
    let fixture = create_fixture_with_data(false);
    let frodo = fixture.users.get(0).unwrap();
    let pool_fixture = &fixture.pools[0];

    // accrue interest
    let requests: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::Borrow as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: 10,
        },
        Request {
            request_type: RequestType::Repay as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: 10,
        },
        Request {
            request_type: RequestType::Borrow as u32,
            address: fixture.tokens[TokenIndex::XLM].address.clone(),
            amount: 10,
        },
        Request {
            request_type: RequestType::Repay as u32,
            address: fixture.tokens[TokenIndex::XLM].address.clone(),
            amount: 10,
        },
        Request {
            request_type: RequestType::Borrow as u32,
            address: fixture.tokens[TokenIndex::WETH].address.clone(),
            amount: 10,
        },
        Request {
            request_type: RequestType::Repay as u32,
            address: fixture.tokens[TokenIndex::WETH].address.clone(),
            amount: 10,
        },
    ];
    pool_fixture.pool.submit(&frodo, &frodo, &frodo, &requests);

    // Disable rate modifiers
    let mut usdc_config: ReserveConfig = fixture.read_reserve_config(0, TokenIndex::STABLE);
    usdc_config.reactivity = 0;

    let mut xlm_config: ReserveConfig = fixture.read_reserve_config(0, TokenIndex::XLM);
    xlm_config.reactivity = 0;
    let mut weth_config: ReserveConfig = fixture.read_reserve_config(0, TokenIndex::WETH);
    weth_config.reactivity = 0;

    fixture.env.as_contract(&fixture.pools[0].pool.address, || {
        let key = PoolDataKey::ResConfig(fixture.tokens[TokenIndex::STABLE].address.clone());
        fixture
            .env
            .storage()
            .persistent()
            .set::<PoolDataKey, ReserveConfig>(&key, &usdc_config);
        let key = PoolDataKey::ResConfig(fixture.tokens[TokenIndex::XLM].address.clone());
        fixture
            .env
            .storage()
            .persistent()
            .set::<PoolDataKey, ReserveConfig>(&key, &xlm_config);
        let key = PoolDataKey::ResConfig(fixture.tokens[TokenIndex::WETH].address.clone());
        fixture
            .env
            .storage()
            .persistent()
            .set::<PoolDataKey, ReserveConfig>(&key, &weth_config);
    });

    // have Frodo Q4W some backstop deposits
    let frodo_pre_q4w_amount = 10_000 * SCALAR_7;
    fixture
        .backstop
        .queue_withdrawal(&frodo, &pool_fixture.pool.address, &frodo_pre_q4w_amount);

    // Create a user
    let samwise = Address::generate(&fixture.env); //sam will be supplying XLM and borrowing STABLE

    // Mint users tokens
    fixture.tokens[TokenIndex::XLM].mint(&samwise, &(500_000 * SCALAR_7));
    fixture.tokens[TokenIndex::WETH].mint(&samwise, &(50 * 10i128.pow(9)));
    fixture.tokens[TokenIndex::USDC].mint(&frodo, &(100_000 * SCALAR_7));

    let frodo_requests: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: 30_000 * 10i128.pow(6),
        },
    ];
    // Supply frodo tokens
    pool_fixture
        .pool
        .submit(&frodo, &frodo, &frodo, &frodo_requests);
    // Supply and borrow sam tokens
    let sam_requests: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: fixture.tokens[TokenIndex::XLM].address.clone(),
            amount: 160_000 * SCALAR_7,
        },
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: fixture.tokens[TokenIndex::WETH].address.clone(),
            amount: 17 * 10i128.pow(9),
        },
        // Sam's max borrow is 39_200 STABLE
        Request {
            request_type: RequestType::Borrow as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: 28_000 * 10i128.pow(6),
        }, // reduces Sam's max borrow to 14_526.31579 STABLE
        Request {
            request_type: RequestType::Borrow as u32,
            address: fixture.tokens[TokenIndex::XLM].address.clone(),
            amount: 65_000 * SCALAR_7,
        },
    ];
    let sam_positions = pool_fixture
        .pool
        .submit(&samwise, &samwise, &samwise, &sam_requests);

    //Utilization is now:
    // * 36_000 / 40_000 = .9 for STABLE
    // * 130_000 / 260_000 = .5 for XLM
    // This equates to the following rough annual interest rates
    //  * 31% for STABLE borrowing
    //  * 25.11% for STABLE lending
    //  * rate will be dragged up to rate modifier
    //  * 6% for XLM borrowing
    //  * 2.7% for XLM lending

    // Let three months go by and call update every week
    for _ in 0..12 {
        // Let one week pass
        fixture.jump(60 * 60 * 24 * 7);
        // Update emissions
        fixture.emitter.distribute();
        fixture.backstop.distribute();
        pool_fixture.pool.gulp_emissions();
    }
    // Start an interest auction
    // type 2 is an interest auction
    let auction_data = pool_fixture.pool.new_auction(
        &2u32,
        &fixture.backstop.address,
        &vec![&fixture.env, fixture.lp.address.clone()],
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::STABLE].address.clone(),
            fixture.tokens[TokenIndex::WETH].address.clone(),
            fixture.tokens[TokenIndex::XLM].address.clone(),
        ],
        &100u32,
    );

    let stable_interest_lot_amount = auction_data
        .lot
        .get_unchecked(fixture.tokens[TokenIndex::STABLE].address.clone());
    assert_approx_eq_abs(stable_interest_lot_amount, 256_746831, 5000000);
    let xlm_interest_lot_amount = auction_data
        .lot
        .get_unchecked(fixture.tokens[TokenIndex::XLM].address.clone());
    assert_approx_eq_abs(xlm_interest_lot_amount, 179_5067018, 5000000);
    let weth_interest_lot_amount = auction_data
        .lot
        .get_unchecked(fixture.tokens[TokenIndex::WETH].address.clone());
    assert_approx_eq_abs(weth_interest_lot_amount, 0_002671545, 5000);
    let lp_donate_bid_amount = auction_data.bid.get_unchecked(fixture.lp.address.clone());
    //NOTE: bid STABLE amount is seven decimals whereas reserve(and lot) STABLE has 6 decomals
    assert_approx_eq_abs(lp_donate_bid_amount, 268_9213686, SCALAR_7);
    assert_eq!(auction_data.block, 151);
    let liq_pct = 30;
    let events = fixture.env.events().all();
    let event = vec![&fixture.env, events.get_unchecked(events.len() - 1)];
    assert_eq!(
        event,
        vec![
            &fixture.env,
            (
                pool_fixture.pool.address.clone(),
                (
                    Symbol::new(&fixture.env, "new_auction"),
                    2u32,
                    fixture.backstop.address.clone(),
                )
                    .into_val(&fixture.env),
                (100u32, auction_data.clone()).into_val(&fixture.env) // event_data.into_val(&fixture.env)
            )
        ]
    );
    // Start a liquidation auction
    let auction_data = pool_fixture.pool.new_auction(
        &0,
        &samwise,
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::STABLE].address.clone(),
            fixture.tokens[TokenIndex::XLM].address.clone(),
        ],
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::WETH].address.clone(),
            fixture.tokens[TokenIndex::XLM].address.clone(),
        ],
        &liq_pct,
    );
    let usdc_bid_amount = auction_data
        .bid
        .get_unchecked(fixture.tokens[TokenIndex::STABLE].address.clone());
    assert_approx_eq_abs(
        usdc_bid_amount,
        sam_positions
            .liabilities
            .get(0)
            .unwrap()
            .fixed_mul_ceil(i128(liq_pct * 100000), SCALAR_7)
            .unwrap(),
        SCALAR_7,
    );
    let xlm_bid_amount = auction_data
        .bid
        .get_unchecked(fixture.tokens[TokenIndex::XLM].address.clone());
    assert_approx_eq_abs(
        xlm_bid_amount,
        sam_positions
            .liabilities
            .get(1)
            .unwrap()
            .fixed_mul_ceil(i128(liq_pct * 100000), SCALAR_7)
            .unwrap(),
        SCALAR_7,
    );
    let xlm_lot_amount = auction_data
        .lot
        .get_unchecked(fixture.tokens[TokenIndex::XLM].address.clone());
    assert_approx_eq_abs(xlm_lot_amount, 40100_6654560, SCALAR_7);
    let weth_lot_amount = auction_data
        .lot
        .get_unchecked(fixture.tokens[TokenIndex::WETH].address.clone());
    assert_approx_eq_abs(weth_lot_amount, 4_260750195, 1000);
    let events = fixture.env.events().all();
    let event = vec![&fixture.env, events.get_unchecked(events.len() - 1)];
    assert_eq!(
        event,
        vec![
            &fixture.env,
            (
                pool_fixture.pool.address.clone(),
                (
                    Symbol::new(&fixture.env, "new_auction"),
                    0 as u32,
                    samwise.clone(),
                )
                    .into_val(&fixture.env),
                (liq_pct, auction_data.clone()).into_val(&fixture.env)
            )
        ]
    );

    //let 100 blocks pass to scale up the modifier
    fixture.jump_with_sequence(101 * 5);
    //fill user and interest liquidation
    let auct_type_1: u32 = 0;
    let auct_type_2: u32 = 2;
    let fill_requests = vec![
        &fixture.env,
        Request {
            request_type: RequestType::FillUserLiquidationAuction as u32,
            address: samwise.clone(),
            amount: 25,
        },
        Request {
            request_type: RequestType::FillUserLiquidationAuction as u32,
            address: samwise.clone(),
            amount: 100,
        },
        Request {
            request_type: RequestType::FillInterestAuction as u32,
            address: fixture.backstop.address.clone(), //address shouldn't matter
            amount: 99,
        },
        Request {
            request_type: RequestType::FillInterestAuction as u32,
            address: fixture.backstop.address.clone(), //address shouldn't matter
            amount: 100,
        },
        Request {
            request_type: RequestType::Repay as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: usdc_bid_amount,
        },
    ];
    let frodo_stable_balance = fixture.tokens[TokenIndex::STABLE].balance(&frodo);
    let frodo_xlm_balance = fixture.tokens[TokenIndex::XLM].balance(&frodo);
    let frodo_weth_balance = fixture.tokens[TokenIndex::WETH].balance(&frodo);
    fixture.lp.approve(
        &frodo,
        &fixture.backstop.address,
        &lp_donate_bid_amount,
        &fixture.env.ledger().sequence(),
    );
    let frodo_positions_post_fill =
        pool_fixture
            .pool
            .submit(&frodo, &frodo, &frodo, &fill_requests);
    assert_approx_eq_abs(
        frodo_positions_post_fill.collateral.get_unchecked(2),
        weth_lot_amount
            .fixed_div_floor(2_0000000, SCALAR_7)
            .unwrap()
            + 10 * 10i128.pow(9),
        1000,
    );
    assert_approx_eq_abs(
        frodo_positions_post_fill.collateral.get_unchecked(1),
        xlm_lot_amount.fixed_div_floor(2_0000000, SCALAR_7).unwrap() + 100_000 * SCALAR_7,
        1000,
    );
    assert_approx_eq_abs(
        frodo_positions_post_fill.liabilities.get_unchecked(1),
        xlm_bid_amount + 65_000 * SCALAR_7,
        1000,
    );
    assert_approx_eq_abs(
        frodo_positions_post_fill.liabilities.get_unchecked(0),
        8_000 * 10i128.pow(6) + 559_285757,
        100000,
    );
    let events = fixture.env.events().all();
    assert_fill_auction_event_no_data(
        &fixture.env,
        events.get_unchecked(events.len() - 16),
        &pool_fixture.pool.address,
        &samwise,
        auct_type_1,
        &frodo,
        25,
    );
    assert_fill_auction_event_no_data(
        &fixture.env,
        events.get_unchecked(events.len() - 15),
        &pool_fixture.pool.address,
        &samwise,
        auct_type_1,
        &frodo,
        100,
    );
    assert_fill_auction_event_no_data(
        &fixture.env,
        events.get_unchecked(events.len() - 9),
        &pool_fixture.pool.address,
        &fixture.backstop.address,
        auct_type_2,
        &frodo,
        99,
    );
    assert_fill_auction_event_no_data(
        &fixture.env,
        events.get_unchecked(events.len() - 3),
        &pool_fixture.pool.address,
        &fixture.backstop.address,
        auct_type_2,
        &frodo,
        100,
    );
    assert_approx_eq_abs(
        fixture.tokens[TokenIndex::STABLE].balance(&frodo),
        frodo_stable_balance - usdc_bid_amount
            + stable_interest_lot_amount
                .fixed_div_floor(2 * 10i128.pow(6), 10i128.pow(6))
                .unwrap(),
        10i128.pow(6),
    );
    assert_approx_eq_abs(
        fixture.tokens[TokenIndex::XLM].balance(&frodo),
        frodo_xlm_balance
            + xlm_interest_lot_amount
                .fixed_div_floor(2 * SCALAR_7, SCALAR_7)
                .unwrap(),
        SCALAR_7,
    );
    assert_approx_eq_abs(
        fixture.tokens[TokenIndex::WETH].balance(&frodo),
        frodo_weth_balance
            + weth_interest_lot_amount
                .fixed_div_floor(2 * 10i128.pow(9), 10i128.pow(9))
                .unwrap(),
        10i128.pow(9),
    );

    //tank eth price
    fixture.oracle.set_price_stable(&vec![
        &fixture.env,
        500_0000000, // eth
        1_0000000,   // usdc
        0_1000000,   // xlm
        1_0000000,   // stable
    ]);

    //fully liquidate user
    let liq_pct = 100;
    let auction_data_2 = pool_fixture.pool.new_auction(
        &0,
        &samwise,
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::STABLE].address.clone(),
            fixture.tokens[TokenIndex::XLM].address.clone(),
        ],
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::WETH].address.clone(),
            fixture.tokens[TokenIndex::XLM].address.clone(),
        ],
        &liq_pct,
    );

    let stable_bid_amount = auction_data_2
        .bid
        .get_unchecked(fixture.tokens[TokenIndex::STABLE].address.clone());
    assert_approx_eq_abs(stable_bid_amount, 19599_872330, 100000);
    let xlm_bid_amount = auction_data_2
        .bid
        .get_unchecked(fixture.tokens[TokenIndex::XLM].address.clone());
    assert_approx_eq_abs(xlm_bid_amount, 45498_8226700, SCALAR_7);
    let xlm_lot_amount = auction_data_2
        .lot
        .get_unchecked(fixture.tokens[TokenIndex::XLM].address.clone());
    assert_approx_eq_abs(xlm_lot_amount, 139947_2453890, SCALAR_7);
    let weth_lot_amount = auction_data_2
        .lot
        .get_unchecked(fixture.tokens[TokenIndex::WETH].address.clone());
    assert_approx_eq_abs(weth_lot_amount, 14_869584990, 100000000);

    //allow 250 blocks to pass
    fixture.jump_with_sequence(251 * 5);

    // fill user liquidation for samwise. Creates bad debt that is passed off to the
    // backstop
    let samwise_pre_full_liq = pool_fixture.pool.get_positions(&samwise);
    let frodo_stable_balance = fixture.tokens[TokenIndex::STABLE].balance(&frodo);
    let frodo_xlm_balance = fixture.tokens[TokenIndex::XLM].balance(&frodo);
    let fill_requests = vec![
        &fixture.env,
        Request {
            request_type: RequestType::FillUserLiquidationAuction as u32,
            address: samwise.clone(),
            amount: 100,
        },
        Request {
            request_type: RequestType::Repay as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: stable_bid_amount
                .fixed_div_floor(2_0000000, SCALAR_7)
                .unwrap(),
        },
        Request {
            request_type: RequestType::Repay as u32,
            address: fixture.tokens[TokenIndex::XLM].address.clone(),
            amount: xlm_bid_amount.fixed_div_floor(2_0000000, SCALAR_7).unwrap(),
        },
    ];
    let stable_filled = stable_bid_amount
        .fixed_mul_floor(3_0000000, SCALAR_7)
        .unwrap()
        .fixed_div_floor(4_0000000, SCALAR_7)
        .unwrap();
    let xlm_filled = xlm_bid_amount
        .fixed_mul_floor(3_0000000, SCALAR_7)
        .unwrap()
        .fixed_div_floor(4_0000000, SCALAR_7)
        .unwrap();
    let new_frodo_positions = pool_fixture
        .pool
        .submit(&frodo, &frodo, &frodo, &fill_requests);
    assert_approx_eq_abs(
        frodo_positions_post_fill.collateral.get(1).unwrap() + xlm_lot_amount,
        new_frodo_positions.collateral.get(1).unwrap(),
        SCALAR_7,
    );
    assert_approx_eq_abs(
        frodo_positions_post_fill.collateral.get(2).unwrap() + weth_lot_amount,
        new_frodo_positions.collateral.get(2).unwrap(),
        SCALAR_7,
    );
    assert_approx_eq_abs(
        frodo_positions_post_fill.liabilities.get(0).unwrap() + stable_filled - 9147_499950,
        new_frodo_positions.liabilities.get(0).unwrap(),
        10i128.pow(6),
    );
    assert_approx_eq_abs(
        frodo_positions_post_fill.liabilities.get(1).unwrap() + xlm_filled - 22438_6298700,
        new_frodo_positions.liabilities.get(1).unwrap(),
        SCALAR_7,
    );
    assert_approx_eq_abs(
        frodo_stable_balance - 9799_936164,
        fixture.tokens[TokenIndex::STABLE].balance(&frodo),
        10i128.pow(6),
    );
    assert_approx_eq_abs(
        frodo_xlm_balance - 22749_4113400,
        fixture.tokens[TokenIndex::XLM].balance(&frodo),
        SCALAR_7,
    );

    // check bad debt was transferred to backstop
    let samwise_positions_post_bd = pool_fixture.pool.get_positions(&samwise);
    assert_eq!(samwise_positions_post_bd.liabilities.len(), 0);
    assert_eq!(samwise_positions_post_bd.collateral.len(), 0);
    let backstop_positions = pool_fixture.pool.get_positions(&fixture.backstop.address);
    // bid scaled to 75%, so 25% is bad debt
    let stable_bad_debt = samwise_pre_full_liq
        .liabilities
        .get(0)
        .unwrap()
        .fixed_mul_floor(0_2500000, SCALAR_7)
        .unwrap();
    let xlm_bad_debt = samwise_pre_full_liq
        .liabilities
        .get(1)
        .unwrap()
        .fixed_mul_floor(0_2500000, SCALAR_7)
        .unwrap();
    assert_eq!(
        stable_bad_debt,
        backstop_positions.liabilities.get(0).unwrap()
    );
    assert_eq!(xlm_bad_debt, backstop_positions.liabilities.get(1).unwrap());

    // validate that frodo cannot withdraw backstop deposits if bad debt exists
    let withdraw_result =
        fixture
            .backstop
            .try_withdraw(&frodo, &pool_fixture.pool.address, &frodo_pre_q4w_amount);
    assert_eq!(
        withdraw_result.err(),
        Some(Ok(Error::from_contract_error(1011)))
    );

    // create a bad debt auction
    let auction_type: u32 = 1;
    let bad_debt_auction_data = pool_fixture.pool.new_auction(
        &1u32,
        &fixture.backstop.address,
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::STABLE].address.clone(),
            fixture.tokens[TokenIndex::XLM].address.clone(),
        ],
        &vec![&fixture.env, fixture.lp.address.clone()],
        &100u32,
    );

    assert_eq!(bad_debt_auction_data.bid.len(), 2);
    assert_eq!(bad_debt_auction_data.lot.len(), 1);

    assert_eq!(
        bad_debt_auction_data
            .bid
            .get_unchecked(fixture.tokens[TokenIndex::STABLE].address.clone()),
        stable_bad_debt //d rate 1.071330239
    );
    assert_eq!(
        bad_debt_auction_data
            .bid
            .get_unchecked(fixture.tokens[TokenIndex::XLM].address.clone()),
        xlm_bad_debt //d rate 1.013853805
    );
    assert_approx_eq_abs(
        bad_debt_auction_data
            .lot
            .get_unchecked(fixture.lp.address.clone()),
        6146_6087407, // lp_token value is $1.25 each
        SCALAR_7,
    );
    let events = fixture.env.events().all();
    let event = vec![&fixture.env, events.get_unchecked(events.len() - 1)];
    assert_eq!(
        event,
        vec![
            &fixture.env,
            (
                pool_fixture.pool.address.clone(),
                (
                    Symbol::new(&fixture.env, "new_auction"),
                    auction_type,
                    fixture.backstop.address.clone(),
                )
                    .into_val(&fixture.env),
                (100u32, bad_debt_auction_data.clone()).into_val(&fixture.env)
            )
        ]
    );

    // allow 100 blocks to pass
    fixture.jump_with_sequence(101 * 5);
    // fill bad debt auction
    let frodo_bstop_pre_fill = fixture.lp.balance(&frodo);
    let backstop_bstop_pre_fill = fixture.lp.balance(&fixture.backstop.address);
    let auction_type: u32 = 1;
    let bad_debt_fill_request = vec![
        &fixture.env,
        Request {
            request_type: RequestType::FillBadDebtAuction as u32,
            address: fixture.backstop.address.clone(),
            amount: 20,
        },
    ];
    let post_bd_fill_frodo_positions =
        pool_fixture
            .pool
            .submit(&frodo, &frodo, &frodo, &bad_debt_fill_request);

    assert_eq!(
        post_bd_fill_frodo_positions.liabilities.get(0).unwrap(),
        new_frodo_positions.liabilities.get(0).unwrap()
            + stable_bad_debt.fixed_mul_ceil(20, 100).unwrap(),
    );
    assert_eq!(
        post_bd_fill_frodo_positions.liabilities.get(1).unwrap(),
        new_frodo_positions.liabilities.get(1).unwrap()
            + xlm_bad_debt.fixed_mul_ceil(20, 100).unwrap(),
    );
    let events = fixture.env.events().all();
    assert_fill_auction_event_no_data(
        &fixture.env,
        events.get_unchecked(events.len() - 1),
        &pool_fixture.pool.address,
        &fixture.backstop.address,
        auction_type,
        &frodo,
        20,
    );
    assert_approx_eq_abs(
        fixture.lp.balance(&frodo),
        frodo_bstop_pre_fill + 614_6608740,
        SCALAR_7,
    );
    assert_approx_eq_abs(
        fixture.lp.balance(&fixture.backstop.address),
        backstop_bstop_pre_fill - 614_6608740,
        SCALAR_7,
    );
    let new_auction = pool_fixture
        .pool
        .get_auction(&(1 as u32), &fixture.backstop.address);
    assert_eq!(new_auction.bid.len(), 2);
    assert_eq!(new_auction.lot.len(), 1);
    assert_eq!(
        new_auction
            .bid
            .get_unchecked(fixture.tokens[TokenIndex::STABLE].address.clone()),
        stable_bad_debt.fixed_mul_floor(80, 100).unwrap()
    );
    assert_eq!(
        new_auction
            .bid
            .get_unchecked(fixture.tokens[TokenIndex::XLM].address.clone()),
        xlm_bad_debt.fixed_mul_floor(80, 100).unwrap()
    );
    assert_approx_eq_abs(
        new_auction.lot.get_unchecked(fixture.lp.address.clone()),
        bad_debt_auction_data
            .lot
            .get_unchecked(fixture.lp.address.clone())
            - 1229_3217480,
        SCALAR_7,
    );
    assert_eq!(new_auction.block, bad_debt_auction_data.block);

    // validate that frodo cannot withdraw backstop during bad debt auction
    let withdraw_result =
        fixture
            .backstop
            .try_withdraw(&frodo, &pool_fixture.pool.address, &frodo_pre_q4w_amount);
    assert_eq!(
        withdraw_result.err(),
        Some(Ok(Error::from_contract_error(1011)))
    );

    // allow another 50 blocks to pass (150 total)
    fixture.jump_with_sequence(50 * 5);
    // fill bad debt auction
    let frodo_bstop_pre_fill = fixture.lp.balance(&frodo);
    let backstop_bstop_pre_fill = fixture.lp.balance(&fixture.backstop.address);
    let auction_type: u32 = 1;
    let bad_debt_fill_request = vec![
        &fixture.env,
        Request {
            request_type: RequestType::FillBadDebtAuction as u32,
            address: fixture.backstop.address.clone(),
            amount: 100,
        },
    ];
    let post_bd_fill_frodo_positions =
        pool_fixture
            .pool
            .submit(&frodo, &frodo, &frodo, &bad_debt_fill_request);
    assert_eq!(
        post_bd_fill_frodo_positions.liabilities.get(0).unwrap(),
        new_frodo_positions.liabilities.get(0).unwrap() + stable_bad_debt,
    );
    assert_eq!(
        post_bd_fill_frodo_positions.liabilities.get(1).unwrap(),
        new_frodo_positions.liabilities.get(1).unwrap() + xlm_bad_debt,
    );
    let events = fixture.env.events().all();
    assert_fill_auction_event_no_data(
        &fixture.env,
        events.get_unchecked(events.len() - 1),
        &pool_fixture.pool.address,
        &fixture.backstop.address,
        auction_type,
        &frodo,
        100,
    );
    assert_approx_eq_abs(
        fixture.lp.balance(&frodo),
        frodo_bstop_pre_fill + 3687_9652440,
        SCALAR_7,
    );
    assert_approx_eq_abs(
        fixture.lp.balance(&fixture.backstop.address),
        backstop_bstop_pre_fill - 3687_9652440,
        SCALAR_7,
    );

    //check that frodo was correctly slashed for both q4w and newly withdrawn deposits
    let original_deposit = 50_000 * SCALAR_7;
    let original_deposit_remaining = original_deposit - frodo_pre_q4w_amount;
    let pre_withdraw_frodo_bstp = fixture.lp.balance(&frodo);
    // withdraw pre_q4w_amount
    fixture
        .backstop
        .withdraw(&frodo, &pool_fixture.pool.address, &frodo_pre_q4w_amount);
    fixture.backstop.queue_withdrawal(
        &frodo,
        &pool_fixture.pool.address,
        &original_deposit_remaining,
    );
    //jump a month
    fixture.jump(45 * 24 * 60 * 60);
    fixture.backstop.withdraw(
        &frodo,
        &pool_fixture.pool.address,
        &original_deposit_remaining,
    );
    assert_approx_eq_abs(
        fixture.lp.balance(&frodo) - pre_withdraw_frodo_bstp,
        original_deposit - 614_6608740 - 3687_9652440 + 268_9213686,
        SCALAR_7,
    );

    // Test bad debt is burned and defaulted correctly
    // Deposit barely over the minimum backstop threshold in tokens
    fixture
        .backstop
        .deposit(&frodo, &pool_fixture.pool.address, &1100_0000000);

    // Sam re-borrows
    let sam_requests: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: fixture.tokens[TokenIndex::WETH].address.clone(),
            amount: 1 * 10i128.pow(9),
        },
        // Sam's max borrow is 39_200 STABLE
        Request {
            request_type: RequestType::Borrow as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: 100 * 10i128.pow(6),
        }, // reduces Sam's max borrow to 14_526.31579 STABLE
    ];
    let sam_positions = pool_fixture
        .pool
        .submit(&samwise, &samwise, &samwise, &sam_requests);

    // Nuke eth price more
    fixture.oracle.set_price_stable(&vec![
        &fixture.env,
        10_0000000, // eth
        1_0000000,  // usdc
        0_1000000,  // xlm
        1_0000000,  // stable
    ]);

    // Liquidate sam
    let liq_pct: u32 = 100;
    let auction_data = pool_fixture.pool.new_auction(
        &0,
        &samwise,
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::STABLE].address.clone(),
        ],
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::WETH].address.clone(),
        ],
        &liq_pct,
    );
    let usdc_bid_amount = auction_data
        .bid
        .get_unchecked(fixture.tokens[TokenIndex::STABLE].address.clone());
    assert_approx_eq_abs(
        usdc_bid_amount,
        sam_positions
            .liabilities
            .get(0)
            .unwrap()
            .fixed_mul_ceil(i128(liq_pct * 100000), SCALAR_7)
            .unwrap(),
        SCALAR_7,
    );

    //jump 400 blocks
    fixture.jump_with_sequence(401 * 5);

    //fill liq
    let bad_debt_fill_request = vec![
        &fixture.env,
        Request {
            request_type: RequestType::FillUserLiquidationAuction as u32,
            address: samwise.clone(),
            amount: 100,
        },
    ];
    pool_fixture
        .pool
        .submit(&frodo, &frodo, &frodo, &bad_debt_fill_request);
    let events = fixture.env.events().all();
    // bad debt event occurs before the auction fill event
    let event = vec![&fixture.env, events.get_unchecked(events.len() - 2)];
    let bad_debt: i128 = 9_2903008;
    assert_eq!(
        event,
        vec![
            &fixture.env,
            (
                pool_fixture.pool.address.clone(),
                (
                    Symbol::new(&fixture.env, "bad_debt"),
                    samwise.clone(),
                    fixture.tokens[TokenIndex::STABLE].address.clone()
                )
                    .into_val(&fixture.env),
                bad_debt.into_val(&fixture.env)
            )
        ]
    );

    // Create bad debt auction
    let bad_deb_auction = pool_fixture.pool.new_auction(
        &1u32,
        &fixture.backstop.address,
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::STABLE].address.clone(),
        ],
        &vec![&fixture.env, fixture.lp.address.clone()],
        &100u32,
    );
    assert!(bad_deb_auction.bid.len() == 1);
    assert_eq!(
        bad_deb_auction
            .bid
            .get_unchecked(fixture.tokens[TokenIndex::STABLE].address.clone()),
        bad_debt
    );

    // Fill bad debt auction
    let frodo_positions = pool_fixture.pool.get_positions(&frodo);
    let bad_debt_fill_request = vec![
        &fixture.env,
        Request {
            request_type: RequestType::FillBadDebtAuction as u32,
            address: fixture.backstop.address.clone(),
            amount: 100,
        },
    ];

    // fill bad debt auction
    // pay of 25% of bad debt, allow other 75% to be defaulted
    fixture.jump_with_sequence(351 * 5);

    let stable_pre_bad_debt = pool_fixture
        .pool
        .get_reserve(&fixture.tokens[TokenIndex::STABLE].address);

    let post_bd_fill_frodo_positions =
        pool_fixture
            .pool
            .submit(&frodo, &frodo, &frodo, &bad_debt_fill_request);
    let defaulted_debt = bad_debt.fixed_mul_floor(75, 100).unwrap();
    let events = fixture.env.events().all();
    let event = vec![&fixture.env, events.get_unchecked(events.len() - 2)];
    assert_eq!(
        event,
        vec![
            &fixture.env,
            (
                pool_fixture.pool.address.clone(),
                (
                    Symbol::new(&fixture.env, "defaulted_debt"),
                    fixture.tokens[TokenIndex::STABLE].address.clone()
                )
                    .into_val(&fixture.env),
                defaulted_debt.into_val(&fixture.env)
            )
        ]
    );
    assert_eq!(
        frodo_positions.liabilities.get_unchecked(0) + (bad_debt - defaulted_debt),
        post_bd_fill_frodo_positions.liabilities.get_unchecked(0)
    );
    let bad_debt_positions = pool_fixture.pool.get_positions(&fixture.backstop.address);
    assert_eq!(bad_debt_positions.liabilities.len(), 0);
    let stable_post_bad_debt = pool_fixture
        .pool
        .get_reserve(&fixture.tokens[TokenIndex::STABLE].address);
    assert_eq!(
        stable_post_bad_debt.data.d_supply,
        stable_pre_bad_debt.data.d_supply - defaulted_debt
    );
    assert_approx_eq_abs(
        stable_pre_bad_debt.total_supply(&fixture.env)
            - stable_post_bad_debt.total_supply(&fixture.env),
        stable_post_bad_debt.to_asset_from_d_token(&fixture.env, defaulted_debt),
        0_0000100,
    );
}

#[test]
fn test_user_restore_position_and_delete_liquidation() {
    let fixture = create_fixture_with_data(false);
    let pool_fixture = &fixture.pools[0];
    let stable_pool_index = pool_fixture.reserves[&TokenIndex::STABLE];
    let xlm_pool_index = pool_fixture.reserves[&TokenIndex::XLM];

    // Create a standard flash loan receiver
    let (receiver_address, _receiver_client) = create_flashloan_receiver(&fixture.env);

    // Create a user that is supply STABLE (cf = 90%, $1) and borrowing XLM (lf = 75%, $0.10)
    let samwise = Address::generate(&fixture.env);
    fixture.tokens[TokenIndex::STABLE].mint(&samwise, &(1100 * 10i128.pow(6)));
    fixture.tokens[TokenIndex::XLM].mint(&samwise, &(10000 * SCALAR_7));

    // deposit $1k stable and borrow to 90% borrow limit ($810)
    let setup_request: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: 1000 * 10i128.pow(6),
        },
        Request {
            request_type: RequestType::Borrow as u32,
            address: fixture.tokens[TokenIndex::XLM].address.clone(),
            amount: 6075 * SCALAR_7,
        },
    ];
    pool_fixture
        .pool
        .submit(&samwise, &samwise, &samwise, &setup_request);

    // simulate 20% XLM price increase ($972 liabilities, $900 limit) and create user liquidation
    fixture.oracle.set_price_stable(&vec![
        &fixture.env,
        2000_0000000, // eth
        1_0000000,    // usdc
        0_1200000,    // xlm
        1_0000000,    // stable
    ]);
    pool_fixture.pool.new_auction(
        &0,
        &samwise,
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::XLM].address.clone(),
        ],
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::STABLE].address.clone(),
        ],
        &50,
    );
    assert!(pool_fixture.pool.try_get_auction(&0, &samwise).is_ok());

    // jump 200 blocks
    fixture.jump_with_sequence(200 * 5);

    // validate liquidation can't be deleted without restoring position
    let delete_only_request: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::DeleteLiquidationAuction as u32,
            address: Address::generate(&fixture.env),
            amount: i128::MAX,
        },
    ];
    let delete_only =
        pool_fixture
            .pool
            .try_submit(&samwise, &samwise, &samwise, &delete_only_request);
    assert_eq!(
        delete_only.err(),
        Some(Ok(Error::from_contract_error(1205)))
    );

    // validate health factor must be fully restored before deleting position
    let short_supply_delete_request: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: 79 * 10i128.pow(6), // need $80 more collateral
        },
        Request {
            request_type: RequestType::DeleteLiquidationAuction as u32,
            address: Address::generate(&fixture.env),
            amount: i128::MAX,
        },
    ];
    let short_supply_delete =
        pool_fixture
            .pool
            .try_submit(&samwise, &samwise, &samwise, &short_supply_delete_request);
    assert_eq!(
        short_supply_delete.err(),
        Some(Ok(Error::from_contract_error(1205)))
    );

    let short_repay_delete_request: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::DeleteLiquidationAuction as u32,
            address: Address::generate(&fixture.env),
            amount: i128::MAX,
        },
        Request {
            request_type: RequestType::Repay as u32,
            address: fixture.tokens[TokenIndex::XLM].address.clone(),
            amount: 449 * SCALAR_7, // need to repay 450 XLM
        },
    ];
    let short_repay_delete =
        pool_fixture
            .pool
            .try_submit(&samwise, &samwise, &samwise, &short_repay_delete_request);
    assert_eq!(
        short_repay_delete.err(),
        Some(Ok(Error::from_contract_error(1205)))
    );

    // validate positions can't be modified without deleting liquidation
    let healthy_no_delete_request: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::Repay as u32,
            address: fixture.tokens[TokenIndex::XLM].address.clone(),
            amount: 10000 * SCALAR_7,
        },
    ];
    let healthy_no_delete =
        pool_fixture
            .pool
            .try_submit(&samwise, &samwise, &samwise, &healthy_no_delete_request);
    assert_eq!(
        healthy_no_delete.err(),
        Some(Ok(Error::from_contract_error(1212)))
    );

    // validate flash loan endpoint also requires liquidation to be deleted
    let flash_loan = FlashLoan {
        contract: receiver_address.clone(),
        asset: fixture.tokens[TokenIndex::XLM].address.clone(),
        amount: 1 * SCALAR_7,
    };
    let flash_loan_no_delete =
        pool_fixture
            .pool
            .try_flash_loan(&samwise, &flash_loan, &healthy_no_delete_request);
    assert_eq!(
        flash_loan_no_delete.err(),
        Some(Ok(Error::from_contract_error(1212)))
    );

    // validate liquidation can be deleted after restoring position
    let delete_request: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: 41 * 10i128.pow(6),
        },
        Request {
            request_type: RequestType::DeleteLiquidationAuction as u32,
            address: Address::generate(&fixture.env),
            amount: i128::MAX,
        },
        Request {
            request_type: RequestType::Repay as u32,
            address: fixture.tokens[TokenIndex::XLM].address.clone(),
            amount: 226 * SCALAR_7,
        },
    ];
    let sam_positions = pool_fixture
        .pool
        .submit(&samwise, &samwise, &samwise, &delete_request);
    // fuzz assert wide to account for b and d rates (only verify actions occurred)
    assert_approx_eq_abs(
        sam_positions.collateral.get_unchecked(stable_pool_index),
        1041 * 10i128.pow(6),
        10000,
    );
    assert_approx_eq_abs(
        sam_positions.liabilities.get_unchecked(xlm_pool_index),
        5849 * SCALAR_7,
        SCALAR_7,
    );
    assert!(pool_fixture.pool.try_get_auction(&0, &samwise).is_err());
}

#[test]
fn test_stale_liquidation_deletion() {
    let fixture = create_fixture_with_data(false);
    let pool_fixture = &fixture.pools[0];

    // Create a user
    let samwise = Address::generate(&fixture.env);

    // have sam create a position to help interest accrue
    fixture.tokens[TokenIndex::STABLE].mint(&samwise, &(1_000_000 * 10i128.pow(6)));
    let setup_request: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: 1_000_000 * 10i128.pow(6),
        },
        Request {
            request_type: RequestType::Borrow as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: 850_000 * 10i128.pow(6),
        },
    ];
    pool_fixture
        .pool
        .submit(&samwise, &samwise, &samwise, &setup_request);

    fixture.jump(60 * 60 * 24 * 14);

    // Start an interest auction
    pool_fixture.pool.new_auction(
        &2u32,
        &fixture.backstop.address,
        &vec![&fixture.env, fixture.lp.address.clone()],
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::STABLE].address.clone(),
            fixture.tokens[TokenIndex::WETH].address.clone(),
            fixture.tokens[TokenIndex::XLM].address.clone(),
        ],
        &100u32,
    );

    // skip 500 blocks (499 past start of auction)
    fixture.jump_with_sequence(500 * 5);

    // validate the auction can't be deleted
    let early_delete = pool_fixture
        .pool
        .try_del_auction(&2u32, &fixture.backstop.address);
    assert_eq!(
        early_delete.err(),
        Some(Ok(Error::from_contract_error(1200)))
    );

    let auction = pool_fixture
        .pool
        .get_auction(&2u32, &fixture.backstop.address);
    assert_eq!(auction.bid.len(), 1);
    assert_eq!(auction.lot.len(), 3);

    // skip 1 more block
    fixture.jump_with_sequence(5);

    // delete the auction
    pool_fixture
        .pool
        .del_auction(&2u32, &fixture.backstop.address);
    assert!(fixture.env.auths().is_empty());
    let event = vec![&fixture.env, fixture.env.events().all().last_unchecked()];
    assert_eq!(
        event,
        vec![
            &fixture.env,
            (
                pool_fixture.pool.address.clone(),
                (
                    Symbol::new(&fixture.env, "delete_auction"),
                    2u32,
                    fixture.backstop.address.clone()
                )
                    .into_val(&fixture.env),
                ().into_val(&fixture.env)
            )
        ]
    );

    let auction = pool_fixture
        .pool
        .try_get_auction(&2u32, &fixture.backstop.address);
    assert!(auction.is_err());
}

#[test]
fn test_bad_debt() {
    let fixture = create_fixture_with_data(false);
    let pool_fixture = &fixture.pools[0];
    let stable_pool_index = pool_fixture.reserves[&TokenIndex::STABLE];
    let stable = &fixture.tokens[TokenIndex::STABLE];
    let xlm = &fixture.tokens[TokenIndex::XLM];
    let stable_scalar: i128 = 10i128.pow(stable.decimals());

    let sam = Address::generate(&fixture.env);
    let elrond = Address::generate(&fixture.env);

    // ***** Setup Elrond to be the liquidator *****
    let elrond_stable_balance = 500_000 * stable_scalar;
    stable.mint(&elrond, &elrond_stable_balance);

    // ***** Test bad debt can be invoked for user with no collateral *****
    let sam_stable_debt = 1_000 * stable_scalar;
    let sam_xlm_collateral = 15_000 * SCALAR_7;
    xlm.mint(&sam, &sam_xlm_collateral);
    let mut sam_positions = pool_fixture.pool.submit(
        &sam,
        &sam,
        &sam,
        &vec![
            &fixture.env,
            Request {
                request_type: RequestType::SupplyCollateral as u32,
                address: xlm.address.clone(),
                amount: sam_xlm_collateral,
            },
            Request {
                request_type: RequestType::Borrow as u32,
                address: stable.address.clone(),
                amount: sam_stable_debt,
            },
        ],
    );

    fixture.jump_with_sequence(100);

    // Validate bad debt can't clear a user's liabilities if they have collateral
    let bad_debt_result_1 = pool_fixture.pool.try_bad_debt(&sam);
    assert_eq!(
        bad_debt_result_1.err(),
        Some(Ok(Error::from_contract_error(1200)))
    );

    // use magic to delete Sam's collateral
    fixture.env.as_contract(&pool_fixture.pool.address, || {
        let key = PoolDataKey::Positions(sam.clone());
        sam_positions.collateral = map![&fixture.env];
        fixture.env.storage().persistent().set(&key, &sam_positions);
    });

    // Validate invalid liquidaiton can't be created with no bid
    let result_sam_liquidation = pool_fixture.pool.try_new_auction(
        &0,
        &sam,
        &vec![&fixture.env, stable.address.clone()],
        &vec![&fixture.env, xlm.address.clone()],
        &100,
    );
    assert!(result_sam_liquidation.is_err());

    // Use bad debt to clear the position
    pool_fixture.pool.bad_debt(&sam);

    let sam_position_post = pool_fixture.pool.get_positions(&sam);
    assert_eq!(sam_position_post.collateral.len(), 0);
    assert_eq!(sam_position_post.liabilities.len(), 0);
    let backstop_post_bd_1 = pool_fixture.pool.get_positions(&fixture.backstop.address);
    assert_eq!(backstop_post_bd_1.collateral.len(), 0);
    assert_eq!(backstop_post_bd_1.liabilities.len(), 1);
    let bad_debt_1 = backstop_post_bd_1
        .liabilities
        .get_unchecked(stable_pool_index);
    // d_rate is barely above 1
    assert_approx_eq_rel(bad_debt_1, sam_stable_debt, 0_001000);

    fixture.jump_with_sequence(100);

    // ***** Test bad debt can be invoked for backstop when under min threshold *****

    // Validate bad debt can't default the backstops liabilities while it's healthy
    let bad_debt_result_2 = pool_fixture.pool.try_bad_debt(&fixture.backstop.address);
    assert_eq!(
        bad_debt_result_2.err(),
        Some(Ok(Error::from_contract_error(1200)))
    );

    // use magic to remove the pool's backstop funds
    let cur_pool_data = fixture.backstop.pool_data(&pool_fixture.pool.address);
    fixture.env.as_contract(&fixture.backstop.address, || {
        let key = BackstopDataKey::PoolBalance(pool_fixture.pool.address.clone());
        let new_balance = PoolBalance {
            shares: cur_pool_data.shares,
            tokens: 0,
            q4w: 0,
        };
        fixture.env.storage().persistent().set(&key, &new_balance);
    });

    fixture.jump_with_sequence(100);

    // Validate invalid liquidaiton can't be created with no lot
    let result_bad_debt_auction = pool_fixture.pool.try_new_auction(
        &1,
        &fixture.backstop.address,
        &vec![&fixture.env, stable.address.clone()],
        &vec![&fixture.env, fixture.lp.address.clone()],
        &100,
    );
    assert!(result_bad_debt_auction.is_err());

    // Use bad debt to default the leftover liabilities
    let pre_default_stable = pool_fixture.pool.get_reserve(&stable.address);
    pool_fixture.pool.bad_debt(&fixture.backstop.address);
    let post_default_stable = pool_fixture.pool.get_reserve(&stable.address);

    assert_eq!(
        post_default_stable.data.d_supply,
        pre_default_stable.data.d_supply - bad_debt_1
    );
    assert_eq!(
        post_default_stable.data.b_supply,
        pre_default_stable.data.b_supply
    );
    assert_approx_eq_abs(
        pre_default_stable.total_supply(&fixture.env)
            - post_default_stable.total_supply(&fixture.env),
        post_default_stable.to_asset_from_d_token(&fixture.env, bad_debt_1),
        0_0000100,
    );
}
