use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::{get_contract_version, set_contract_version};

use crate::error::ContractError;
use crate::executions::{claim, participate, register_merkle_root, update_config, withdraw};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::queries::{query_config, query_merkle_root, query_state, query_user_state};

// Version info, for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-merkle-airdrop";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let response = crate::executions::instantiate(deps, env, info, msg)?;

    Ok(response)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            admin,
            expired,
            distribution_schedule
        } => update_config(deps, env, info, admin, expired, distribution_schedule),
        ExecuteMsg::RegisterMerkleRoot {
            merkle_root,
            total_amount,
        } => register_merkle_root(
            deps,
            env,
            info,
            merkle_root,
            total_amount,
        ),
        ExecuteMsg::Participate {
            amount,
            proof,
        } => participate(deps, env, info, amount, proof),
        ExecuteMsg::Claim {} => claim(deps, env, info),
        ExecuteMsg::Withdraw { address } => {
            withdraw(deps, env, info, address)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::UserState { address } => to_binary(&query_user_state(deps, env, address)?),
        QueryMsg::MerkleRoot {} => to_binary(&query_merkle_root(deps)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}