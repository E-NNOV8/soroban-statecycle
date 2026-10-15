#![cfg(test)]
use pool::{FlashLoan, Request, RequestType};
use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::{
    map,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Events},
    vec, Address, IntoVal, Symbol, Val, Vec,
};
use test_suites::{
    create_fixture_with_data,
    moderc3156::create_flashloan_receiver,
    test_fixture::{TokenIndex, SCALAR_12, SCALAR_7},
};

#[test]
fn test_flashloan() {
    let fixture = create_fixture_with_data(false);
    let pool_fixture = &fixture.pools[0];

    let xlm = &fixture.tokens[TokenIndex::XLM];
    let xlm_address = xlm.address.clone();
    let stable = &fixture.tokens[TokenIndex::STABLE];
    let stable_address = stable.address.clone();

    let (receiver_address, _) = create_flashloan_receiver(&fixture.env);

    let samwise = Address::generate(&fixture.env);

    let pool_starting_xlm_balance = xlm.balance(&pool_fixture.pool.address);
    let pool_starting_stable_balance = stable.balance(&pool_fixture.pool.address);
    let starting_xlm_balance = 100 * SCALAR_7;
    let starting_stable_balance = 100 * SCALAR_7;
    let approval_ledger = fixture.env.ledger().sequence() + 17280;

    xlm.mint(&samwise, &starting_xlm_balance);
    xlm.approve(
        &samwise,
        &pool_fixture.pool.address,
        &i128::MAX,
        &approval_ledger,
    );
    stable.mint(&samwise, &starting_stable_balance);
    stable.approve(
        &samwise,
        &pool_fixture.pool.address,
        &starting_stable_balance,
        &approval_ledger,
    );

    let flash_loan = FlashLoan {
        contract: receiver_address.clone(),
        asset: xlm_address.clone(),
        amount: 1_000 * SCALAR_7,
    };
    let supply_amount = 50 * SCALAR_7;
    let repay_amount = 900 * SCALAR_7;
    let requests: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: stable_address.clone(),
            amount: supply_amount,
        },
        Request {
            request_type: RequestType::Repay as u32,
            address: xlm_address.clone(),
            amount: repay_amount,
        },
    ];

    let result = pool_fixture
        .pool
        .flash_loan(&samwise, &flash_loan, &requests);

    // valdiate auth
    assert_eq!(
        fixture.env.auths()[0],
        (
            samwise.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    pool_fixture.pool.address.clone(),
                    Symbol::new(&fixture.env, "flash_loan"),
                    vec![
                        &fixture.env,
                        samwise.to_val(),
                        flash_loan.into_val(&fixture.env),
                        requests.to_val(),
                    ]
                )),
                sub_invocations: std::vec![AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        receiver_address.clone(),
                        Symbol::new(&fixture.env, "exec_op"),
                        vec![
                            &fixture.env,
                            samwise.to_val(),
                            flash_loan.asset.to_val(),
                            flash_loan.amount.into_val(&fixture.env),
                            0i128.into_val(&fixture.env),
                        ]
                    )),
                    sub_invocations: std::vec![]
                }]
            }
        )
    );

    // validate events
    let events = fixture.env.events().all();

    let xlm_res_data = pool_fixture.pool.get_reserve(&xlm_address);
    let stable_res_data = pool_fixture.pool.get_reserve(&stable_address);

    let flash_loan_events = vec![&fixture.env, events.get_unchecked(0)];
    let flash_loan_d_tokens_minted = flash_loan
        .amount
        .fixed_div_ceil(xlm_res_data.data.d_rate, SCALAR_12)
        .unwrap();
    let flash_loan_event_data: soroban_sdk::Vec<Val> = vec![
        &fixture.env,
        flash_loan.amount.into_val(&fixture.env),
        flash_loan_d_tokens_minted.into_val(&fixture.env),
    ];
    assert_eq!(
        flash_loan_events,
        vec![
            &fixture.env,
            (
                pool_fixture.pool.address.clone(),
                (
                    Symbol::new(&fixture.env, "flash_loan"),
                    flash_loan.asset.clone(),
                    samwise.clone(),
                    flash_loan.contract.clone(),
                )
                    .into_val(&fixture.env),
                flash_loan_event_data.into_val(&fixture.env),
            )
        ]
    );

    let supply_event = vec![&fixture.env, events.get_unchecked(1)];
    let supply_b_tokens_minted = supply_amount
        .fixed_div_floor(stable_res_data.data.b_rate, SCALAR_12)
        .unwrap();
    let supply_event_data: soroban_sdk::Vec<Val> = vec![
        &fixture.env,
        supply_amount.into_val(&fixture.env),
        supply_b_tokens_minted.into_val(&fixture.env),
    ];
    assert_eq!(
        supply_event,
        vec![
            &fixture.env,
            (
                pool_fixture.pool.address.clone(),
                (
                    Symbol::new(&fixture.env, "supply_collateral"),
                    stable_address.clone(),
                    samwise.clone(),
                )
                    .into_val(&fixture.env),
                supply_event_data.into_val(&fixture.env),
            )
        ]
    );

    let repay_event = vec![&fixture.env, events.get_unchecked(2)];
    let repay_d_tokens_burned = repay_amount
        .fixed_div_floor(xlm_res_data.data.d_rate, SCALAR_12)
        .unwrap();
    let repay_event_data: soroban_sdk::Vec<Val> = vec![
        &fixture.env,
        repay_amount.into_val(&fixture.env),
        repay_d_tokens_burned.into_val(&fixture.env),
    ];
    assert_eq!(
        repay_event,
        vec![
            &fixture.env,
            (
                pool_fixture.pool.address.clone(),
                (
                    Symbol::new(&fixture.env, "repay"),
                    xlm_address.clone(),
                    samwise.clone(),
                )
                    .into_val(&fixture.env),
                repay_event_data.into_val(&fixture.env),
            )
        ]
    );

    // validate results
    assert_eq!(result.collateral.len(), 1);
    assert_eq!(result.liabilities.len(), 1);
    assert_eq!(result.supply.len(), 0);

    assert_eq!(result.collateral.get_unchecked(0), supply_b_tokens_minted);
    assert_eq!(
        result.liabilities.get_unchecked(1),
        flash_loan_d_tokens_minted - repay_d_tokens_burned
    );

    assert_eq!(
        stable.balance(&pool_fixture.pool.address),
        pool_starting_stable_balance + supply_amount
    );
    assert_eq!(
        xlm.balance(&pool_fixture.pool.address),
        pool_starting_xlm_balance - flash_loan.amount + repay_amount
    );

    assert_eq!(
        xlm.balance(&samwise),
        starting_xlm_balance + flash_loan.amount - repay_amount
    );
    assert_eq!(
        stable.balance(&samwise),
        starting_stable_balance - supply_amount
    );
}

