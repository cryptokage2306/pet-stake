#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, SubMsg, Uint128,
};

use cw2::set_contract_version;
use cw_utils::maybe_addr;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, StakedResponse, TotalStakeResponse};
use crate::state::{Config, ADMIN, BALANCES, CONFIG, TOTAL};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:pet-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let api = deps.api;
    ADMIN.set(deps.branch(), maybe_addr(api, msg.admin)?)?;

    let config = Config { addr: msg.addr };
    CONFIG.save(deps.storage, &config)?;
    TOTAL.save(deps.storage, &Uint128::zero())?;

    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => {
            Ok(ADMIN.execute_update_admin(deps, info, maybe_addr(api, admin)?)?)
        }
        ExecuteMsg::Stake { amount } => stake(deps, env, info, amount),
        ExecuteMsg::Withdraw { amount } => withdraw(deps, env, info, amount),
        ExecuteMsg::Mint { amount } => mint(deps, env, info, amount),
    }
}

pub fn mint(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG
        .may_load(deps.storage)?
        .ok_or(ContractError::Unauthorized {})?;

    let msg = SubMsg::new(config.new_mint(&info.sender, amount)?);
    let res = Response::new()
        .add_submessage(msg)
        .add_attribute("action", "mint")
        .add_attribute("to", info.sender.into_string())
        .add_attribute("amount", amount);

    Ok(res)
}

pub fn withdraw(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let total = TOTAL
        .may_load(deps.storage)?
        .ok_or(ContractError::NoData {})?
        .checked_sub(amount)
        .map_err(|_x| ContractError::NoFunds {})?;

    let config = CONFIG
        .may_load(deps.storage)?
        .ok_or(ContractError::Unauthorized {})?;

    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;

    TOTAL.save(deps.storage, &total)?;

    let msg = SubMsg::new(config.new_transfer(&info.sender, amount)?);
    let res = Response::new()
        .add_submessage(msg)
        .add_attribute("action", "withdraw")
        .add_attribute("to", info.sender.into_string())
        .add_attribute("amount", amount);

    Ok(res)
}

pub fn stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG
        .may_load(deps.storage)?
        .ok_or(ContractError::NoData {})?;

    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_add(amount)?)
        },
    )?;

    let total = TOTAL
        .may_load(deps.storage)?
        .ok_or(ContractError::NoData {})?
        .checked_add(amount)
        .map_err(|_| ContractError::Unauthorized {})?;

    TOTAL.save(deps.storage, &total)?;

    let msg =
        SubMsg::new(config.new_transfer_from_msg(&info.sender, &env.contract.address, amount)?);
    let res = Response::new()
        .add_submessage(msg)
        .add_attribute("action", "stake")
        .add_attribute("to", env.contract.address)
        .add_attribute("amount", amount);

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::TotalStaked {} => to_json_binary(&query_total_stake(deps)?),
        QueryMsg::Staked { address } => to_json_binary(&query_staked(deps, address)?),
        QueryMsg::Admin {} => to_json_binary(&ADMIN.query_admin(deps)?),
    }
}

fn query_total_stake(deps: Deps) -> StdResult<TotalStakeResponse> {
    let total_stake = TOTAL.load(deps.storage)?;
    Ok(TotalStakeResponse { stake: total_stake })
}

