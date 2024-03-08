use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, OverflowError, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

use cw20::AllowanceResponse;

#[cw_serde]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Uint128,
    pub mint: Addr,
}


#[cw_serde]
pub struct PetStakingData {
    pub start_time: Timestamp,
    pub amount: Uint128,
}

impl PetStakingData {
    pub fn new(start_time: &Timestamp) -> Self {
        Self {
            start_time: *start_time,
            amount: Uint128::from(10_000_000u128),
        }
    }

    pub fn is_valid(&self, time: &Timestamp) -> bool {
        // start > time || start + 1 < time
        if self.start_time.gt(time) || self.start_time.plus_days(1u64).lt(time) {
            return false;
        }
        true
    }

    pub fn update_amount(&mut self, amount: &Uint128) -> Result<(), OverflowError> {
        self.amount = self.amount.checked_sub(*amount)?;
        Ok(())
    }
}

pub const TOKEN_INFO: Item<TokenInfo> = Item::new("token_info");
pub const BALANCES: Map<&Addr, Uint128> = Map::new("balance");
pub const ALLOWANCES: Map<(&Addr, &Addr), AllowanceResponse> = Map::new("allowance");
// TODO: After https://github.com/CosmWasm/cw-plus/issues/670 is implemented, replace this with a `MultiIndex` over `ALLOWANCES`
pub const ALLOWANCES_SPENDER: Map<(&Addr, &Addr), AllowanceResponse> =
    Map::new("allowance_spender");
pub const PET_STAKING_DATA: Item<PetStakingData> = Item::new("pet_staking_data");
