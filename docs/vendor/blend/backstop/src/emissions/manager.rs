use cast::{i128, u64};
use sep_41_token::TokenClient;
use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::{panic_with_error, unwrap::UnwrapOptimized, vec, Address, Env, Vec};

use crate::{
    backstop::{is_pool_above_threshold, load_pool_backstop_data},
    constants::{MAX_BACKFILLED_EMISSIONS, MAX_RZ_SIZE, SCALAR_7},
    dependencies::EmitterClient,
    errors::BackstopError,
    storage::{self, BackstopEmissionData, RzEmissions},
    PoolBalance,
};

use super::distributor::update_emission_data;

/// Add a pool to the reward zone. If the reward zone is full, attempt to swap it with the pool to remove.
pub fn add_to_reward_zone(e: &Env, to_add: Address, to_remove: Option<Address>) {
    let mut reward_zone = storage::get_reward_zone(e);

    // ensure an entity in the reward zone cannot be included twice
    if reward_zone.contains(to_add.clone()) {
        panic_with_error!(e, BackstopError::BadRequest);
    }

    // ensure to_add has met the minimum backstop deposit threshold
    // NOTE: "to_add" can only carry a pool balance if it is a deployed pool from the factory
    let pool_data = load_pool_backstop_data(e, &to_add);
    if !is_pool_above_threshold(&pool_data) {
        panic_with_error!(e, BackstopError::InvalidRewardZoneEntry);
    }

    // if updating the rz list, ensure distribute was run recently
    if reward_zone.len() > 0 {
        require_distribute_run_recently(e);
    }

    if MAX_RZ_SIZE > reward_zone.len() {
        // there is room in the reward zone. Add "to_add".
        reward_zone.push_front(to_add.clone());
    } else {
        match to_remove {
            None => panic_with_error!(e, BackstopError::RewardZoneFull),
            Some(to_remove) => {
                // Verify "to_add" has a higher backstop deposit that "to_remove"
                if pool_data.tokens <= storage::get_pool_balance(e, &to_remove).tokens {
                    panic_with_error!(e, BackstopError::InvalidRewardZoneEntry);
                }
                remove_pool(e, &mut reward_zone, &to_remove);
                reward_zone.push_front(to_add.clone());
            }
        }
    }
    storage::set_reward_zone(e, &reward_zone);
}

/// Remove a pool to the reward zone if below the minimum backstop deposit threshold
pub fn remove_from_reward_zone(e: &Env, to_remove: Address) {
    let mut reward_zone = storage::get_reward_zone(e);

    // ensure to_remove has not met the backstop threshold
    let pool_data = load_pool_backstop_data(e, &to_remove);
    if is_pool_above_threshold(&pool_data) {
        panic_with_error!(e, BackstopError::BadRequest);
    } else {
        require_distribute_run_recently(e);
        remove_pool(e, &mut reward_zone, &to_remove);
        storage::set_reward_zone(e, &reward_zone);
    }
}

/// Remove a pool from the reward zone
fn remove_pool(e: &Env, reward_zone: &mut Vec<Address>, to_remove: &Address) {
    let to_remove_index = reward_zone.first_index_of(to_remove.clone());
    match to_remove_index {
        Some(idx) => {
            reward_zone.remove(idx);
        }
        None => panic_with_error!(e, BackstopError::InvalidRewardZoneEntry),
    }
}

/// Require distribute was run recently to prevent rz edits from significantly disrupting emissions
///
/// Note - this will always fail after the emitter stops emitting tokens to the backstop. This is
/// ok as the reward zone is only used to determine the distribution of said emissions.
fn require_distribute_run_recently(e: &Env) {
    let last_distribution = storage::get_last_distribution_time(e);
    if last_distribution < e.ledger().timestamp() - 60 * 60 {
        panic_with_error!(e, BackstopError::BadRequest);
    }
}

/// Distribute emissions from the emitter to the reward zone and backstop depositors. This also implements
/// backfilling emissions if the emitter has not distributed to this version of the backstop before.
pub fn distribute(e: &Env) -> i128 {
    let is_backfill: bool;
    let mut needs_reset: bool = false;
    let last_backfill_status = storage::get_backfill_status(e);
    let emitter = storage::get_emitter(e);
    let emitter_last_distribution =
        match EmitterClient::new(&e, &emitter).try_get_last_distro(&e.current_contract_address()) {
            Ok(distro) => {
                is_backfill = false;
                if last_backfill_status != Some(false) {
                    // first time the backstop has gotten a distro time from the emitter
                    // reset last distribution time if we were backfilling previously
                    needs_reset = last_backfill_status == Some(true);
                    storage::set_backfill_status(e, &false);
                }
                distro.unwrap_optimized()
            }
            // allows for backfilled emissions
            Err(_) => {
                is_backfill = true;
                if last_backfill_status.is_none() {
                    // first time calling with backfill emissions
                    storage::set_backfill_status(e, &true);
                } else if last_backfill_status == Some(false) {
                    // backfilling has already stopped. Getting an error from the emitter
                    // is unexpected.
                    panic_with_error!(e, BackstopError::BadRequest);
                }
                e.ledger().timestamp()
            }
        };
    let last_distribution = storage::get_last_distribution_time(e);

    // if we have never distributed before, record the emitter's last distribution time and
    // start emissions from that time
    if last_distribution == 0 {
        storage::set_last_distribution_time(e, &emitter_last_distribution);
        return 0;
    }

    // if this is the first distribution after a backstop swap, we need to stop the backfill emissions
    // safely. The only way to do this is to reset the last distribution time to the emitters.
    // This skips all emissions between the last distribution time and the emitter's last distribution time.
    // This is necessary as the backstop cannot determine how much BLND was actually emitted
    // between those two timepoints.
    if needs_reset {
        storage::set_last_distribution_time(e, &emitter_last_distribution);
        return 0;
    }

    // if at least 5 seconds (1 block) has not passed, panic
    if emitter_last_distribution - last_distribution < 5 {
        panic_with_error!(e, BackstopError::BadRequest);
    }

    let reward_zone = storage::get_reward_zone(e);
    let rz_len = reward_zone.len();
    // reward zone must have at least one pool for emissions to start
    if rz_len == 0 {
        panic_with_error!(e, BackstopError::BadRequest);
    }

    // emitter releases 1 token per second
    let mut new_emissions = i128(emitter_last_distribution - last_distribution) * SCALAR_7;

    // if backfilling emissions, ensure we are not over the maximum backfilled emissions allotment.
    // backfilled emissions must fit within the maximum drop amount from the emitter.
    if is_backfill {
        let mut cur_backfill = storage::get_backfill_emissions(e);
        // panic if we already reached the maximum backfilled emissions
        if cur_backfill >= MAX_BACKFILLED_EMISSIONS {
            panic_with_error!(e, BackstopError::MaxBackfillEmissions);
        }
        // cap new emissions to the maximum backfilled emissions
        if new_emissions + cur_backfill > MAX_BACKFILLED_EMISSIONS {
            new_emissions = MAX_BACKFILLED_EMISSIONS - cur_backfill;
        }
        cur_backfill += new_emissions;
        storage::set_backfill_emissions(e, &cur_backfill);
    }
    storage::set_last_distribution_time(e, &emitter_last_distribution);

    let mut rz_balance: Vec<(Address, PoolBalance)> = vec![e];

    // fetch total non-queued backstop tokens in the reward zone
    let mut total_non_queued_tokens: i128 = 0;
    for rz_pool in reward_zone {
        let pool_balance = storage::get_pool_balance(e, &rz_pool);
        total_non_queued_tokens += pool_balance.non_queued_tokens();
        rz_balance.push_back((rz_pool, pool_balance));
    }

    // store emissions due for each reward zone pool
    for (rz_pool, pool_balance) in rz_balance {
        let pool_non_queued_tokens = pool_balance.non_queued_tokens();
        let share = pool_non_queued_tokens
            .fixed_div_floor(total_non_queued_tokens, SCALAR_7)
            .unwrap_optimized();

        let new_pool_emissions = share
            .fixed_mul_floor(new_emissions, SCALAR_7)
            .unwrap_optimized();
        let mut accrued_emissions = storage::get_rz_emis(e, &rz_pool);
        accrued_emissions.accrued += new_pool_emissions;
        storage::set_rz_emis(e, &rz_pool, &accrued_emissions);
    }

    return new_emissions;
}

