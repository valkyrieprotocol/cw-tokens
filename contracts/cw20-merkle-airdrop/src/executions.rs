use cosmwasm_std::{attr, to_binary, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg, Timestamp, Decimal, StdResult, StdError};
use cw20::Cw20ExecuteMsg;
use cw_utils::{Expiration};
use sha2::Digest;
use std::convert::TryInto;
use std::ops::Add;

use crate::error::ContractError;
use crate::msg::InstantiateMsg;
use crate::state::{Config, MERKLE_ROOT, DistributionSchedule, State, UserState};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    validate_distribution_schedule(&msg.distribution_schedule)?;

    Config {
        admin: info.sender.clone(),
        cw20_token_address: deps.api.addr_validate(&msg.cw20_token_address)?,
        expired: Timestamp::from_seconds(msg.expired),
        distribution_schedule: msg.distribution_schedule.iter()
            .map(|item|
                DistributionSchedule {
                    start: item.0,
                    end: item.1,
                    percent: item.2
                }
            )
            .collect(),
    }.save(deps.storage)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    admin: Option<String>,
    expired: Option<u64>,
    distribution_schedule: Option<Vec<(u64, u64, Decimal)>>,
) -> Result<Response, ContractError> {
    let mut config = Config::load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(admin) = admin {
        config.admin = deps.api.addr_validate(admin.as_str())?
    }

    if let Some(expired) = expired {
        config.expired = Timestamp::from_seconds(expired);
    }

    if let Some(distribution_schedule) = distribution_schedule {
        validate_distribution_schedule(&distribution_schedule)?;

        config.distribution_schedule = distribution_schedule.iter()
            .map(|item|
                DistributionSchedule {
                    start: item.0,
                    end: item.1,
                    percent: item.2
                }
            )
            .collect();
    }

    config.save(deps.storage)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn register_merkle_root(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    merkle_root: String,
    total_amount: Uint128,
) -> Result<Response, ContractError> {
    let config = Config::load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    // check merkle root length
    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(&merkle_root, &mut root_buf)?;

    let stage = 0;
    MERKLE_ROOT.save(deps.storage, stage, &merkle_root)?;

    let mut state = State::load(deps.storage);
    state.total_airdrop_amount = total_amount;
    state.save(deps.storage)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "register_merkle_root"),
        attr("merkle_root", merkle_root),
        attr("total_amount", total_amount),
    ]))
}

pub fn participate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    proof: Vec<String>,
) -> Result<Response, ContractError> {


    // not expired
    let config = Config::load(deps.storage)?;
    let expiration = Expiration::AtTime(config.expired);
    if expiration.is_expired(&env.block) {
        return Err(ContractError::Expired { expiration });
    }

    // verify not claimed
    let user_state = UserState::load(deps.storage, info.sender.clone());
    if user_state.assigned_amount != Uint128::zero() {
        return Err(ContractError::Claimed {});
    }

    let stage:u8 = 0;
    let merkle_root = MERKLE_ROOT.load(deps.storage, stage)?;

    let user_input = format!("{}{}", info.sender, amount);
    let hash = sha2::Sha256::digest(user_input.as_bytes())
        .as_slice()
        .try_into()
        .map_err(|_| ContractError::WrongLength {})?;

    let hash = proof.into_iter().try_fold(hash, |hash, p| {
        let mut proof_buf = [0; 32];
        hex::decode_to_slice(p, &mut proof_buf)?;
        let mut hashes = [hash, proof_buf];
        hashes.sort_unstable();
        sha2::Sha256::digest(&hashes.concat())
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::WrongLength {})
    })?;

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root, &mut root_buf)?;
    if root_buf != hash {
        return Err(ContractError::VerificationFailed {});
    }

    // Update total claimed to reflect
    let mut state = State::load(deps.storage);
    state.total_assigned_amount += amount.clone();
    state.save(deps.storage)?;

    let mut user_state = UserState::load(deps.storage, info.sender.clone());
    user_state.assigned_amount = amount.clone();
    user_state.save(deps.storage)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "execute_claim"),
        attr("user", info.sender.to_string()),
        attr("assigned_amount", amount.to_string()),
    ]))
}

pub fn claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {


    let mut user_state = UserState::load(deps.storage, info.sender.clone());
    let claimed_amount = user_state.calc_claimable_amount(deps.storage, env.block.time)?;

    let mut state = State::load(deps.storage);
    state.total_claimed_amount += claimed_amount;
    state.save(deps.storage)?;

    user_state.claimed_amount += claimed_amount;
    user_state.last_claimed_time = env.block.time.seconds();
    user_state.save(deps.storage)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "execute_withdraw_token"),
        attr("user", info.sender.to_string()),
        attr("claimed_amount", claimed_amount.to_string()),
    ]))
}

pub fn withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    // authorize owner
    let config = Config::load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    // make sure is expired
    let expiration = Expiration::AtTime(config.expired);
    if !expiration.is_expired(&env.block) {
        return Err(ContractError::NotExpired { expiration });
    }

    // Get total amount per stage and total claimed
    let state = State::load(deps.storage);
    let total_amount = state.total_assigned_amount;
    let claimed_amount = state.total_claimed_amount;

    // impossible but who knows
    if claimed_amount > total_amount {
        return Err(ContractError::Unauthorized {});
    }

    // Get balance
    let balance_to_withdraw = total_amount - claimed_amount;

    // Validate address
    let recipient = deps.api.addr_validate(&address)?;

    // Withdraw the tokens and response
    let res = Response::new()
        .add_message(WasmMsg::Execute {
            contract_addr: config.cw20_token_address.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: recipient.into(),
                amount: balance_to_withdraw,
            })?,
        })
        .add_attributes(vec![
            attr("action", "withdraw"),
            attr("address", info.sender),
            attr("amount", balance_to_withdraw),
            attr("recipient", address),
        ]);
    Ok(res)
}

fn validate_distribution_schedule(schedule: &Vec<(u64, u64, Decimal)>) -> StdResult<()> {
    if schedule.len() == 0 {
        return Err(StdError::generic_err("Invalid schedule. schedule is empty."));
    }

    let mut sum_of_ratio = Decimal::zero();
    let mut last_end:u64 = 0;

    for (start, end, ratio) in schedule.iter() {
        if start > end {
            return Err(StdError::generic_err(format!("Invalid schedule. must be start <= end (start: {}, end: {}, ratio: {})", start, end, ratio)));
        }

        if ratio.is_zero() {
            return Err(StdError::generic_err(format!("Invalid schedule. ratio > 0 (start: {}, end: {}, ratio: {})", start, end, ratio)));
        }

        if start.clone() < last_end {
            return Err(StdError::generic_err(format!("Invalid schedule. schedule's start >= previous schedule's end (previous end: {}, start: {})", last_end, start)));
        }

        last_end = end.clone();
        sum_of_ratio = sum_of_ratio.add(ratio.clone());
    }

    if sum_of_ratio != Decimal::one() {
        Err(StdError::generic_err(format!("Sum of ratio must be One(1) (sum: {})", sum_of_ratio)))
    } else {
        Ok(())
    }
}