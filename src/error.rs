use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Insufficient funds sent")]
    InsufficientFundsSend {},

    #[error("Auction Ended")]
    AuctionEnded {},

    #[error("Auction Not Ended Yet")]
    AuctionNotEnded {},

    #[error("unregistered minter")]
    UnregisteredMinter {},

    #[error("some of royalty rates are larger than 1")]
    InvalidRoyaltyRate {},
}