#[test]
fn test_flashloan_reentrancy_disabled() {
    let fixture = create_fixture_with_data(true);
    let pool_fixture = &fixture.pools[0];

    let xlm = &fixture.tokens[TokenIndex::XLM];
    let xlm_address = xlm.address.clone();
    let stable = &fixture.tokens[TokenIndex::STABLE];
    let stable_address = stable.address.clone();

    let (receiver_address, receiver_client) = create_flashloan_receiver(&fixture.env);

    let samwise = Address::generate(&fixture.env);
    let merry = Address::generate(&fixture.env);

    let starting_xlm_balance = 100 * SCALAR_7;
    let starting_stable_balance = 100 * SCALAR_7;
    let approval_ledger = fixture.env.ledger().sequence() + 17280;

    xlm.mint(&samwise, &starting_xlm_balance);
    xlm.approve(
        &samwise,
        &pool_fixture.pool.address,
        &i128::MAX,
        &approval_ledger,
    );
    stable.mint(&samwise, &starting_stable_balance);
    stable.approve(
        &samwise,
        &pool_fixture.pool.address,
        &starting_stable_balance,
        &approval_ledger,
    );

    // test that the flash loan contract works
    receiver_client.set_re_entrant(&pool_fixture.pool.address);
    xlm.mint(&receiver_address, &starting_xlm_balance);
    stable.mint(&merry, &starting_stable_balance);

    // 1. supply collateral so merry can borrow XLM
    pool_fixture.pool.submit(
        &merry,
        &merry,
        &merry,
        &vec![
            &fixture.env,
            Request {
                request_type: RequestType::SupplyCollateral as u32,
                address: stable_address.clone(),
                amount: starting_stable_balance,
            },
        ],
    );

    // 2. call receiver contract to borrow XLM on behalf of merry, and return the flash loan amount
    receiver_client.exec_op(&merry, &xlm.address, &starting_xlm_balance, &0);

    let merry_positions = pool_fixture.pool.get_positions(&merry);
    assert_eq!(
        merry_positions.liabilities,
        map![&fixture.env, (1, 999974126)]
    );
    assert_eq!(
        merry_positions.collateral,
        map![&fixture.env, (0, 999995310)]
    );
    assert_eq!(starting_xlm_balance * 2, xlm.balance(&merry));

    // attempt to do reentrancy attack. setup samwise with enough collateral
    // to complete the malicious borrow
    pool_fixture.pool.submit(
        &samwise,
        &samwise,
        &samwise,
        &vec![
            &fixture.env,
            Request {
                request_type: RequestType::SupplyCollateral as u32,
                address: stable_address.clone(),
                amount: starting_stable_balance,
            },
        ],
    );

    let flash_loan = FlashLoan {
        contract: receiver_address.clone(),
        asset: xlm_address.clone(),
        amount: 100 * SCALAR_7,
    };
    let requests: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::Repay as u32,
            address: xlm_address.clone(),
            amount: 101 * SCALAR_7,
        },
    ];

    // validate re-entrancy attack is protected against by the env
    let result = pool_fixture
        .pool
        .try_flash_loan(&samwise, &flash_loan, &requests);
    assert_eq!(
        result.err(),
        Some(Ok(soroban_sdk::Error::from_type_and_code(
            soroban_sdk::xdr::ScErrorType::Context,
            soroban_sdk::xdr::ScErrorCode::InvalidAction
        )))
    );
}