pub fn query_staked(deps: Deps, addr: String) -> StdResult<StakedResponse> {
    let address = deps.api.addr_validate(&addr)?;
    let balance = BALANCES
        .may_load(deps.storage, &address)?
        .unwrap_or_default();
    Ok(StakedResponse { stake: balance })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, WasmMsg,
    };
    use cw20::Cw20ExecuteMsg;

    use super::*;

    const INIT_ADMIN: &str = "juan";
    const CW20_ADDRESS: &str = "wasm1234567890";

    fn default_instantiate(deps: DepsMut) {
        do_instantiate(deps)
    }

    fn do_instantiate(deps: DepsMut) {
        let msg = InstantiateMsg {
            addr: Addr::unchecked(CW20_ADDRESS),
            admin: Some(INIT_ADMIN.into()),
        };
        let info = mock_info("creator", &[]);
        instantiate(deps, mock_env(), info, msg).unwrap();
    }

    #[test]
    fn proper_instantiation() {
        let mut deps = mock_dependencies();
        default_instantiate(deps.as_mut());

        // it worked, let's query the state
        let res = ADMIN.query_admin(deps.as_ref()).unwrap();
        assert_eq!(Some(INIT_ADMIN.into()), res.admin);

        let res = query_total_stake(deps.as_ref()).unwrap();
        assert_eq!(Uint128::zero(), res.stake);
    }

    #[test]
    fn mint() {
        let mut deps = mock_dependencies();
        default_instantiate(deps.as_mut());

        let msg = ExecuteMsg::Mint {
            amount: 999_999u128.into(),
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info("mintu", &[]), msg).unwrap();
        assert_eq!(
            res.messages[0],
            SubMsg::new(WasmMsg::Execute {
                contract_addr: CW20_ADDRESS.into(),
                msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                    recipient: "mintu".into(),
                    amount: 999_999u128.into()
                })
                .unwrap(),
                funds: vec![]
            })
        );

        let staked = query_staked(deps.as_ref(), "mintu".into()).unwrap();
        assert_eq!(
            staked,
            StakedResponse {
                stake: Uint128::zero()
            }
        )
    }

    #[test]
    fn staked() {
        let mut deps = mock_dependencies();
        default_instantiate(deps.as_mut());

        let msg = ExecuteMsg::Stake {
            amount: 999_999u128.into(),
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info("mintu", &[]), msg).unwrap();
        assert_eq!(
            res.messages[0],
            SubMsg::new(WasmMsg::Execute {
                contract_addr: CW20_ADDRESS.into(),
                msg: to_json_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "mintu".into(),
                    recipient: mock_env().contract.address.into(),
                    amount: 999_999u128.into()
                })
                .unwrap(),
                funds: vec![]
            })
        );

        let staked = query_staked(deps.as_ref(), "mintu".into()).unwrap();
        assert_eq!(
            staked,
            StakedResponse {
                stake: 999_999u128.into()
            }
        );

        let msg = ExecuteMsg::Stake {
            amount: 1u128.into(),
        };

        execute(deps.as_mut(), mock_env(), mock_info("sam", &[]), msg).unwrap();

        let staked = query_staked(deps.as_ref(), "sam".into()).unwrap();
        assert_eq!(
            staked,
            StakedResponse {
                stake: 1u128.into()
            }
        );

        let total_staked = query_total_stake(deps.as_ref()).unwrap();
        assert_eq!(
            total_staked,
            TotalStakeResponse {
                stake: 1_000_000u128.into()
            }
        )
    }

    #[test]
    fn withdraw() {
        let mut deps = mock_dependencies();
        default_instantiate(deps.as_mut());

        let msg = ExecuteMsg::Stake {
            amount: 999_999u128.into(),
        };

        execute(deps.as_mut(), mock_env(), mock_info("mintu", &[]), msg).unwrap();

        let msg = ExecuteMsg::Withdraw {
            amount: 999u128.into(),
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info("mintu", &[]), msg).unwrap();
        assert_eq!(
            res.messages[0],
            SubMsg::new(WasmMsg::Execute {
                contract_addr: CW20_ADDRESS.into(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "mintu".into(),
                    amount: 999u128.into()
                })
                .unwrap(),
                funds: vec![]
            })
        );

        let staked = query_staked(deps.as_ref(), "mintu".into()).unwrap();
        assert_eq!(
            staked,
            StakedResponse {
                stake: 999_000u128.into()
            }
        );

        let total_staked = query_total_stake(deps.as_ref()).unwrap();
        assert_eq!(
            total_staked,
            TotalStakeResponse {
                stake: 999_000u128.into()
            }
        )
    }
}
