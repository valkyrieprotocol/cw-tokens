use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Decimal, Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Owner if none set to info.sender.
    pub cw20_token_address: String,
    pub expired: u64,
    pub distribution_schedule: Vec<(u64, u64, Decimal)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig {
        admin: Option<String>,
        expired: Option<u64>,
        distribution_schedule: Option<Vec<(u64, u64, Decimal)>>,
    },
    RegisterMerkleRoot {
        /// MerkleRoot is hex-encoded merkle root.
        merkle_root: String,
        total_amount: Uint128,
    },
    /// Claim does not check if contract has enough funds, owner must ensure it.
    Participate {
        amount: Uint128,
        /// Proof is hex-encoded merkle proof.
        proof: Vec<String>,
    },
    /// Claim does not check if contract has enough funds, owner must ensure it.
    Claim {},
    /// Withdraw the remaining tokens after expire time (only owner)
    Withdraw { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    State {},
    UserState { address: String },
    MerkleRoot {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct UserStateResponse {
    pub user: String,
    pub assigned_amount: Uint128,
    pub claimed_amount: Uint128,
    pub claimable_amount: Uint128,
    pub last_claimed_time: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
