#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::Order::Ascending;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};

use cw2::{ensure_from_older_version, set_contract_version};
use cw20::{BalanceResponse, Cw20Coin, Cw20ReceiveMsg, TokenInfoResponse};

use crate::allowances::{
    execute_decrease_allowance, execute_increase_allowance, execute_transfer_from, query_allowance,
};
use crate::enumerable::{query_all_accounts, query_owner_allowances};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{
    PetStakingData, TokenInfo, ALLOWANCES, ALLOWANCES_SPENDER, BALANCES, PET_STAKING_DATA,
    TOKEN_INFO,
};

// version info for migration info
const CONTRACT_NAME: &str = "mypet";
const CONTRACT_VERSION: &str = "1.0.0";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // check valid token info
    msg.validate()?;
    // create initial accounts
    let total_supply = create_accounts(&mut deps, &msg.initial_balances)?;
    let minter = deps.api.addr_validate(&msg.mint)?;
    // store token info
    let data = TokenInfo {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        total_supply,
        mint: minter,
    };
    TOKEN_INFO.save(deps.storage, &data)?;
    Ok(Response::default())
}

pub fn create_accounts(
    deps: &mut DepsMut,
    accounts: &[Cw20Coin],
) -> Result<Uint128, ContractError> {
    validate_accounts(accounts)?;
    let mut total_supply = Uint128::zero();
    for row in accounts {
        let address = deps.api.addr_validate(&row.address)?;
        BALANCES.save(deps.storage, &address, &row.amount)?;
        total_supply += row.amount;
    }

    Ok(total_supply)
}

pub fn validate_accounts(accounts: &[Cw20Coin]) -> Result<(), ContractError> {
    let mut addresses = accounts.iter().map(|c| &c.address).collect::<Vec<_>>();
    addresses.sort();
    addresses.dedup();

    if addresses.len() != accounts.len() {
        Err(ContractError::DuplicateInitialBalanceAddresses {})
    } else {
        Ok(())
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Transfer { recipient, amount } => {
            execute_transfer(deps, env, info, recipient, amount)
        }
        ExecuteMsg::Mint { recipient, amount } => execute_mint(deps, env, info, recipient, amount),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_increase_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_decrease_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => execute_transfer_from(deps, env, info, owner, recipient, amount),
        ExecuteMsg::UpdateMinter { new_minter } => {
            execute_update_minter(deps, env, info, new_minter)
        }
    }
}

pub fn execute_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let rcpt_addr = deps.api.addr_validate(&recipient)?;

    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let res = Response::new()
        .add_attribute("action", "transfer")
        .add_attribute("from", info.sender)
        .add_attribute("to", recipient)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // lower balance
    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    // reduce total_supply
    TOKEN_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.total_supply = info.total_supply.checked_sub(amount)?;
        Ok(info)
    })?;

    let res = Response::new()
        .add_attribute("action", "burn")
        .add_attribute("from", info.sender)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let mut config = TOKEN_INFO
        .may_load(deps.storage)?
        .ok_or(ContractError::Unauthorized {})?;

    let mut pet_staking_data = PET_STAKING_DATA.may_load(deps.storage)?.map_or(
        PetStakingData::new(&env.block.time),
        |d| {
            if !d.is_valid(&env.block.time) {
                return PetStakingData::new(&env.block.time);
            }
            return d;
        },
    );

    if config.mint.as_ref() != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // update supply and enforce cap
    config.total_supply += amount;
    TOKEN_INFO.save(deps.storage, &config)?;
    pet_staking_data
        .update_amount(&amount)
        .map_err(|_| ContractError::TokenLimitReached {})?;

    // add amount to recipient balance
    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;
    PET_STAKING_DATA.save(deps.storage, &pet_staking_data)?;

    let res = Response::new()
        .add_attribute("action", "mint")
        .add_attribute("to", recipient)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_send(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    let rcpt_addr = deps.api.addr_validate(&contract)?;

    // move the tokens to the contract
    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let res = Response::new()
        .add_attribute("action", "send")
        .add_attribute("from", &info.sender)
        .add_attribute("to", &contract)
        .add_attribute("amount", amount)
        .add_message(
            Cw20ReceiveMsg {
                sender: info.sender.into(),
                amount,
                msg,
            }
            .into_cosmos_msg(contract)?,
        );
    Ok(res)
}

