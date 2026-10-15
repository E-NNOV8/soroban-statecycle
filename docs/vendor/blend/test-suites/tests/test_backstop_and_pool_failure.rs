#![cfg(test)]
use pool::{Request, RequestType};
use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::{testutils::Address as AddressTestTrait, vec, Address};
use test_suites::{
    assertions::{assert_approx_eq_abs, assert_approx_eq_rel},
    create_fixture_with_data,
    test_fixture::{TokenIndex, SCALAR_7},
};

#[test]
fn test_backstop_and_pool_failure() {
    let fixture = create_fixture_with_data(false);
    let pool_fixture = &fixture.pools[0];
    let stable_pool_index = pool_fixture.reserves[&TokenIndex::STABLE];
    let xlm_pool_index = pool_fixture.reserves[&TokenIndex::XLM];
    let weth_pool_index = pool_fixture.reserves[&TokenIndex::WETH];
    let stable = &fixture.tokens[TokenIndex::STABLE];
    let xlm = &fixture.tokens[TokenIndex::XLM];
    let weth = &fixture.tokens[TokenIndex::WETH];
    let stable_scalar: i128 = 10i128.pow(stable.decimals());
    let weth_scalar: i128 = 10i128.pow(weth.decimals());

    let frodo = fixture.users[0].clone();
    let gandalf = Address::generate(&fixture.env);
    let sam = Address::generate(&fixture.env);
    let pippin = Address::generate(&fixture.env);
    let elrond = Address::generate(&fixture.env);

    /*
     * Backstop starts with 50,000 LP tokens, worth about
     * 12500 USDC and 500k BLND, or about 62,500 USDC total
     * at the setup LP weights.
     *
     * Frodo positions:
     * - STABLE 10,000 / 8,000
     * - XLM 100,000 / 65,000
     * - WETH 10 / 5
     *
     * - Setup gandalf to suppply 300,000 STABLE to the pool
     * - Setup sam to borrow 100,000 STABLE against XLM
     * - Setup pippin to borrow 100,000 STABLE against WETH
     *
     * - Crash the price of XLM and WETH
     *
     * - Liquidate Sam, have enough STABLE bad debt to wipe out the backstop
     * but no default
     *
     * - Liquidate Pippin, verify the STABLE bad debt is defaulted
     */

    // ***** Setup Gandalf with 300,000 STABLE supply *****
    let gandalf_stable_supply = 300_000 * stable_scalar;
    stable.mint(&gandalf, &gandalf_stable_supply);
    pool_fixture.pool.submit(
        &gandalf,
        &gandalf,
        &gandalf,
        &vec![
            &fixture.env,
            Request {
                request_type: RequestType::Supply as u32,
                address: stable.address.clone(),
                amount: gandalf_stable_supply,
            },
        ],
    );

    // ***** Setup Sam with 100,000 STABLE debt against XLM *****
    // STABLE = 0.95 LF
    // XLM = 0.1 STABLE, 0.75 CF ~= 0.07125 STABLE/XLM
    let sam_stable_debt = 100_000 * stable_scalar;
    let sam_xlm_collateral = 1_500_000 * SCALAR_7;
    xlm.mint(&sam, &sam_xlm_collateral);
    pool_fixture.pool.submit(
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

    // ***** Setup Pippin with 100,000 STABLE debt against WETH *****
    // STABLE = 0.95 LF
    // WETH = 2000 STABLE, 0.8 CF ~= 1520 STABLE/WETH
    let pippin_stable_debt = 100_000 * stable_scalar;
    let pippin_weth_collateral = 66 * weth_scalar;
    weth.mint(&pippin, &pippin_weth_collateral);
    pool_fixture.pool.submit(
        &pippin,
        &pippin,
        &pippin,
        &vec![
            &fixture.env,
            Request {
                request_type: RequestType::SupplyCollateral as u32,
                address: weth.address.clone(),
                amount: pippin_weth_collateral,
            },
            Request {
                request_type: RequestType::Borrow as u32,
                address: stable.address.clone(),
                amount: pippin_stable_debt,
            },
        ],
    );

    fixture.jump_with_sequence(100);

    // ***** Crash the price of XLM and WETH *****
    // XLM - want 50k supply value for sam to force ~60k of bad debt to
    //       be covered by backstop, w/ 10k profit for liquidator
    //     = 50k / 1.5m = 0.033
    // WETH - we want 50k supply value for pippin to force 60k of bad debt to
    //        defaulted, w/ 10k profit for liquidator
    //      = 50k / 66 = 757.6
    fixture.oracle.set_price_stable(&vec![
        &fixture.env,
        757_6000000, // eth
        1_0000000,   // usdc
        0_0330000,   // xlm
        1_0000000,   // stable
    ]);

    // ***** Liquidate Sam and auction off the bad debt *****

    // Setup Elrond to be the liquidator throughout this process
    let elrond_stable_balance = 500_000 * stable_scalar;
    stable.mint(&elrond, &elrond_stable_balance);

    // Create Sam's liquidation
    let sam_position_pre = pool_fixture.pool.get_positions(&sam);
    let sam_liquidation = pool_fixture.pool.new_auction(
        &0,
        &sam,
        &vec![&fixture.env, stable.address.clone()],
        &vec![&fixture.env, xlm.address.clone()],
        &100,
    );
    assert_eq!(
        sam_liquidation.bid.get_unchecked(stable.address.clone()),
        sam_position_pre
            .liabilities
            .get_unchecked(stable_pool_index)
    );
    assert_eq!(
        sam_liquidation.lot.get_unchecked(xlm.address.clone()),
        sam_position_pre.collateral.get_unchecked(xlm_pool_index)
    );

    // wait 320 blocks (plus 1 for auction to start) to fill such that
    // -> bid * 0.4 ~= 40k STABLE
    // -> lot * 1 = 1.5m XLM
    fixture.jump_with_sequence(320 * 5 + 5);

    // Fill the auction, repay debt and withdraw lot
    pool_fixture.pool.submit(
        &elrond,
        &elrond,
        &elrond,
        &vec![
            &fixture.env,
            Request {
                request_type: RequestType::FillUserLiquidationAuction as u32,
                address: sam.clone(),
                amount: 100,
            },
            Request {
                request_type: RequestType::Repay as u32,
                address: stable.address.clone(),
                amount: elrond_stable_balance / 2,
            },
            Request {
                request_type: RequestType::WithdrawCollateral as u32,
                address: xlm.address.clone(),
                amount: sam_xlm_collateral * 2,
            },
        ],
    );

    let sam_position_post = pool_fixture.pool.get_positions(&sam);
    assert_eq!(sam_position_post.collateral.len(), 0);
    assert_eq!(sam_position_post.liabilities.len(), 0);
    let backstop_post_liq_1 = pool_fixture.pool.get_positions(&fixture.backstop.address);
    assert_eq!(backstop_post_liq_1.collateral.len(), 0);
    assert_eq!(backstop_post_liq_1.liabilities.len(), 1);
    let bad_debt_1 = backstop_post_liq_1
        .liabilities
        .get_unchecked(stable_pool_index);
    // d_rate is barely above 1
    assert_approx_eq_rel(bad_debt_1, 60_000 * stable_scalar, 0_001000);

    // create bad debt auction to empty the backstop
    let pool_backstop_data = fixture.backstop.pool_data(&pool_fixture.pool.address);
    assert!(pool_backstop_data.tokens > 0);
    assert!(pool_backstop_data.shares > 0);

    let bad_debt_auction = pool_fixture.pool.new_auction(
        &1,
        &fixture.backstop.address,
        &vec![&fixture.env, stable.address.clone()],
        &vec![&fixture.env, fixture.lp.address.clone()],
        &100,
    );
    assert_eq!(bad_debt_auction.bid.len(), 1);
    assert_eq!(
        bad_debt_auction.bid.get_unchecked(stable.address.clone()),
        bad_debt_1
    );
    assert_eq!(bad_debt_auction.lot.len(), 1);
    assert_eq!(
        bad_debt_auction
            .lot
            .get_unchecked(fixture.lp.address.clone()),
        pool_backstop_data.tokens
    );

    // wait 200 blocks (plus 1 block for auction to start)
    // to fill the full auction (take all backstop tokens)
    fixture.jump_with_sequence(200 * 5 + 5);

    pool_fixture.pool.submit(
        &elrond,
        &elrond,
        &elrond,
        &vec![
            &fixture.env,
            Request {
                request_type: RequestType::FillBadDebtAuction as u32,
                address: fixture.backstop.address.clone(),
                amount: 100,
            },
            Request {
                request_type: RequestType::Repay as u32,
                address: stable.address.clone(),
                amount: elrond_stable_balance / 2,
            },
        ],
    );

    let pool_backstop_data = fixture.backstop.pool_data(&pool_fixture.pool.address);
    assert_eq!(pool_backstop_data.tokens, 0);
    assert!(pool_backstop_data.shares > 0);

    // ***** Liquidate Pippin and auction off the bad debt *****

    // Create Pippin's liquidation
    let pippin_position_pre = pool_fixture.pool.get_positions(&pippin);
    let pippin_liquidation = pool_fixture.pool.new_auction(
        &0,
        &pippin,
        &vec![&fixture.env, stable.address.clone()],
        &vec![&fixture.env, weth.address.clone()],
        &100,
    );
    assert_eq!(
        pippin_liquidation.bid.get_unchecked(stable.address.clone()),
        pippin_position_pre
            .liabilities
            .get_unchecked(stable_pool_index)
    );
    assert_eq!(
        pippin_liquidation.lot.get_unchecked(weth.address.clone()),
        pippin_position_pre
            .collateral
            .get_unchecked(weth_pool_index)
    );

    // wait 320 blocks (plus 1 for auction to start) to fill such that
    // -> bid * 0.4 ~= 40k STABLE
    // -> lot * 1 = 1.5m XLM
    fixture.jump_with_sequence(320 * 5 + 5);

    let pre_stable_reserve = pool_fixture.pool.get_reserve(&stable.address);

    // Fill the auction, repay debt and withdraw lot
    pool_fixture.pool.submit(
        &elrond,
        &elrond,
        &elrond,
        &vec![
            &fixture.env,
            Request {
                request_type: RequestType::FillUserLiquidationAuction as u32,
                address: pippin.clone(),
                amount: 100,
            },
            Request {
                request_type: RequestType::Repay as u32,
                address: stable.address.clone(),
                amount: elrond_stable_balance / 2,
            },
            Request {
                request_type: RequestType::WithdrawCollateral as u32,
                address: weth.address.clone(),
                amount: pippin_weth_collateral * 2,
            },
        ],
    );

    let pippin_position_post = pool_fixture.pool.get_positions(&pippin);
    assert_eq!(pippin_position_post.collateral.len(), 0);
    assert_eq!(pippin_position_post.liabilities.len(), 0);
    let backstop_post_liq_2 = pool_fixture.pool.get_positions(&fixture.backstop.address);
    assert_eq!(backstop_post_liq_2.collateral.len(), 0);
    assert_eq!(backstop_post_liq_2.liabilities.len(), 1);
    let bad_debt_2 = backstop_post_liq_2
        .liabilities
        .get_unchecked(stable_pool_index);
    // d_rate is barely above 1
    assert_approx_eq_rel(bad_debt_2, 60_000 * stable_scalar, 0_001000);

    // default the bad debt
    pool_fixture.pool.bad_debt(&fixture.backstop.address);

    // check b_rate loss (7 decimals)
    let post_stable_reserve = pool_fixture.pool.get_reserve(&stable.address);
    let supply_value = post_stable_reserve
        .data
        .b_rate
        .fixed_div_floor(pre_stable_reserve.data.b_rate, SCALAR_7)
        .unwrap();
    let supply_loss = SCALAR_7 - supply_value;
    // 310k STABLE supplied, 60k defaulted
    let est_loss = (60_000 * stable_scalar)
        .fixed_div_floor(310_000 * stable_scalar, SCALAR_7)
        .unwrap();

    assert_approx_eq_abs(est_loss, supply_loss, 0_0001000);

    fixture.jump_with_sequence(100);

    // ***** Frodo and Gandalf withdraw all STABLE funds *****

    // Have frodo repay outstanding liabilities first
    let frodo_stable_collateral = 10_000 * stable_scalar;
    pool_fixture.pool.submit(
        &frodo,
        &frodo,
        &frodo,
        &vec![
            &fixture.env,
            Request {
                request_type: RequestType::Repay as u32,
                address: stable.address.clone(),
                amount: frodo_stable_collateral,
            },
        ],
    );

    // Gandalf withdraw all supply
    let pre_wd_gandalf_stable = stable.balance(&gandalf);
    let wd_gandalf = pool_fixture.pool.submit(
        &gandalf,
        &gandalf,
        &gandalf,
        &vec![
            &fixture.env,
            Request {
                request_type: RequestType::Withdraw as u32,
                address: stable.address.clone(),
                amount: gandalf_stable_supply * 2,
            },
        ],
    );
    assert_eq!(wd_gandalf.supply.get(stable_pool_index), None);
    let post_wd_gandalf_stable = stable.balance(&gandalf);

    // Frodo withdraw all supply
    let pre_wd_frodo_stable = stable.balance(&frodo);
    pool_fixture.pool.submit(
        &frodo,
        &frodo,
        &frodo,
        &vec![
            &fixture.env,
            Request {
                request_type: RequestType::WithdrawCollateral as u32,
                address: stable.address.clone(),
                amount: frodo_stable_collateral * 2,
            },
        ],
    );
    assert_eq!(wd_gandalf.collateral.get(stable_pool_index), None);
    let post_wd_frodo_stable = stable.balance(&frodo);

    // verify slash is applied fairly
    let gandalf_wd_amount = post_wd_gandalf_stable - pre_wd_gandalf_stable;
    let frodo_wd_amount = post_wd_frodo_stable - pre_wd_frodo_stable;
    let gandalf_expected_wd_amount = gandalf_stable_supply
        .fixed_mul_floor(supply_value, SCALAR_7)
        .unwrap();
    let frodo_expected_wd_amount = frodo_stable_collateral
        .fixed_mul_floor(supply_value, SCALAR_7)
        .unwrap();
    assert_approx_eq_rel(gandalf_wd_amount, gandalf_expected_wd_amount, 0_0000100);
    assert_approx_eq_rel(frodo_wd_amount, frodo_expected_wd_amount, 0_0000100);

    // verify STABLE is empty
    let final_stable_reserve = pool_fixture.pool.get_reserve(&stable.address);
    // accrual occurs in between default since frodo still holds liabilities
    assert_approx_eq_abs(
        final_stable_reserve.data.b_rate,
        post_stable_reserve.data.b_rate,
        0_000_000_010_000,
    );
    assert_eq!(final_stable_reserve.data.b_supply, 0);
}
