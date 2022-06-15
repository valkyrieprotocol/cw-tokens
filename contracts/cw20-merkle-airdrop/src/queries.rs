use cosmwasm_std::{Deps, Env, StdResult};
use crate::msg::{UserStateResponse};
use crate::state::{Config, MERKLE_ROOT, State, UserState};

pub fn query_config(deps: Deps) -> StdResult<Config> {
    let config = Config::load(deps.storage)?;
    Ok(config)
}

pub fn query_state(deps: Deps) -> StdResult<State> {
    let state = State::load(deps.storage);
    Ok(state)
}

pub fn query_user_state(
    deps: Deps,
    env: Env,
    address: String
) -> StdResult<UserStateResponse> {
    let user_state = UserState::load(deps.storage, deps.api.addr_validate(address.as_str())?);
    let claimable_amount = user_state.calc_claimable_amount(deps.storage, env.block.time)?;

    Ok(UserStateResponse {
        user: user_state.user.to_string(),
        assigned_amount: user_state.assigned_amount,
        claimed_amount: user_state.claimed_amount,
        last_claimed_time: user_state.last_claimed_time,
        claimable_amount,
    })
}

pub fn query_merkle_root(deps: Deps) -> StdResult<String> {
    let stage = 0;
    let merkle_root = MERKLE_ROOT.load(deps.storage, stage)?;
    Ok(merkle_root)
}