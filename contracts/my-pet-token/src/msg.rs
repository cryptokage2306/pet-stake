use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{StdError, StdResult, Uint128};
use cw20::Cw20Coin;
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::PetStakingData;

#[cw_serde]
pub enum ExecuteMsg {
    /// Transfer is a base message to move tokens to another account without triggering actions
    Transfer { recipient: String, amount: Uint128 },
    /// Only with "approval" extension. Allows spender to access an additional amount tokens
    /// from the owner's (env.sender) account. If expires is Some(), overwrites current allowance
    /// expiration with this one.
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Only with "approval" extension. Lowers the spender's access of tokens
    /// from the owner's (env.sender) account by amount. If expires is Some(), overwrites current
    /// allowance expiration with this one.
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Only with "approval" extension. Transfers amount tokens from owner -> recipient
    /// if `env.sender` has sufficient pre-approval.
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
    /// Only with the "mintable" extension. If authorized, creates amount new tokens
    /// and adds to the recipient balance.
    Mint { recipient: String, amount: Uint128 },
    /// Only with the "mintable" extension. The current minter may set
    /// a new minter. Setting the minter to None will remove the
    /// token's minter forever.
    UpdateMinter { new_minter: String },
}

#[cw_serde]
#[cfg_attr(test, derive(Default))]
pub struct InstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Vec<Cw20Coin>,
    pub mint: String,
}

impl InstantiateMsg {
    pub fn validate(&self) -> StdResult<()> {
        // Check name, symbol, decimals
        if !self.has_valid_name() {
            return Err(StdError::generic_err(
                "Name is not in the expected format (3-50 UTF-8 bytes)",
            ));
        }
        if !self.has_valid_symbol() {
            return Err(StdError::generic_err(
                "Ticker symbol is not in expected format [a-zA-Z\\-]{3,12}",
            ));
        }
        if self.decimals > 18 {
            return Err(StdError::generic_err("Decimals must not exceed 18"));
        }
        Ok(())
    }

    fn has_valid_name(&self) -> bool {
        let bytes = self.name.as_bytes();
        if bytes.len() < 3 || bytes.len() > 50 {
            return false;
        }
        true
    }

    fn has_valid_symbol(&self) -> bool {
        let bytes = self.symbol.as_bytes();
        if bytes.len() < 3 || bytes.len() > 12 {
            return false;
        }
        for byte in bytes.iter() {
            if (*byte != 45) && (*byte < 65 || *byte > 90) && (*byte < 97 || *byte > 122) {
                return false;
            }
        }
        true
    }
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the current balance of the given address, 0 if unset.
    #[returns(cw20::BalanceResponse)]
    Balance { address: String },
    /// Returns metadata on the contract - name, decimals, supply, etc.
    #[returns(cw20::TokenInfoResponse)]
    TokenInfo {},
    /// Only with "mintable" extension.
    /// Returns who can mint and the hard cap on maximum tokens after minting.
    #[returns(cw20::MinterResponse)]
    Minter {},
    /// Only with "allowance" extension.
    /// Returns how much spender can use from owner account, 0 if unset.
    #[returns(cw20::AllowanceResponse)]
    Allowance { owner: String, spender: String },
    /// Only with "enumerable" extension (and "allowances")
    /// Returns all allowances this owner has approved. Supports pagination.
    #[returns(cw20::AllAllowancesResponse)]
    AllAllowances {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Only with "enumerable" extension
    /// Returns all accounts that have balances. Supports pagination.
    #[returns(cw20::AllAccountsResponse)]
    AllAccounts {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(PetStakingData)]
    PetStaking {},
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MigrateMsg {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_instantiatemsg_name() {
        // Too short
        let mut msg = InstantiateMsg {
            name: str::repeat("a", 2),
            ..InstantiateMsg::default()
        };
        assert!(!msg.has_valid_name());

        // In the correct length range
        msg.name = str::repeat("a", 3);
        assert!(msg.has_valid_name());

        // Too long
        msg.name = str::repeat("a", 51);
        assert!(!msg.has_valid_name());
    }

    #[test]
    fn validate_instantiatemsg_symbol() {
        // Too short
        let mut msg = InstantiateMsg {
            symbol: str::repeat("a", 2),
            ..InstantiateMsg::default()
        };
        assert!(!msg.has_valid_symbol());

        // In the correct length range
        msg.symbol = str::repeat("a", 3);
        assert!(msg.has_valid_symbol());

        // Too long
        msg.symbol = str::repeat("a", 13);
        assert!(!msg.has_valid_symbol());

        // Has illegal char
        let illegal_chars = [[64u8], [91u8], [123u8]];
        illegal_chars.iter().for_each(|c| {
            let c = std::str::from_utf8(c).unwrap();
            msg.symbol = str::repeat(c, 3);
            assert!(!msg.has_valid_symbol());
        });
    }
}