/// Assign backstop and pool emissions to `pool` based on the reward zone and the backstop emissions index
/// Returns the amount of backstop and pool emissions assigned to the pool
#[allow(clippy::zero_prefixed_literal)]
pub fn gulp_emissions(e: &Env, pool: &Address) -> (i128, i128) {
    let pool_balance = storage::get_pool_balance(e, pool);
    let new_emissions = storage::get_rz_emis(e, pool);

    // Only allow pools to accrue once per day
    if new_emissions.last_time > e.ledger().timestamp() - 24 * 60 * 60 {
        panic_with_error!(e, BackstopError::BadRequest);
    }

    if new_emissions.accrued > 0 {
        let new_backstop_emissions = new_emissions
            .accrued
            .fixed_mul_floor(0_7000000, SCALAR_7)
            .unwrap_optimized();
        let new_pool_emissions = new_emissions
            .accrued
            .fixed_mul_floor(0_3000000, SCALAR_7)
            .unwrap_optimized();

        // distribute pool emissions via allowance to pools
        let blnd_token_client = TokenClient::new(e, &storage::get_blnd_token(e));
        let current_allowance = blnd_token_client.allowance(&e.current_contract_address(), pool);
        let new_seq = e.ledger().sequence() + storage::LEDGER_BUMP_USER; // ~120 days
        blnd_token_client.approve(
            &e.current_contract_address(),
            pool,
            &(current_allowance + new_pool_emissions),
            &new_seq,
        );
        storage::set_rz_emis(
            e,
            pool,
            &RzEmissions {
                accrued: 0,
                last_time: e.ledger().timestamp(),
            },
        );
        set_backstop_emission_eps(e, pool, &pool_balance, new_backstop_emissions);
        return (new_backstop_emissions, new_pool_emissions);
    }
    return (0, 0);
}