pub fn execute_update_minter(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_minter: String,
) -> Result<Response, ContractError> {
    let mut config = TOKEN_INFO
        .may_load(deps.storage)?
        .ok_or(ContractError::Unauthorized {})?;

    let mint = config.mint.as_ref();
    if mint != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let minter_data = deps.api.addr_validate(&new_minter)?;

    config.mint = minter_data;

    TOKEN_INFO.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "update_minter")
        .add_attribute("new_minter", config.mint))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { address } => to_json_binary(&query_balance(deps, address)?),
        QueryMsg::TokenInfo {} => to_json_binary(&query_token_info(deps)?),
        QueryMsg::Minter {} => to_json_binary(&query_minter(deps)?),
        QueryMsg::Allowance { owner, spender } => {
            to_json_binary(&query_allowance(deps, owner, spender)?)
        }
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_json_binary(&query_owner_allowances(deps, owner, start_after, limit)?),
        QueryMsg::AllAccounts { start_after, limit } => {
            to_json_binary(&query_all_accounts(deps, start_after, limit)?)
        }
        QueryMsg::PetStaking {} => to_json_binary(&query_pet_staking_data(deps)?),
    }
}

pub fn query_balance(deps: Deps, address: String) -> StdResult<BalanceResponse> {
    let address = deps.api.addr_validate(&address)?;
    let balance = BALANCES
        .may_load(deps.storage, &address)?
        .unwrap_or_default();
    Ok(BalanceResponse { balance })
}

pub fn query_token_info(deps: Deps) -> StdResult<TokenInfoResponse> {
    let info = TOKEN_INFO.load(deps.storage)?;
    let res = TokenInfoResponse {
        name: info.name,
        symbol: info.symbol,
        decimals: info.decimals,
        total_supply: info.total_supply,
    };
    Ok(res)
}

pub fn query_minter(deps: Deps) -> StdResult<Addr> {
    let token_info = TOKEN_INFO.load(deps.storage)?;
    Ok(token_info.mint)
}

