use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, StdResult, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use cw_controllers::{Admin, Claims};
use cw_storage_plus::{Item, Map, SnapshotMap, Strategy};
use cw_utils::Duration;

pub const CLAIMS: Claims = Claims::new("claims");

#[cw_serde]
pub struct Config {
    /// denom of the token to stake
    pub addr: Addr,
}

pub const ADMIN: Admin = Admin::new("admin");
pub const CONFIG: Item<Config> = Item::new("config");
pub const BALANCES: Map<&Addr, Uint128> = Map::new("balance");
pub const TOTAL: Item<Uint128> = Item::new("total_coins");

pub const MEMBERS: SnapshotMap<&Addr, u64> = SnapshotMap::new(
    cw4::MEMBERS_KEY,
    cw4::MEMBERS_CHECKPOINTS,
    cw4::MEMBERS_CHANGELOG,
    Strategy::EveryBlock,
);

pub const STAKE: Map<&Addr, Uint128> = Map::new("stake");

impl Config {
    pub fn new_transfer_from_msg(
        self,
        sender: &Addr,
        recipient: &Addr,
        amount: Uint128,
    ) -> StdResult<CosmosMsg> {
        let msg = to_json_binary(&Cw20ExecuteMsg::TransferFrom {
            owner: sender.into(),
            recipient: recipient.into(),
            amount,
        })?;
        let execute = WasmMsg::Execute {
            contract_addr: self.addr.into(),
            msg,
            funds: vec![],
        };
        Ok(execute.into())
    }

    pub fn new_transfer(self, recipient: &Addr, amount: Uint128) -> StdResult<CosmosMsg> {
        let msg = to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: recipient.into(),
            amount,
        })?;
        let execute = WasmMsg::Execute {
            contract_addr: self.addr.into(),
            msg,
            funds: vec![],
        };
        Ok(execute.into())
    }

    pub fn new_mint(self, recipient: &Addr, amount: Uint128) -> StdResult<CosmosMsg> {
        let msg = to_json_binary(&Cw20ExecuteMsg::Mint {
            recipient: recipient.into(),
            amount,
        })?;
        let execute = WasmMsg::Execute {
            contract_addr: self.addr.into(),
            msg,
            funds: vec![],
        };
        Ok(execute.into())
    }
}
