use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
pub use cw_controllers::ClaimsResponse;

#[cw_serde]
pub struct InstantiateMsg {
    /// denom of the token to stake
    pub addr: Addr,

    // admin can only add/remove hooks, not change other parameters
    pub admin: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Stake { amount: Uint128 },
    Withdraw { amount: Uint128 },
    Mint { amount: Uint128 },
    UpdateAdmin { admin: Option<String> },
}

#[cw_serde]
pub enum ReceiveMsg {
    /// Only valid cw20 message is to bond the tokens
    Bond {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Show the number of tokens currently staked by this address.
    #[returns(Uint128)]
    Staked { address: String },
    #[returns(Addr)]
    Admin {},
    #[returns(Uint128)]
    TotalStaked {},
}

#[cw_serde]
pub struct StakedResponse {
    pub stake: Uint128,
}

#[cw_serde]
pub struct TotalStakeResponse {
    pub stake: Uint128,
}