pub fn query_pet_staking_data(deps: Deps) -> StdResult<PetStakingData> {
    let pet_staking = PET_STAKING_DATA.load(deps.storage)?;
    Ok(pet_staking)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let original_version =
        ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if original_version < "0.14.0".parse::<semver::Version>().unwrap() {
        // Build reverse map of allowances per spender
        let data = ALLOWANCES
            .range(deps.storage, None, None, Ascending)
            .collect::<StdResult<Vec<_>>>()?;
        for ((owner, spender), allowance) in data {
            ALLOWANCES_SPENDER.save(deps.storage, (&spender, &owner), &allowance)?;
        }
    }
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{
        mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info,
    };
    use cosmwasm_std::{coins, from_json, Addr, CosmosMsg, StdError, WasmMsg};

    use super::*;

    fn get_balance<T: Into<String>>(deps: Deps, address: T) -> Uint128 {
        query_balance(deps, address.into()).unwrap().balance
    }

    // this will set up the instantiation for other tests
    fn do_instantiate_with_minter(
        deps: DepsMut,
        addr: &str,
        amount: Uint128,
        minter: &str,
    ) -> TokenInfoResponse {
        _do_instantiate(deps, addr, amount, minter.to_string())
    }

    // this will set up the instantiation for other tests
    fn do_instantiate(deps: DepsMut, addr: &str, amount: Uint128) -> TokenInfoResponse {
        _do_instantiate(deps, addr, amount, "test_minter".to_string())
    }

    // this will set up the instantiation for other tests
    fn _do_instantiate(
        mut deps: DepsMut,
        addr: &str,
        amount: Uint128,
        mint: String,
    ) -> TokenInfoResponse {
        let instantiate_msg = InstantiateMsg {
            name: "Auto Gen".to_string(),
            symbol: "AUTO".to_string(),
            decimals: 3,
            initial_balances: vec![Cw20Coin {
                address: addr.to_string(),
                amount,
            }],
            mint: mint.clone(),
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let res = instantiate(deps.branch(), env, info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let meta = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(
            meta,
            TokenInfoResponse {
                name: "Auto Gen".to_string(),
                symbol: "AUTO".to_string(),
                decimals: 3,
                total_supply: amount,
            }
        );
        assert_eq!(get_balance(deps.as_ref(), addr), amount);
        assert_eq!(query_minter(deps.as_ref()).unwrap(), mint,);
        meta
    }

    mod instantiate {
        use super::*;

        #[test]
        fn basic() {
            let mut deps = mock_dependencies();
            let amount = Uint128::from(11223344u128);
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![Cw20Coin {
                    address: String::from("addr0000"),
                    amount,
                }],
                mint: "test_minter".to_string(),
            };
            let info = mock_info("creator", &[]);
            let env = mock_env();
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());

            assert_eq!(
                query_token_info(deps.as_ref()).unwrap(),
                TokenInfoResponse {
                    name: "Cash Token".to_string(),
                    symbol: "CASH".to_string(),
                    decimals: 9,
                    total_supply: amount,
                }
            );
            assert_eq!(
                get_balance(deps.as_ref(), "addr0000"),
                Uint128::new(11223344)
            );
        }

        #[test]
        fn mintable() {
            let mut deps = mock_dependencies();
            let amount = Uint128::new(11223344);
            let minter = String::from("asmodat");
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![Cw20Coin {
                    address: "addr0000".into(),
                    amount,
                }],
                mint: minter.clone(),
            };
            let info = mock_info("creator", &[]);
            let env = mock_env();
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());

            assert_eq!(
                query_token_info(deps.as_ref()).unwrap(),
                TokenInfoResponse {
                    name: "Cash Token".to_string(),
                    symbol: "CASH".to_string(),
                    decimals: 9,
                    total_supply: amount,
                }
            );
            assert_eq!(
                get_balance(deps.as_ref(), "addr0000"),
                Uint128::new(11223344)
            );
            assert_eq!(query_minter(deps.as_ref()).unwrap(), minter,);
        }

        #[test]
        fn mint_within_limit() {
            let mut deps = mock_dependencies();
            do_instantiate_with_minter(
                deps.as_mut(),
                &String::from("genesis"),
                Uint128::new(1234),
                &String::from("minter"),
            );

            let msg = ExecuteMsg::Mint {
                recipient: String::from("lucky"),
                amount: Uint128::new(9_999_998u128),
            };

            let mut env = mock_env();

            execute(deps.as_mut(), env.clone(), mock_info("minter", &[]), msg).unwrap();
            env.block.time = env.block.time.plus_hours(23u64);

            let msg = ExecuteMsg::Mint {
                recipient: String::from("max"),
                amount: Uint128::new(2u128),
            };
            execute(deps.as_mut(), env.clone(), mock_info("minter", &[]), msg).unwrap();
            env.block.time = env.block.time.plus_minutes(59u64);

            let msg = ExecuteMsg::Mint {
                recipient: String::from("jane"),
                amount: Uint128::new(1u128),
            };
            let res =
                execute(deps.as_mut(), env.clone(), mock_info("minter", &[]), msg).unwrap_err();
            assert_eq!(res, ContractError::TokenLimitReached {})
        }

        #[test]
        fn mint_if_new_date_arrived() {
            let mut deps = mock_dependencies();
            do_instantiate_with_minter(
                deps.as_mut(),
                &String::from("genesis"),
                Uint128::new(1234),
                &String::from("minter"),
            );

            let msg = ExecuteMsg::Mint {
                recipient: String::from("lucky"),
                amount: Uint128::new(9_999_998u128),
            };

            let mut env = mock_env();

            execute(deps.as_mut(), env.clone(), mock_info("minter", &[]), msg).unwrap();
            env.block.time = env.block.time.plus_hours(23u64);

            let msg = ExecuteMsg::Mint {
                recipient: String::from("max"),
                amount: Uint128::new(2u128),
            };
            execute(deps.as_mut(), env.clone(), mock_info("minter", &[]), msg).unwrap();
            env.block.time = env.block.time.plus_minutes(59u64);

            let msg = ExecuteMsg::Mint {
                recipient: String::from("jane"),
                amount: Uint128::new(1u128),
            };
            let res =
                execute(deps.as_mut(), env.clone(), mock_info("minter", &[]), msg).unwrap_err();
            assert_eq!(res, ContractError::TokenLimitReached {});

            let msg = ExecuteMsg::Mint {
                recipient: String::from("peter"),
                amount: Uint128::new(9_999_998u128),
            };
            let mut new_env = mock_env();
            new_env.block.time = new_env.block.time.plus_days(1u64).plus_seconds(1u64);
            execute(deps.as_mut(), new_env.clone(), mock_info("minter", &[]), msg).unwrap();
            let pet_staking_data = query_pet_staking_data(deps.as_ref()).unwrap();
            assert_eq!(pet_staking_data.amount, Uint128::from(2u128));
            assert_eq!(pet_staking_data.start_time, new_env.block.time);
        }

        #[test]
        fn others_cannot_mint() {
            let mut deps = mock_dependencies();
            do_instantiate_with_minter(
                deps.as_mut(),
                &String::from("genesis"),
                Uint128::new(1234),
                &String::from("minter"),
            );

            let msg = ExecuteMsg::Mint {
                recipient: String::from("lucky"),
                amount: Uint128::new(222),
            };
            let info = mock_info("anyone else", &[]);
            let env = mock_env();
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(err, ContractError::Unauthorized {});
        }

        #[test]
        fn minter_can_update_minter_but_not_cap() {
            let mut deps = mock_dependencies();
            let minter = String::from("minter");
            do_instantiate_with_minter(
                deps.as_mut(),
                &String::from("genesis"),
                Uint128::new(1234),
                &minter,
            );

            let new_minter = "new_minter".to_string();
            let msg = ExecuteMsg::UpdateMinter {
                new_minter: new_minter.clone(),
            };

            let info = mock_info(&minter, &[]);
            let env = mock_env();
            let res = execute(deps.as_mut(), env.clone(), info, msg);
            assert!(res.is_ok());
            let query_minter_msg = QueryMsg::Minter {};
            let res = query(deps.as_ref(), env, query_minter_msg);
            let mint: Addr = from_json(&res.unwrap()).unwrap();

            assert!(mint.to_string() == new_minter)
        }

        #[test]
        fn others_cannot_update_minter() {
            let mut deps = mock_dependencies();
            let minter = String::from("minter");
            do_instantiate_with_minter(
                deps.as_mut(),
                &String::from("genesis"),
                Uint128::new(1234),
                &minter,
            );

            let msg = ExecuteMsg::UpdateMinter {
                new_minter: "new_minter".to_string(),
            };

            let info = mock_info("not the minter", &[]);
            let env = mock_env();
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(err, ContractError::Unauthorized {});
        }

        #[test]
        fn instantiate_multiple_accounts() {
            let mut deps = mock_dependencies();
            let amount1 = Uint128::from(11223344u128);
            let addr1 = String::from("addr0001");
            let amount2 = Uint128::from(7890987u128);
            let addr2 = String::from("addr0002");
            let info = mock_info("creator", &[]);
            let env = mock_env();

            // Fails with duplicate addresses
            let instantiate_msg = InstantiateMsg {
                name: "Bash Shell".to_string(),
                symbol: "BASH".to_string(),
                decimals: 6,
                initial_balances: vec![
                    Cw20Coin {
                        address: addr1.clone(),
                        amount: amount1,
                    },
                    Cw20Coin {
                        address: addr1.clone(),
                        amount: amount2,
                    },
                ],
                mint: "test_minter".to_string(),
            };
            let err =
                instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap_err();
            assert_eq!(err, ContractError::DuplicateInitialBalanceAddresses {});

            // Works with unique addresses
            let instantiate_msg = InstantiateMsg {
                name: "Bash Shell".to_string(),
                symbol: "BASH".to_string(),
                decimals: 6,
                initial_balances: vec![
                    Cw20Coin {
                        address: addr1.clone(),
                        amount: amount1,
                    },
                    Cw20Coin {
                        address: addr2.clone(),
                        amount: amount2,
                    },
                ],
                mint: "test_minter".to_string(),
            };
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            assert_eq!(
                query_token_info(deps.as_ref()).unwrap(),
                TokenInfoResponse {
                    name: "Bash Shell".to_string(),
                    symbol: "BASH".to_string(),
                    decimals: 6,
                    total_supply: amount1 + amount2,
                }
            );
            assert_eq!(get_balance(deps.as_ref(), addr1), amount1);
            assert_eq!(get_balance(deps.as_ref(), addr2), amount2);
        }

        #[test]
        fn queries_work() {
            let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
            let addr1 = String::from("addr0001");
            let amount1 = Uint128::from(12340000u128);

            let expected = do_instantiate(deps.as_mut(), &addr1, amount1);

            // check meta query
            let loaded = query_token_info(deps.as_ref()).unwrap();
            assert_eq!(expected, loaded);

            let _info = mock_info("test", &[]);
            let env = mock_env();
            // check balance query (full)
            let data = query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::Balance { address: addr1 },
            )
            .unwrap();
            let loaded: BalanceResponse = from_json(&data).unwrap();
            assert_eq!(loaded.balance, amount1);

            // check balance query (empty)
            let data = query(
                deps.as_ref(),
                env,
                QueryMsg::Balance {
                    address: String::from("addr0002"),
                },
            )
            .unwrap();
            let loaded: BalanceResponse = from_json(&data).unwrap();
            assert_eq!(loaded.balance, Uint128::zero());
        }

        #[test]
        fn transfer() {
            let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
            let addr1 = String::from("addr0001");
            let addr2 = String::from("addr0002");
            let amount1 = Uint128::from(12340000u128);
            let transfer = Uint128::from(76543u128);
            let too_much = Uint128::from(12340321u128);

            do_instantiate(deps.as_mut(), &addr1, amount1);

            // Allows transferring 0
            let info = mock_info(addr1.as_ref(), &[]);
            let env = mock_env();
            let msg = ExecuteMsg::Transfer {
                recipient: addr2.clone(),
                amount: Uint128::zero(),
            };
            execute(deps.as_mut(), env, info, msg).unwrap();

            // cannot send more than we have
            let info = mock_info(addr1.as_ref(), &[]);
            let env = mock_env();
            let msg = ExecuteMsg::Transfer {
                recipient: addr2.clone(),
                amount: too_much,
            };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

            // cannot send from empty account
            let info = mock_info(addr2.as_ref(), &[]);
            let env = mock_env();
            let msg = ExecuteMsg::Transfer {
                recipient: addr1.clone(),
                amount: transfer,
            };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

            // valid transfer
            let info = mock_info(addr1.as_ref(), &[]);
            let env = mock_env();
            let msg = ExecuteMsg::Transfer {
                recipient: addr2.clone(),
                amount: transfer,
            };
            let res = execute(deps.as_mut(), env, info, msg).unwrap();
            assert_eq!(res.messages.len(), 0);

            let remainder = amount1.checked_sub(transfer).unwrap();
            assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
            assert_eq!(get_balance(deps.as_ref(), addr2), transfer);
            assert_eq!(
                query_token_info(deps.as_ref()).unwrap().total_supply,
                amount1
            );
        }
        mod migration {
            use super::*;

            use cosmwasm_std::Empty;
            use cw20::AllAllowancesResponse;
            use cw_multi_test::{App, Contract, ContractWrapper, Executor};
            use cw_utils::Expiration;

            fn cw20_contract() -> Box<dyn Contract<Empty>> {
                let contract = ContractWrapper::new(
                    crate::contract::execute,
                    crate::contract::instantiate,
                    crate::contract::query,
                )
                .with_migrate(crate::contract::migrate);
                Box::new(contract)
            }

            #[test]
            fn test_migrate() {
                let mut app = App::default();

                let cw20_id = app.store_code(cw20_contract());
                let cw20_addr = app
                    .instantiate_contract(
                        cw20_id,
                        Addr::unchecked("sender"),
                        &InstantiateMsg {
                            name: "Token".to_string(),
                            symbol: "TOKEN".to_string(),
                            decimals: 6,
                            initial_balances: vec![Cw20Coin {
                                address: "sender".to_string(),
                                amount: Uint128::new(100),
                            }],
                            mint: "test_minter".to_string(),
                        },
                        &[],
                        "TOKEN",
                        Some("sender".to_string()),
                    )
                    .unwrap();

                // no allowance to start
                let allowance: AllAllowancesResponse = app
                    .wrap()
                    .query_wasm_smart(
                        cw20_addr.to_string(),
                        &QueryMsg::AllAllowances {
                            owner: "sender".to_string(),
                            start_after: None,
                            limit: None,
                        },
                    )
                    .unwrap();
                assert_eq!(allowance, AllAllowancesResponse::default());

                // Set allowance
                let allow1 = Uint128::new(7777);
                let expires = Expiration::AtHeight(123_456);
                let msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cw20_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::IncreaseAllowance {
                        spender: "spender".into(),
                        amount: allow1,
                        expires: Some(expires),
                    })
                    .unwrap(),
                    funds: vec![],
                });
                app.execute(Addr::unchecked("sender"), msg).unwrap();

                // Now migrate
                app.execute(
                    Addr::unchecked("sender"),
                    CosmosMsg::Wasm(WasmMsg::Migrate {
                        contract_addr: cw20_addr.to_string(),
                        new_code_id: cw20_id,
                        msg: to_json_binary(&MigrateMsg {}).unwrap(),
                    }),
                )
                .unwrap();

                // Smoke check that the contract still works.
                let balance: cw20::BalanceResponse = app
                    .wrap()
                    .query_wasm_smart(
                        cw20_addr.clone(),
                        &QueryMsg::Balance {
                            address: "sender".to_string(),
                        },
                    )
                    .unwrap();

                assert_eq!(balance.balance, Uint128::new(100));
            }
        }
    }
}
