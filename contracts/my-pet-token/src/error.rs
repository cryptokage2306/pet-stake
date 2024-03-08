use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Cannot set to own account")]
    CannotSetOwnAccount {},

    // Unused error case. Zero is now treated like every other value.
    #[deprecated(note = "Unused. All zero amount checks have been removed")]
    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Allowance is expired")]
    Expired {},

    #[error("No allowance for this account")]
    NoAllowance {},

    #[error("Minting cannot exceed the cap")]
    CannotExceedCap {},

    #[error("Invalid expiration value")]
    InvalidExpiration {},

    #[error("Duplicate initial balance addresses")]
    DuplicateInitialBalanceAddresses {},

    #[error("Token limit of 1M per day for pet staking has been reached")]
    TokenLimitReached {},
}
