use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: Addr,
    pub cw20_token_address: Addr,
    pub expired: Timestamp,
    pub distribution_schedule: Vec<DistributionSchedule>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionSchedule {
    pub start: u64,
    pub end: u64,
    pub percent: Decimal,
}

impl Config {
    pub fn load(storage: &dyn Storage) -> StdResult<Config> {
        CONFIG.load(storage)
    }

    pub fn save(&mut self, storage: &mut dyn Storage) -> StdResult<()> {
        CONFIG.save(storage, self)
    }
}

pub const MERKLE_ROOT_PREFIX: &str = "merkle_root";
pub const MERKLE_ROOT: Map<u8, String> = Map::new(MERKLE_ROOT_PREFIX);

const STATE: Item<State> = Item::new("state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub total_airdrop_amount: Uint128,
    pub total_assigned_amount: Uint128,
    pub total_claimed_amount: Uint128,
}

impl State {
    pub fn load(storage: &dyn Storage) -> State {
        STATE.load(storage).unwrap_or( State {
            total_airdrop_amount: Uint128::zero(),
            total_assigned_amount: Uint128::zero(),
            total_claimed_amount: Uint128::zero(),
        })
    }

    pub fn save(&mut self, storage: &mut dyn Storage) -> StdResult<()> {
        STATE.save(storage, self)
    }
}

const USER_STATE: Map<Addr, UserState> = Map::new("user-state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserState {
    pub user: Addr,
    pub assigned_amount: Uint128,
    pub claimed_amount: Uint128,
    pub last_claimed_time: u64,
}

impl UserState {
    pub fn load(storage: &dyn Storage, user:Addr) -> UserState {
        USER_STATE.load(storage, user.clone()).unwrap_or( UserState {
            user: user.clone(),
            assigned_amount: Uint128::zero(),
            claimed_amount: Uint128::zero(),
            last_claimed_time: 0,
        })
    }

    pub fn save(&mut self, storage: &mut dyn Storage) -> StdResult<()> {
        USER_STATE.save(storage, self.user.clone(), self)
    }

    pub fn calc_claimable_amount(&self, storage:&dyn Storage, time: Timestamp) -> StdResult<Uint128> {
        let now_time_seconds = time.seconds();

        let mut claimable_amount = Uint128::zero();
        let total_amount = self.assigned_amount;

        let config = Config::load(storage)?;
        let mut last_end_time = 0u64;

        for schedule in config.distribution_schedule.iter() {
            let start_time = schedule.start;
            let end_time = schedule.end;

            if start_time <= now_time_seconds && end_time >= self.last_claimed_time {
                let distribution_amount_schedule = total_amount * schedule.percent;

                if start_time == end_time {
                    claimable_amount += distribution_amount_schedule;
                } else {
                    // min(s.1, block_height) - max(s.0, last_distributed)
                    let passed_seconds =
                        std::cmp::min(end_time, now_time_seconds) - std::cmp::max(start_time, self.last_claimed_time);

                    let num_seconds = end_time - start_time;
                    let distribution_amount_per_second: Decimal = Decimal::from_ratio(distribution_amount_schedule, num_seconds);
                    // distribution_amount_per_block = distribution amount of this schedule / blocks count of this schedule.
                    claimable_amount +=
                        distribution_amount_per_second * Uint128::new(passed_seconds as u128);
                }
            }

            last_end_time = end_time;
        }

        if last_end_time <= now_time_seconds {
            Ok(total_amount - self.claimed_amount)
        } else {
            Ok(claimable_amount)
        }
    }
}