/// Set a new EPS for the backstop
pub fn set_backstop_emission_eps(
    e: &Env,
    pool_id: &Address,
    pool_balance: &PoolBalance,
    new_tokens: i128,
) {
    let mut tokens_left_to_emit = new_tokens;
    let expiration = e.ledger().timestamp() + 7 * 24 * 60 * 60;

    if let Some(mut emission_data) = update_emission_data(e, pool_id, &pool_balance) {
        // a previous data exists - update with old data before setting new EPS
        if emission_data.last_time != e.ledger().timestamp() {
            // force the emission data to be updated to the current timestamp
            emission_data.last_time = e.ledger().timestamp();
        }
        // determine the amount of tokens not emitted from the last config
        if emission_data.expiration > e.ledger().timestamp() {
            let time_since_last_emission = emission_data.expiration - e.ledger().timestamp();

            // Eps is scaled by 14 decimal places
            let tokens_since_last_emission = i128(emission_data.eps)
                .fixed_mul_floor(i128(time_since_last_emission), SCALAR_7)
                .unwrap_optimized();
            tokens_left_to_emit += tokens_since_last_emission;
        }
        // Scale eps by 14 decimal places to reduce rounding errors
        let eps = u64(tokens_left_to_emit * SCALAR_7 / (7 * 24 * 60 * 60)).unwrap_optimized();
        emission_data.eps = eps;
        emission_data.expiration = expiration;
        storage::set_backstop_emis_data(e, pool_id, &emission_data);
    } else {
        // first time the pool's backstop is receiving emissions - ensure data is written
        let eps = u64(tokens_left_to_emit * SCALAR_7 / (7 * 24 * 60 * 60)).unwrap_optimized();
        storage::set_backstop_emis_data(
            e,
            pool_id,
            &BackstopEmissionData {
                eps,
                expiration,
                index: 0,
                last_time: e.ledger().timestamp(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        vec, Vec,
    };

    use crate::{
        backstop::PoolBalance,
        testutils::{
            create_backstop, create_blnd_token, create_comet_lp_pool_with_tokens_per_share,
            create_emitter, create_usdc_token,
        },
    };

    /********** gulp_emissions **********/

    #[test]
    fn test_gulp_emissions_outside_rz() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let backstop = create_backstop(&e);
        let blnd_token_client = create_blnd_token(&e, &backstop, &Address::generate(&e)).1;
        let pool_1 = Address::generate(&e);
        let pool_2 = Address::generate(&e);
        let pool_3 = Address::generate(&e);
        let reward_zone: Vec<Address> = vec![&e, pool_2.clone(), pool_3.clone()];

        // setup pool 1 to have ongoing emissions - it was recently removed from RZ
        let pool_1_emissions_data = BackstopEmissionData {
            expiration: 1713139200 + 86400,
            eps: 0_10000000000000,
            index: 887766550000000,
            last_time: 1713139200 - 12345,
        };
        let pool_1_accrued = RzEmissions {
            accrued: 20_000_0000000,
            last_time: 0,
        };
        let pool_1_allowance: i128 = 100_123_0000000;
        e.as_contract(&backstop, || {
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_backstop_emis_data(&e, &pool_1, &pool_1_emissions_data);
            storage::set_pool_balance(
                &e,
                &pool_1,
                // 35_000_0000000 unqeued shares
                &PoolBalance {
                    tokens: 150_000_0000000,
                    shares: 40_000_0000000,
                    q4w: 5_000_0000000,
                },
            );
            blnd_token_client.approve(
                &backstop,
                &pool_1,
                &pool_1_allowance,
                &e.ledger().sequence(),
            );
            storage::set_rz_emis(&e, &pool_1, &pool_1_accrued);

            gulp_emissions(&e, &pool_1);

            assert_eq!(
                blnd_token_client.allowance(&backstop, &pool_1),
                pool_1_allowance + 6_000_0000000
            );
            let new_pool_1_data = storage::get_backstop_emis_data(&e, &pool_1).unwrap_optimized();
            assert_eq!(new_pool_1_data.eps, 0_0374338_6243386);
            assert_eq!(new_pool_1_data.expiration, 1713139200 + 7 * 24 * 60 * 60);
            assert_eq!(
                new_pool_1_data.index,
                pool_1_emissions_data.index + 0_0352714_2857142
            );
            assert_eq!(new_pool_1_data.last_time, 1713139200);
            let rz_emis = storage::get_rz_emis(&e, &pool_1);
            assert_eq!(rz_emis.accrued, 0);
            assert_eq!(rz_emis.last_time, 1713139200);
        });
    }

    #[test]
    fn test_gulp_emissions() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let backstop = create_backstop(&e);
        let emitter_distro_time = 1713139200 - 10;
        let blnd_token_client = create_blnd_token(&e, &backstop, &Address::generate(&e)).1;
        create_emitter(
            &e,
            &backstop,
            &Address::generate(&e),
            &Address::generate(&e),
            emitter_distro_time,
        );
        let pool_1 = Address::generate(&e);
        let pool_2 = Address::generate(&e);
        let pool_3 = Address::generate(&e);
        let reward_zone: Vec<Address> = vec![&e, pool_1.clone(), pool_2.clone(), pool_3.clone()];

        // setup pool 1 to have ongoing emissions
        let pool_1_emissions_data = BackstopEmissionData {
            expiration: 1713139200 + 1000,
            eps: 0_10000000000000,
            index: 8877660000000,
            last_time: 1713139200 - 12345,
        };

        // setup pool 2 to have expired emissions
        let pool_2_emissions_data = BackstopEmissionData {
            expiration: 1713139200 - 12345,
            eps: 0_05000000000000,
            index: 4532340000000,
            last_time: 1713139200 - 12345,
        };
        // setup pool 3 to have no emissions
        e.as_contract(&backstop, || {
            storage::set_last_distribution_time(&e, &(emitter_distro_time - 7 * 24 * 60 * 60));
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_backstop_emis_data(&e, &pool_1, &pool_1_emissions_data);
            storage::set_backstop_emis_data(&e, &pool_2, &pool_2_emissions_data);
            storage::set_pool_balance(
                &e,
                &pool_1,
                &PoolBalance {
                    tokens: 300_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_2,
                &PoolBalance {
                    tokens: 200_000_0000000,
                    shares: 150_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_3,
                &PoolBalance {
                    tokens: 500_000_0000000,
                    shares: 600_000_0000000,
                    q4w: 0,
                },
            );
            blnd_token_client.approve(&backstop, &pool_1, &100_123_0000000, &e.ledger().sequence());

            distribute(&e);
            gulp_emissions(&e, &pool_1);
            gulp_emissions(&e, &pool_2);
            gulp_emissions(&e, &pool_3);

            assert_eq!(storage::get_last_distribution_time(&e), emitter_distro_time);
            assert_eq!(
                storage::get_pool_balance(&e, &pool_1).tokens,
                300_000_0000000
            );
            assert_eq!(
                storage::get_pool_balance(&e, &pool_2).tokens,
                200_000_0000000
            );
            assert_eq!(
                storage::get_pool_balance(&e, &pool_3).tokens,
                500_000_0000000
            );
            assert_eq!(
                blnd_token_client.allowance(&backstop, &pool_1),
                154_555_0000000
            );
            assert_eq!(
                blnd_token_client.allowance(&backstop, &pool_2),
                36_288_0000000
            );
            assert_eq!(
                blnd_token_client.allowance(&backstop, &pool_3),
                90_720_0000000
            );

            // validate backstop emissions

            let new_pool_1_data = storage::get_backstop_emis_data(&e, &pool_1).unwrap_optimized();
            assert_eq!(new_pool_1_data.eps, 0_21016534391534);
            assert_eq!(new_pool_1_data.expiration, 1713139200 + 7 * 24 * 60 * 60);
            assert_eq!(new_pool_1_data.index, 9494910000000);
            assert_eq!(new_pool_1_data.last_time, 1713139200);
            let rz_emis_1 = storage::get_rz_emis(&e, &pool_1);
            assert_eq!(rz_emis_1.accrued, 0);
            assert_eq!(rz_emis_1.last_time, 1713139200);

            let new_pool_2_data = storage::get_backstop_emis_data(&e, &pool_2).unwrap_optimized();
            assert_eq!(new_pool_2_data.eps, 0_14000000000000);
            assert_eq!(new_pool_2_data.expiration, 1713139200 + 7 * 24 * 60 * 60);
            assert_eq!(new_pool_2_data.index, 4532340000000);
            assert_eq!(new_pool_2_data.last_time, 1713139200);
            let rz_emis_2 = storage::get_rz_emis(&e, &pool_2);
            assert_eq!(rz_emis_2.accrued, 0);
            assert_eq!(rz_emis_2.last_time, 1713139200);

            let new_pool_3_data = storage::get_backstop_emis_data(&e, &pool_3).unwrap_optimized();
            assert_eq!(new_pool_3_data.eps, 0_35000000000000);
            assert_eq!(new_pool_3_data.expiration, 1713139200 + 7 * 24 * 60 * 60);
            assert_eq!(new_pool_3_data.index, 0);
            assert_eq!(new_pool_3_data.last_time, 1713139200);
            let rz_emis_3 = storage::get_rz_emis(&e, &pool_3);
            assert_eq!(rz_emis_3.accrued, 0);
            assert_eq!(rz_emis_3.last_time, 1713139200);
        });
    }

    /********** distribute **********/

    #[test]
    fn test_distribute() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let backstop = create_backstop(&e);
        let emitter_distro_time = 1713139200 - 10;
        create_emitter(
            &e,
            &backstop,
            &Address::generate(&e),
            &Address::generate(&e),
            emitter_distro_time,
        );

        let pool_1 = Address::generate(&e);
        let pool_2 = Address::generate(&e);
        let pool_3 = Address::generate(&e);
        let reward_zone: Vec<Address> = vec![&e, pool_1.clone(), pool_2.clone(), pool_3.clone()];

        let start_pool_2_accrued = RzEmissions {
            accrued: 1_0000001,
            last_time: 123,
        };
        let start_pool_3_accrued = RzEmissions {
            accrued: 20_000_0000000,
            last_time: 0,
        };

        e.as_contract(&backstop, || {
            storage::set_backfill_status(&e, &false);
            storage::set_last_distribution_time(&e, &(emitter_distro_time - (60 * 60 * 24)));
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_pool_balance(
                &e,
                &pool_1,
                // 300_000_0000000 unqueued tokens
                &PoolBalance {
                    tokens: 300_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_2,
                // 200_000_0000000 unqueued tokens
                &PoolBalance {
                    tokens: 400_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 100_000_0000000,
                },
            );
            storage::set_rz_emis(&e, &pool_2, &start_pool_2_accrued);
            storage::set_pool_balance(
                &e,
                &pool_3,
                // 500_000_0000000 unqueued tokens
                &PoolBalance {
                    tokens: 1_000_000_0000000,
                    shares: 1_200_000_0000000,
                    q4w: 600_000_0000000,
                },
            );
            storage::set_rz_emis(&e, &pool_3, &start_pool_3_accrued);

            distribute(&e);

            let last_distro_time = storage::get_last_distribution_time(&e);
            assert_eq!(last_distro_time, emitter_distro_time);
            let backfilled_emissions = storage::get_backfill_emissions(&e);
            assert_eq!(backfilled_emissions, 0);

            let pool_1_accrued = storage::get_rz_emis(&e, &pool_1);
            assert_eq!(pool_1_accrued.accrued, 25_920_0000000 + 0);
            assert_eq!(pool_1_accrued.last_time, 0);
            let pool_2_accrued = storage::get_rz_emis(&e, &pool_2);
            assert_eq!(
                pool_2_accrued.accrued,
                17_280_0000000 + start_pool_2_accrued.accrued
            );
            assert_eq!(pool_2_accrued.last_time, start_pool_2_accrued.last_time);
            let pool_3_accrued = storage::get_rz_emis(&e, &pool_3);
            assert_eq!(
                pool_3_accrued.accrued,
                43_200_0000000 + start_pool_3_accrued.accrued
            );
            assert_eq!(pool_3_accrued.last_time, start_pool_3_accrued.last_time);

            // backfill status remains false
            let backfill_status = storage::get_backfill_status(&e);
            assert_eq!(backfill_status, Some(false));
        });
    }

    #[test]
    fn test_distribute_one_block_rounding_ok() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let backstop = create_backstop(&e);
        let emitter_distro_time = 1713139200 - 5;
        create_emitter(
            &e,
            &backstop,
            &Address::generate(&e),
            &Address::generate(&e),
            emitter_distro_time,
        );

        let pool_1 = Address::generate(&e);
        let pool_2 = Address::generate(&e);
        let pool_3 = Address::generate(&e);
        let reward_zone: Vec<Address> = vec![&e, pool_1.clone(), pool_2.clone(), pool_3.clone()];

        let start_pool_2_accrued = RzEmissions {
            accrued: 1_0000001,
            last_time: 123,
        };
        let start_pool_3_accrued = RzEmissions {
            accrued: 20_000_0000000,
            last_time: 0,
        };

        e.as_contract(&backstop, || {
            storage::set_backfill_status(&e, &false);
            // like distribute was called on previous block
            storage::set_last_distribution_time(&e, &(&emitter_distro_time - 5));
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_pool_balance(
                &e,
                &pool_1,
                &PoolBalance {
                    tokens: 10_000_0000000,
                    shares: 10_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_2,
                // 500_000_0000000 unqueued tokens
                &PoolBalance {
                    tokens: 1_000_000_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 100_000_0000000,
                },
            );
            storage::set_rz_emis(&e, &pool_2, &start_pool_2_accrued);
            storage::set_pool_balance(
                &e,
                &pool_3,
                // 1_000_000_000_0000000 unqueued tokens
                &PoolBalance {
                    tokens: 2_000_000_000_0000000,
                    shares: 1_200_000_0000000,
                    q4w: 600_000_0000000,
                },
            );
            storage::set_rz_emis(&e, &pool_3, &start_pool_3_accrued);

            distribute(&e);

            let last_distro_time = storage::get_last_distribution_time(&e);
            assert_eq!(last_distro_time, emitter_distro_time);
            let backfilled_emissions = storage::get_backfill_emissions(&e);
            assert_eq!(backfilled_emissions, 0);

            let pool_1_accrued = storage::get_rz_emis(&e, &pool_1);
            assert_eq!(pool_1_accrued.accrued, 330 + 0);
            assert_eq!(pool_1_accrued.last_time, 0);
            let pool_2_accrued = storage::get_rz_emis(&e, &pool_2);
            assert_eq!(
                pool_2_accrued.accrued,
                1_6666555 + start_pool_2_accrued.accrued
            );
            assert_eq!(pool_2_accrued.last_time, start_pool_2_accrued.last_time);
            let pool_3_accrued = storage::get_rz_emis(&e, &pool_3);
            assert_eq!(
                pool_3_accrued.accrued,
                3_3333110 + start_pool_3_accrued.accrued
            );
            assert_eq!(pool_3_accrued.last_time, start_pool_3_accrued.last_time);

            // backfill status remains false
            let backfill_status = storage::get_backfill_status(&e);
            assert_eq!(backfill_status, Some(false));
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1000)")]
    fn test_distribute_empty_rz() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let backstop = create_backstop(&e);
        let emitter_distro_time = 1713139200 - 10;
        create_emitter(
            &e,
            &backstop,
            &Address::generate(&e),
            &Address::generate(&e),
            emitter_distro_time,
        );

        let pool_1 = Address::generate(&e);

        let reward_zone: Vec<Address> = vec![&e];

        e.as_contract(&backstop, || {
            storage::set_last_distribution_time(&e, &(emitter_distro_time - (60 * 60 * 24)));
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_pool_balance(
                &e,
                &pool_1,
                &PoolBalance {
                    tokens: 300_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 0,
                },
            );

            distribute(&e);
        });
    }

    #[test]
    fn test_distribute_no_last_dist_time() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let backstop = create_backstop(&e);
        let emitter_distro_time = 1713139200 - 10;
        create_emitter(
            &e,
            &backstop,
            &Address::generate(&e),
            &Address::generate(&e),
            emitter_distro_time,
        );

        let pool_1 = Address::generate(&e);
        let pool_2 = Address::generate(&e);
        let pool_3 = Address::generate(&e);
        let reward_zone: Vec<Address> = vec![&e, pool_1.clone(), pool_2.clone(), pool_3.clone()];

        e.as_contract(&backstop, || {
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_pool_balance(
                &e,
                &pool_1,
                &PoolBalance {
                    tokens: 300_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_2,
                &PoolBalance {
                    tokens: 200_000_0000000,
                    shares: 150_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_3,
                &PoolBalance {
                    tokens: 500_000_0000000,
                    shares: 600_000_0000000,
                    q4w: 0,
                },
            );

            let new_emissions = distribute(&e);

            assert_eq!(new_emissions, 0);
            let last_distro_time = storage::get_last_distribution_time(&e);
            assert_eq!(last_distro_time, emitter_distro_time);
            let pool_1_accrued_1 = storage::get_rz_emis(&e, &pool_1);
            assert_eq!(pool_1_accrued_1.accrued, 0);
            assert_eq!(pool_1_accrued_1.last_time, 0);
            let pool_2_accrued_1 = storage::get_rz_emis(&e, &pool_2);
            assert_eq!(pool_2_accrued_1.accrued, 0);
            assert_eq!(pool_2_accrued_1.last_time, 0);
            let pool_3_accrued_1 = storage::get_rz_emis(&e, &pool_3);
            assert_eq!(pool_3_accrued_1.accrued, 0);
            assert_eq!(pool_3_accrued_1.last_time, 0);

            // sets backfill status to false
            let backfill_status = storage::get_backfill_status(&e);
            assert_eq!(backfill_status, Some(false));
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1000)")]
    fn test_distribute_under_5_block_time_panics() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let backstop = create_backstop(&e);
        let emitter_distro_time = 1713139200 - 5;
        create_emitter(
            &e,
            &backstop,
            &Address::generate(&e),
            &Address::generate(&e),
            emitter_distro_time,
        );

        let pool_1 = Address::generate(&e);
        let pool_2 = Address::generate(&e);
        let pool_3 = Address::generate(&e);
        let reward_zone: Vec<Address> = vec![&e, pool_1.clone(), pool_2.clone(), pool_3.clone()];

        e.as_contract(&backstop, || {
            storage::set_backfill_status(&e, &false);
            storage::set_last_distribution_time(&e, &(emitter_distro_time - 4));
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_pool_balance(
                &e,
                &pool_1,
                // 300_000_0000000 unqueued tokens
                &PoolBalance {
                    tokens: 300_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_2,
                // 200_000_0000000 unqueued tokens
                &PoolBalance {
                    tokens: 400_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 100_000_0000000,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_3,
                // 500_000_0000000 unqueued tokens
                &PoolBalance {
                    tokens: 1_000_000_0000000,
                    shares: 1_200_000_0000000,
                    q4w: 600_000_0000000,
                },
            );

            distribute(&e);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1000)")]
    fn test_distribute_last_distro_panics_errors() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let v1_backstop = create_backstop(&e);
        let backstop = create_backstop(&e);
        let emitter_distro_time = 1713139200 - 10;
        // set backstop to another address to force a panic
        create_emitter(
            &e,
            &v1_backstop,
            &Address::generate(&e),
            &Address::generate(&e),
            emitter_distro_time,
        );

        let pool_1 = Address::generate(&e);
        let pool_2 = Address::generate(&e);
        let pool_3 = Address::generate(&e);
        let reward_zone: Vec<Address> = vec![&e, pool_1.clone(), pool_2.clone(), pool_3.clone()];

        let start_pool_2_accrued = RzEmissions {
            accrued: 1_0000001,
            last_time: 123,
        };
        let start_pool_3_accrued = RzEmissions {
            accrued: 20_000_0000000,
            last_time: 0,
        };

        e.as_contract(&backstop, || {
            storage::set_backfill_status(&e, &false);
            storage::set_last_distribution_time(&e, &(emitter_distro_time - (60 * 60 * 24)));
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_pool_balance(
                &e,
                &pool_1,
                // 300_000_0000000 unqueued tokens
                &PoolBalance {
                    tokens: 300_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_2,
                // 200_000_0000000 unqueued tokens
                &PoolBalance {
                    tokens: 400_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 100_000_0000000,
                },
            );
            storage::set_rz_emis(&e, &pool_2, &start_pool_2_accrued);
            storage::set_pool_balance(
                &e,
                &pool_3,
                // 500_000_0000000 unqueued tokens
                &PoolBalance {
                    tokens: 1_000_000_0000000,
                    shares: 1_200_000_0000000,
                    q4w: 600_000_0000000,
                },
            );
            storage::set_rz_emis(&e, &pool_3, &start_pool_3_accrued);

            distribute(&e);
        });
    }

    #[test]
    fn test_distribute_backfill_emissions() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let v1_backstop = create_backstop(&e);
        let backstop = create_backstop(&e);
        let emitter_distro_time = 1713139200 - 1000;
        create_emitter(
            &e,
            &v1_backstop,
            &Address::generate(&e),
            &Address::generate(&e),
            emitter_distro_time,
        );

        let pool_1 = Address::generate(&e);
        let pool_2 = Address::generate(&e);
        let pool_3 = Address::generate(&e);
        let reward_zone: Vec<Address> = vec![&e, pool_1.clone(), pool_2.clone(), pool_3.clone()];
        let start_backfilled_emissions = 1_000_000 * SCALAR_7;
        let start_pool_2_accrued = RzEmissions {
            accrued: 1_0000001,
            last_time: 123,
        };
        let start_pool_3_accrued = RzEmissions {
            accrued: 20_000_0000000,
            last_time: 0,
        };

        e.as_contract(&backstop, || {
            storage::set_backfill_status(&e, &true);
            storage::set_backfill_emissions(&e, &start_backfilled_emissions);
            storage::set_last_distribution_time(&e, &(1713139200 - (60 * 60 * 24)));
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_pool_balance(
                &e,
                &pool_1,
                &PoolBalance {
                    tokens: 300_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_2,
                &PoolBalance {
                    tokens: 200_000_0000000,
                    shares: 150_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_rz_emis(&e, &pool_2, &start_pool_2_accrued);
            storage::set_pool_balance(
                &e,
                &pool_3,
                &PoolBalance {
                    tokens: 500_000_0000000,
                    shares: 600_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_rz_emis(&e, &pool_3, &start_pool_3_accrued);

            distribute(&e);

            let last_distro_time = storage::get_last_distribution_time(&e);
            assert_eq!(last_distro_time, e.ledger().timestamp());
            let backfilled_emissions = storage::get_backfill_emissions(&e);
            assert_eq!(
                backfilled_emissions,
                start_backfilled_emissions + (60 * 60 * 24) * SCALAR_7
            );
            let is_backfill = storage::get_backfill_status(&e);
            assert_eq!(is_backfill, Some(true));

            let pool_1_accrued = storage::get_rz_emis(&e, &pool_1);
            assert_eq!(pool_1_accrued.accrued, 25_920_0000000 + 0);
            assert_eq!(pool_1_accrued.last_time, 0);
            let pool_2_accrued = storage::get_rz_emis(&e, &pool_2);
            assert_eq!(
                pool_2_accrued.accrued,
                17_280_0000000 + start_pool_2_accrued.accrued
            );
            assert_eq!(pool_2_accrued.last_time, start_pool_2_accrued.last_time);
            let pool_3_accrued = storage::get_rz_emis(&e, &pool_3);
            assert_eq!(
                pool_3_accrued.accrued,
                43_200_0000000 + start_pool_3_accrued.accrued
            );
            assert_eq!(pool_3_accrued.last_time, start_pool_3_accrued.last_time);
        });
    }

    #[test]
    fn test_distribute_backfill_emissions_first_call() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let v1_backstop = create_backstop(&e);
        let backstop = create_backstop(&e);
        let emitter_distro_time = 1713139200 - 10;
        create_emitter(
            &e,
            &v1_backstop,
            &Address::generate(&e),
            &Address::generate(&e),
            emitter_distro_time,
        );

        let pool_1 = Address::generate(&e);
        let pool_2 = Address::generate(&e);
        let pool_3 = Address::generate(&e);
        let reward_zone: Vec<Address> = vec![&e, pool_1.clone(), pool_2.clone(), pool_3.clone()];

        e.as_contract(&backstop, || {
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_pool_balance(
                &e,
                &pool_1,
                &PoolBalance {
                    tokens: 300_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_2,
                &PoolBalance {
                    tokens: 200_000_0000000,
                    shares: 150_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_3,
                &PoolBalance {
                    tokens: 500_000_0000000,
                    shares: 600_000_0000000,
                    q4w: 0,
                },
            );

            distribute(&e);

            let last_distro_time = storage::get_last_distribution_time(&e);
            assert_eq!(last_distro_time, e.ledger().timestamp());
            let backfilled_emissions = storage::get_backfill_emissions(&e);
            assert_eq!(backfilled_emissions, 0);
            let is_backfill = storage::get_backfill_status(&e);
            assert_eq!(is_backfill, Some(true));
            let pool_1_accrued = storage::get_rz_emis(&e, &pool_1);
            assert_eq!(pool_1_accrued.accrued, 0);
            assert_eq!(pool_1_accrued.last_time, 0);
            let pool_2_accrued = storage::get_rz_emis(&e, &pool_2);
            assert_eq!(pool_2_accrued.accrued, 0);
            assert_eq!(pool_2_accrued.last_time, 0);
            let pool_3_accrued = storage::get_rz_emis(&e, &pool_3);
            assert_eq!(pool_3_accrued.accrued, 0);
            assert_eq!(pool_3_accrued.last_time, 0);
        });
    }

    #[test]
    fn test_distribute_backfill_emissions_distributes_at_most_max() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let v1_backstop = create_backstop(&e);
        let backstop = create_backstop(&e);
        let emitter_distro_time = 1713139200 - 10;
        create_emitter(
            &e,
            &v1_backstop,
            &Address::generate(&e),
            &Address::generate(&e),
            emitter_distro_time,
        );

        let pool_1 = Address::generate(&e);
        let pool_2 = Address::generate(&e);
        let pool_3 = Address::generate(&e);
        let reward_zone: Vec<Address> = vec![&e, pool_1.clone(), pool_2.clone(), pool_3.clone()];
        let start_backfilled_emissions = MAX_BACKFILLED_EMISSIONS - (60 * 60 * 24) * SCALAR_7;

        e.as_contract(&backstop, || {
            storage::set_backfill_emissions(&e, &start_backfilled_emissions);
            storage::set_last_distribution_time(&e, &(emitter_distro_time - 60 * 60 * 30));
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_pool_balance(
                &e,
                &pool_1,
                &PoolBalance {
                    tokens: 300_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 200_000_0000000,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_2,
                &PoolBalance {
                    // 400k non-q4w, 40%
                    tokens: 500_000_0000000,
                    shares: 400_000_0000000,
                    q4w: 80_000_0000000,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_3,
                &PoolBalance {
                    tokens: 600_000_0000000,
                    shares: 700_000_0000000,
                    q4w: 0,
                },
            );

            distribute(&e);
            let last_distro_time = storage::get_last_distribution_time(&e);
            assert_eq!(last_distro_time, e.ledger().timestamp());
            let backfilled_emissions = storage::get_backfill_emissions(&e);
            assert_eq!(backfilled_emissions, MAX_BACKFILLED_EMISSIONS);
            let is_backfill = storage::get_backfill_status(&e);
            assert_eq!(is_backfill, Some(true));

            let pool_1_accrued = storage::get_rz_emis(&e, &pool_1);
            assert_eq!(pool_1_accrued.accrued, 0);
            assert_eq!(pool_1_accrued.last_time, 0);
            let pool_2_accrued = storage::get_rz_emis(&e, &pool_2);
            assert_eq!(pool_2_accrued.accrued, 34_560_0000000);
            assert_eq!(pool_2_accrued.last_time, 0);
            let pool_3_accrued = storage::get_rz_emis(&e, &pool_3);
            assert_eq!(pool_3_accrued.accrued, 51_840_0000000);
            assert_eq!(pool_3_accrued.last_time, 0);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1010)")]
    fn test_distribute_backfill_emissions_at_max_panics() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let v1_backstop = create_backstop(&e);
        let backstop = create_backstop(&e);
        let emitter_distro_time = 1713139200 - 10;
        create_emitter(
            &e,
            &v1_backstop,
            &Address::generate(&e),
            &Address::generate(&e),
            emitter_distro_time,
        );

        let pool_1 = Address::generate(&e);
        let pool_2 = Address::generate(&e);
        let pool_3 = Address::generate(&e);
        let reward_zone: Vec<Address> = vec![&e, pool_1.clone(), pool_2.clone(), pool_3.clone()];
        let start_backfilled_emissions = MAX_BACKFILLED_EMISSIONS;

        e.as_contract(&backstop, || {
            storage::set_backfill_emissions(&e, &start_backfilled_emissions);
            storage::set_last_distribution_time(&e, &(emitter_distro_time - 60 * 60 * 24));
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_pool_balance(
                &e,
                &pool_1,
                &PoolBalance {
                    tokens: 300_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 200_000_0000000,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_2,
                &PoolBalance {
                    // 400k non-q4w, 40%
                    tokens: 500_000_0000000,
                    shares: 400_000_0000000,
                    q4w: 80_000_0000000,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_3,
                &PoolBalance {
                    tokens: 600_000_0000000,
                    shares: 700_000_0000000,
                    q4w: 0,
                },
            );

            distribute(&e);
        });
    }

    #[test]
    fn test_distribute_backfill_emissions_over_needs_reset() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let backstop = create_backstop(&e);
        let emitter_distro_time = 1713139200 - 10;
        create_emitter(
            &e,
            &backstop,
            &Address::generate(&e),
            &Address::generate(&e),
            emitter_distro_time,
        );

        let pool_1 = Address::generate(&e);
        let pool_2 = Address::generate(&e);
        let pool_3 = Address::generate(&e);
        let reward_zone: Vec<Address> = vec![&e, pool_1.clone(), pool_2.clone(), pool_3.clone()];
        let start_backfilled_emissions = 1_000_000 * SCALAR_7;
        let last_distro_time = 1713139200 - 10000;

        let start_pool_2_accrued = RzEmissions {
            accrued: 1_0000001,
            last_time: 123,
        };
        let start_pool_3_accrued = RzEmissions {
            accrued: 20_000_0000000,
            last_time: 0,
        };

        e.as_contract(&backstop, || {
            storage::set_backfill_status(&e, &true);
            storage::set_backfill_emissions(&e, &start_backfilled_emissions);
            storage::set_last_distribution_time(&e, &last_distro_time);
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_pool_balance(
                &e,
                &pool_1,
                &PoolBalance {
                    tokens: 300_000_0000000,
                    shares: 200_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_pool_balance(
                &e,
                &pool_2,
                &PoolBalance {
                    tokens: 200_000_0000000,
                    shares: 150_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_rz_emis(&e, &pool_2, &start_pool_2_accrued);
            storage::set_pool_balance(
                &e,
                &pool_3,
                &PoolBalance {
                    tokens: 500_000_0000000,
                    shares: 600_000_0000000,
                    q4w: 0,
                },
            );
            storage::set_rz_emis(&e, &pool_3, &start_pool_3_accrued);

            distribute(&e);

            let last_distro_time = storage::get_last_distribution_time(&e);
            assert_eq!(last_distro_time, emitter_distro_time);
            let backfilled_emissions = storage::get_backfill_emissions(&e);
            assert_eq!(backfilled_emissions, start_backfilled_emissions);
            let is_backfill = storage::get_backfill_status(&e);
            assert_eq!(is_backfill, Some(false));
            let pool_1_accrued = storage::get_rz_emis(&e, &pool_1);
            assert_eq!(pool_1_accrued.accrued, 0);
            assert_eq!(pool_1_accrued.last_time, 0);
            let pool_2_accrued = storage::get_rz_emis(&e, &pool_2);
            assert_eq!(pool_2_accrued.accrued, start_pool_2_accrued.accrued);
            assert_eq!(pool_2_accrued.last_time, start_pool_2_accrued.last_time);
            let pool_3_accrued = storage::get_rz_emis(&e, &pool_3);
            assert_eq!(pool_3_accrued.accrued, start_pool_3_accrued.accrued);
            assert_eq!(pool_3_accrued.last_time, start_pool_3_accrued.last_time);
        });
    }

    /********** add_to_reward_zone **********/

    #[test]
    fn test_add_to_rz_empty_adds_pool() {
        let e = Env::default();
        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            base_reserve: 10,
            network_id: Default::default(),
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let backstop_id = create_backstop(&e);
        let to_add = Address::generate(&e);

        let (blnd_id, _) = create_blnd_token(&e, &backstop_id, &bombadil);
        let (usdc_id, _) = create_usdc_token(&e, &backstop_id, &bombadil);
        create_comet_lp_pool_with_tokens_per_share(
            &e,
            &backstop_id,
            &bombadil,
            &blnd_id,
            5_0000000,
            &usdc_id,
            0_1000000,
        );

        e.as_contract(&backstop_id, || {
            storage::set_last_distribution_time(&e, &0);
            storage::set_pool_balance(
                &e,
                &to_add,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_000_0000000,
                    q4w: 1_000_0000000,
                },
            );

            add_to_reward_zone(&e, to_add.clone(), None);
            let actual_rz = storage::get_reward_zone(&e);
            let expected_rz: Vec<Address> = vec![&e, to_add];
            assert_eq!(actual_rz, expected_rz);
        });
    }

    #[test]
    fn test_add_to_rz_before_max() {
        let e = Env::default();
        e.ledger().set(LedgerInfo {
            timestamp: 1713139200 - 100000,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let backstop_id = create_backstop(&e);
        let to_add = Address::generate(&e);

        let (blnd_id, _) = create_blnd_token(&e, &backstop_id, &bombadil);
        let (usdc_id, _) = create_usdc_token(&e, &backstop_id, &bombadil);
        create_comet_lp_pool_with_tokens_per_share(
            &e,
            &backstop_id,
            &bombadil,
            &blnd_id,
            5_0000000,
            &usdc_id,
            0_1000000,
        );
        let mut reward_zone: Vec<Address> = vec![
            &e,
            Address::generate(&e),
            Address::generate(&e),
            Address::generate(&e),
            Address::generate(&e),
            Address::generate(&e),
            Address::generate(&e),
            Address::generate(&e),
            Address::generate(&e),
            Address::generate(&e),
        ];

        e.as_contract(&backstop_id, || {
            storage::set_last_distribution_time(&e, &(1713139200 - 100));
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_pool_balance(
                &e,
                &to_add,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_000_0000000,
                    q4w: 1_000_0000000,
                },
            );

            add_to_reward_zone(&e, to_add.clone(), None);
            let actual_rz = storage::get_reward_zone(&e);
            reward_zone.push_front(to_add);
            assert_eq!(actual_rz, reward_zone);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1002)")]
    fn test_add_to_rz_empty_pool_under_backstop_threshold() {
        let e = Env::default();
        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            base_reserve: 10,
            network_id: Default::default(),
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let backstop_id = create_backstop(&e);
        let to_add = Address::generate(&e);

        let (blnd_id, _) = create_blnd_token(&e, &backstop_id, &bombadil);
        let (usdc_id, _) = create_usdc_token(&e, &backstop_id, &bombadil);
        create_comet_lp_pool_with_tokens_per_share(
            &e,
            &backstop_id,
            &bombadil,
            &blnd_id,
            5_0000000,
            &usdc_id,
            0_1000000,
        );

        e.as_contract(&backstop_id, || {
            storage::set_last_distribution_time(&e, &(1713139200 - 100));
            storage::set_pool_balance(
                &e,
                &to_add,
                &PoolBalance {
                    shares: 30_000_0000000,
                    tokens: 40_000_0000000,
                    q4w: 1_000_0000000,
                },
            );
            // storage::set_lp_token_val(&e, &(5_0000000, 0_1000000));

            add_to_reward_zone(&e, to_add.clone(), None);
            let actual_rz = storage::get_reward_zone(&e);
            let expected_rz: Vec<Address> = vec![&e, to_add];
            assert_eq!(actual_rz, expected_rz);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1009)")]
    fn test_add_to_rz_respects_max_size() {
        let e = Env::default();
        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let backstop_id = create_backstop(&e);
        let to_add = Address::generate(&e);

        let (blnd_id, _) = create_blnd_token(&e, &backstop_id, &bombadil);
        let (usdc_id, _) = create_usdc_token(&e, &backstop_id, &bombadil);
        create_comet_lp_pool_with_tokens_per_share(
            &e,
            &backstop_id,
            &bombadil,
            &blnd_id,
            5_0000000,
            &usdc_id,
            0_1000000,
        );
        let mut reward_zone: Vec<Address> = vec![&e];
        for _ in 0..30 {
            reward_zone.push_back(Address::generate(&e));
        }
        e.as_contract(&backstop_id, || {
            storage::set_last_distribution_time(&e, &(1713139200 - 100));
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_pool_balance(
                &e,
                &to_add,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_000_0000000,
                    q4w: 1_000_0000000,
                },
            );

            assert!(reward_zone.len() == 30);

            // This should fail due to the reward zone being full and not having a pool to remove
            add_to_reward_zone(&e, to_add.clone(), None);
        });
    }

    #[test]
    fn test_add_to_rz_swap_happy_path() {
        let e = Env::default();
        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let backstop_id = create_backstop(&e);
        create_blnd_token(&e, &backstop_id, &Address::generate(&e));
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let backstop_id = create_backstop(&e);
        let to_add = Address::generate(&e);
        let to_remove = Address::generate(&e);

        let (blnd_id, _) = create_blnd_token(&e, &backstop_id, &bombadil);
        let (usdc_id, _) = create_usdc_token(&e, &backstop_id, &bombadil);
        create_comet_lp_pool_with_tokens_per_share(
            &e,
            &backstop_id,
            &bombadil,
            &blnd_id,
            5_0000000,
            &usdc_id,
            0_1000000,
        );
        let mut reward_zone: Vec<Address> = vec![&e];
        for _ in 0..30 {
            reward_zone.push_back(Address::generate(&e));
        }
        reward_zone.set(7, to_remove.clone());

        e.as_contract(&backstop_id, || {
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_last_distribution_time(&e, &(1713139200 - 100));
            storage::set_pool_balance(
                &e,
                &to_add,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_001_0000000,
                    q4w: 1_000_0000000,
                },
            );
            storage::set_pool_balance(
                &e,
                &to_remove,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_000_0000000,
                    q4w: 1_000_0000000,
                },
            );
            storage::set_backstop_emis_data(
                &e,
                &to_remove,
                &BackstopEmissionData {
                    eps: 0_10000000000000,
                    expiration: 1713139200 + 1000,
                    index: 0,
                    last_time: 1713139200 - 12345,
                },
            );
            add_to_reward_zone(&e, to_add.clone(), Some(to_remove.clone()));
            let actual_rz = storage::get_reward_zone(&e);
            assert_eq!(actual_rz.len(), 30);
            reward_zone.remove(7);
            reward_zone.push_front(to_add.clone());
            assert_eq!(actual_rz, reward_zone);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1002)")]
    fn test_add_to_rz_swap_not_enough_tokens() {
        let e = Env::default();
        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let backstop_id = create_backstop(&e);
        let to_add = Address::generate(&e);
        let to_remove = Address::generate(&e);

        let (blnd_id, _) = create_blnd_token(&e, &backstop_id, &bombadil);
        let (usdc_id, _) = create_usdc_token(&e, &backstop_id, &bombadil);
        create_comet_lp_pool_with_tokens_per_share(
            &e,
            &backstop_id,
            &bombadil,
            &blnd_id,
            5_0000000,
            &usdc_id,
            0_1000000,
        );
        let mut reward_zone: Vec<Address> = vec![&e];
        for _ in 0..30 {
            reward_zone.push_back(Address::generate(&e));
        }
        reward_zone.set(7, to_remove.clone());

        e.as_contract(&backstop_id, || {
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_last_distribution_time(&e, &(1713139200 - 60 * 60));
            storage::set_pool_balance(
                &e,
                &to_add,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_000_0000000,
                    q4w: 1_000_0000000,
                },
            );
            storage::set_pool_balance(
                &e,
                &to_remove,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_000_0000000,
                    q4w: 1_000_0000000,
                },
            );

            add_to_reward_zone(&e, to_add.clone(), Some(to_remove));
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1000)")]
    fn test_add_to_rz_swap_distribution_too_long_ago() {
        let e = Env::default();
        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let backstop_id = create_backstop(&e);
        let to_add = Address::generate(&e);
        let to_remove = Address::generate(&e);

        let (blnd_id, _) = create_blnd_token(&e, &backstop_id, &bombadil);
        let (usdc_id, _) = create_usdc_token(&e, &backstop_id, &bombadil);
        create_comet_lp_pool_with_tokens_per_share(
            &e,
            &backstop_id,
            &bombadil,
            &blnd_id,
            5_0000000,
            &usdc_id,
            0_1000000,
        );
        let mut reward_zone: Vec<Address> = vec![&e];
        for _ in 0..30 {
            reward_zone.push_back(Address::generate(&e));
        }
        reward_zone.set(7, to_remove.clone());

        e.as_contract(&backstop_id, || {
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_last_distribution_time(&e, &(1713139200 - 60 * 60 - 1));
            storage::set_pool_balance(
                &e,
                &to_add,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_001_0000000,
                    q4w: 1_000_0000000,
                },
            );
            storage::set_pool_balance(
                &e,
                &to_remove,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_000_0000000,
                    q4w: 1_000_0000000,
                },
            );

            add_to_reward_zone(&e, to_add.clone(), Some(to_remove));
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1002)")]
    fn test_add_to_rz_to_remove_not_in_rz() {
        let e = Env::default();
        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let backstop_id = create_backstop(&e);
        let to_add = Address::generate(&e);
        let to_remove = Address::generate(&e);

        let (blnd_id, _) = create_blnd_token(&e, &backstop_id, &bombadil);
        let (usdc_id, _) = create_usdc_token(&e, &backstop_id, &bombadil);
        create_comet_lp_pool_with_tokens_per_share(
            &e,
            &backstop_id,
            &bombadil,
            &blnd_id,
            5_0000000,
            &usdc_id,
            0_1000000,
        );
        let mut reward_zone: Vec<Address> = vec![&e];
        for _ in 0..30 {
            reward_zone.push_back(Address::generate(&e));
        }

        e.as_contract(&backstop_id, || {
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_last_distribution_time(&e, &(1713139200 - 60 * 60));
            storage::set_pool_balance(
                &e,
                &to_add,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_001_0000000,
                    q4w: 1_000_0000000,
                },
            );
            storage::set_pool_balance(
                &e,
                &to_remove,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_000_0000000,
                    q4w: 1_000_0000000,
                },
            );

            add_to_reward_zone(&e, to_add.clone(), Some(to_remove));
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1000)")]
    fn test_add_to_rz_already_exists_panics() {
        let e = Env::default();
        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let backstop_id = create_backstop(&e);
        let to_add = Address::generate(&e);
        let to_remove = Address::generate(&e);

        let (blnd_id, _) = create_blnd_token(&e, &backstop_id, &bombadil);
        let (usdc_id, _) = create_usdc_token(&e, &backstop_id, &bombadil);
        create_comet_lp_pool_with_tokens_per_share(
            &e,
            &backstop_id,
            &bombadil,
            &blnd_id,
            5_0000000,
            &usdc_id,
            0_1000000,
        );
        let reward_zone: Vec<Address> = vec![
            &e,
            Address::generate(&e),
            to_remove.clone(),
            Address::generate(&e),
            Address::generate(&e),
            Address::generate(&e),
            Address::generate(&e),
            Address::generate(&e),
            to_add.clone(),
            Address::generate(&e),
            Address::generate(&e),
        ];

        e.as_contract(&backstop_id, || {
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_last_distribution_time(&e, &(1713139200 - 60 * 60));
            storage::set_pool_balance(
                &e,
                &to_add,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_001_0000000,
                    q4w: 1_000_0000000,
                },
            );
            storage::set_pool_balance(
                &e,
                &to_remove,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_000_0000000,
                    q4w: 1_000_0000000,
                },
            );

            add_to_reward_zone(&e, to_add.clone(), Some(to_remove.clone()));
        });
    }

    /********** remove_from_reward_zone **********/

    #[test]
    fn test_remove_from_rz() {
        let e = Env::default();
        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let backstop_id = create_backstop(&e);
        let to_remove = Address::generate(&e);

        let (blnd_id, _) = create_blnd_token(&e, &backstop_id, &bombadil);
        let (usdc_id, _) = create_usdc_token(&e, &backstop_id, &bombadil);
        create_comet_lp_pool_with_tokens_per_share(
            &e,
            &backstop_id,
            &bombadil,
            &blnd_id,
            5_0000000,
            &usdc_id,
            0_1000000,
        );
        let mut reward_zone: Vec<Address> = vec![
            &e,
            Address::generate(&e),
            to_remove.clone(), // index 7
        ];

        e.as_contract(&backstop_id, || {
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_last_distribution_time(&e, &(1713139200 - 60 * 60));
            storage::set_pool_balance(
                &e,
                &to_remove,
                &PoolBalance {
                    shares: 90_000_0000000,
                    tokens: 100_001_0000000,
                    q4w: 1_000_0000000,
                },
            );
            storage::set_pool_balance(
                &e,
                &to_remove,
                &PoolBalance {
                    shares: 35_000_0000000,
                    tokens: 40_000_0000000,
                    q4w: 1_000_0000000,
                },
            );
            storage::set_backstop_emis_data(
                &e,
                &to_remove,
                &BackstopEmissionData {
                    eps: 0_10000000000000,
                    expiration: 1713139200 + 1000,
                    index: 0,
                    last_time: 1713139200 - 12345,
                },
            );
            remove_from_reward_zone(&e, to_remove.clone());
            let actual_rz = storage::get_reward_zone(&e);
            reward_zone.remove(1);
            assert_eq!(actual_rz.len(), 1);
            assert_eq!(actual_rz, reward_zone);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1000)")]
    fn test_remove_from_rz_above_threshold() {
        let e = Env::default();
        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let backstop_id = create_backstop(&e);
        let to_remove = Address::generate(&e);

        let (blnd_id, _) = create_blnd_token(&e, &backstop_id, &bombadil);
        let (usdc_id, _) = create_usdc_token(&e, &backstop_id, &bombadil);
        create_comet_lp_pool_with_tokens_per_share(
            &e,
            &backstop_id,
            &bombadil,
            &blnd_id,
            5_0000000,
            &usdc_id,
            0_1000000,
        );
        let reward_zone: Vec<Address> = vec![
            &e,
            Address::generate(&e),
            to_remove.clone(), // index 7
        ];

        e.as_contract(&backstop_id, || {
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_last_distribution_time(&e, &(1713139200 - 60 * 60));
            storage::set_pool_balance(
                &e,
                &to_remove,
                &PoolBalance {
                    shares: 80_000_0000000,
                    tokens: 90_000_0000000,
                    q4w: 1_000_0000000,
                },
            );
            storage::set_backstop_emis_data(
                &e,
                &to_remove,
                &BackstopEmissionData {
                    eps: 0_10000000000000,
                    expiration: 1713139200 + 1000,
                    index: 0,
                    last_time: 1713139200 - 12345,
                },
            );

            remove_from_reward_zone(&e, to_remove.clone());
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1000)")]
    fn test_remove_from_rz_last_distribution_too_long_ago() {
        let e = Env::default();
        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let backstop_id = create_backstop(&e);
        let to_remove = Address::generate(&e);

        let (blnd_id, _) = create_blnd_token(&e, &backstop_id, &bombadil);
        let (usdc_id, _) = create_usdc_token(&e, &backstop_id, &bombadil);
        create_comet_lp_pool_with_tokens_per_share(
            &e,
            &backstop_id,
            &bombadil,
            &blnd_id,
            5_0000000,
            &usdc_id,
            0_1000000,
        );
        let reward_zone: Vec<Address> = vec![
            &e,
            Address::generate(&e),
            to_remove.clone(), // index 7
        ];

        e.as_contract(&backstop_id, || {
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_last_distribution_time(&e, &(1713139200 - 60 * 60 - 1));
            storage::set_pool_balance(
                &e,
                &to_remove,
                &PoolBalance {
                    shares: 80_000_0000000,
                    tokens: 90_000_0000000,
                    q4w: 1_000_0000000,
                },
            );
            storage::set_backstop_emis_data(
                &e,
                &to_remove,
                &BackstopEmissionData {
                    eps: 0_10000000000000,
                    expiration: 1713139200 + 1000,
                    index: 0,
                    last_time: 1713139200 - 12345,
                },
            );

            remove_from_reward_zone(&e, to_remove.clone());
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1002)")]
    fn test_remove_from_rz_not_in_rz() {
        let e = Env::default();
        e.ledger().set(LedgerInfo {
            timestamp: 1713139200,
            protocol_version: 22,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let backstop_id = create_backstop(&e);
        let to_remove = Address::generate(&e);

        let (blnd_id, _) = create_blnd_token(&e, &backstop_id, &bombadil);
        let (usdc_id, _) = create_usdc_token(&e, &backstop_id, &bombadil);
        create_comet_lp_pool_with_tokens_per_share(
            &e,
            &backstop_id,
            &bombadil,
            &blnd_id,
            5_0000000,
            &usdc_id,
            0_1000000,
        );
        let reward_zone: Vec<Address> = vec![&e, Address::generate(&e)];

        e.as_contract(&backstop_id, || {
            storage::set_reward_zone(&e, &reward_zone);
            storage::set_last_distribution_time(&e, &(1713139200 - 60 * 60));
            storage::set_pool_balance(
                &e,
                &to_remove,
                &PoolBalance {
                    shares: 35_000_0000000,
                    tokens: 40_000_0000000,
                    q4w: 1_000_0000000,
                },
            );
            storage::set_backstop_emis_data(
                &e,
                &to_remove,
                &BackstopEmissionData {
                    eps: 0_10000000000000,
                    expiration: 1713139200 + 1000,
                    index: 0,
                    last_time: 1713139200 - 12345,
                },
            );
            remove_from_reward_zone(&e, to_remove.clone());
        });
    }
}
