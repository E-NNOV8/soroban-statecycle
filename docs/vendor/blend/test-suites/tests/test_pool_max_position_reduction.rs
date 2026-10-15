#![cfg(test)]

use pool::{Request, RequestType};
use soroban_sdk::{testutils::Address as _, vec, Address, Error};
use test_suites::{
    create_fixture_with_data,
    test_fixture::{TokenIndex, SCALAR_7},
};

#[test]
fn test_pool_max_positions_reduction() {
    let fixture = create_fixture_with_data(false);
    let pool_fixture = &fixture.pools[0];
    let weth_pool_index = pool_fixture.reserves[&TokenIndex::WETH];
    let xlm = &fixture.tokens[TokenIndex::XLM];
    let weth = &fixture.tokens[TokenIndex::WETH];
    let stable = &fixture.tokens[TokenIndex::STABLE];
    let stable_scalar: i128 = 10i128.pow(stable.decimals());
    let weth_scalar: i128 = 10i128.pow(weth.decimals());

    let sam = Address::generate(&fixture.env);

    // Mint sam tokens
    let xlm_amount = 10_000 * SCALAR_7;
    let weth_amount = 1 * weth_scalar;
    let stable_amount = 1_000 * stable_scalar;
    xlm.mint(&sam, &(xlm_amount * 2));
    weth.mint(&sam, &(weth_amount * 2));
    stable.mint(&sam, &(stable_amount * 2));

    // Pool allows 6 max positions by default

    // Sam create 6 positions
    let requests = vec![
        &fixture.env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: xlm.address.clone(),
            amount: xlm_amount,
        },
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: weth.address.clone(),
            amount: weth_amount,
        },
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: stable.address.clone(),
            amount: stable_amount,
        },
        Request {
            request_type: RequestType::Borrow as u32,
            address: xlm.address.clone(),
            amount: xlm_amount / 2,
        },
        Request {
            request_type: RequestType::Borrow as u32,
            address: weth.address.clone(),
            amount: weth_amount / 2,
        },
        Request {
            request_type: RequestType::Borrow as u32,
            address: stable.address.clone(),
            amount: stable_amount / 2,
        },
    ];
    let result = pool_fixture.pool.submit(&sam, &sam, &sam, &requests);

    assert_eq!(result.collateral.len(), 3);
    assert_eq!(result.liabilities.len(), 3);

    fixture.jump_with_sequence(100);

    // admin lowers max positions to 4
    pool_fixture.pool.update_pool(&0_1000000, &4, &1_0000000);

    fixture.jump_with_sequence(100);

    // verify sam can remove positions via repay
    let request = vec![
        &fixture.env,
        Request {
            request_type: RequestType::Repay as u32,
            address: weth.address.clone(),
            amount: weth_amount / 2 + 1000,
        },
    ];
    let result = pool_fixture.pool.submit(&sam, &sam, &sam, &request);

    assert_eq!(result.collateral.len(), 3);
    assert_eq!(result.liabilities.len(), 2);

    // verify sam can remove positions via withdraw
    let request = vec![
        &fixture.env,
        Request {
            request_type: RequestType::WithdrawCollateral as u32,
            address: xlm.address.clone(),
            amount: xlm_amount + SCALAR_7,
        },
    ];
    let result = pool_fixture.pool.submit(&sam, &sam, &sam, &request);
    assert_eq!(result.collateral.len(), 2);
    assert_eq!(result.liabilities.len(), 2);

    // verify sam cannot create new positions anymore
    let request = vec![
        &fixture.env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: xlm.address.clone(),
            amount: SCALAR_7,
        },
    ];
    let max_position_error = pool_fixture.pool.try_submit(&sam, &sam, &sam, &request);
    assert_eq!(
        max_position_error.err(),
        Some(Ok(Error::from_contract_error(1208)))
    );

    // verify sam can use existing positions
    let prev_weth_collat = result.collateral.get_unchecked(weth_pool_index);
    let request = vec![
        &fixture.env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: weth.address.clone(),
            amount: weth_amount / 2,
        },
    ];
    let result = pool_fixture.pool.submit(&sam, &sam, &sam, &request);
    assert_eq!(result.collateral.len(), 2);
    assert_eq!(result.liabilities.len(), 2);
    assert!(result.collateral.get_unchecked(weth_pool_index) > prev_weth_collat);
}
