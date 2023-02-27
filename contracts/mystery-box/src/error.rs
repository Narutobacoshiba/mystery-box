use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized{},

    #[error("InvalidRandomness")]
    InvalidRandomness{},

    #[error("InvalidAddress")]
    InvalidAddress{},
    
    #[error("Uint256OperatorError")]
    Uint256OperatorError{},
    
    #[error("SupplierAlreadyLinked")]
    SupplierAlreadyLinked{},

    #[error("SupplierNotLinked")]
    SupplierNotLinked{},

    #[error("InvalidTokenReplyId")]
    InvalidTokenReplyId{},

    #[error("InvalidEndTime")]
    InvalidEndTime{},

    #[error("InvalidRarityRate")]
    InvalidRarityRate{},

    #[error("MysteryBoxTimeOut")]
    MysteryBoxTimeOut{},

    #[error("MysteryBoxNotStart")]
    MysteryBoxNotStart{},
    
    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
}