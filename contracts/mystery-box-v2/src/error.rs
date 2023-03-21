use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized{},

    #[error("InsufficientAmount")]
    InsufficientAmount{},

    #[error("InvalidRandomness")]
    InvalidRandomness{},

    #[error("PriceInsufficient")]
    PriceInsufficient{},

    #[error("InvalidAddress")]
    InvalidAddress{},
    
    #[error("Uint256OperatorError")]
    Uint256OperatorError{},
    
    #[error("SupplierAlreadyLinked")]
    BoxSupplierAlreadyLinked{},

    #[error("BoxSupplierNotLinked")]
    BoxSupplierNotLinked{},

    #[error("ItemSupplierAlreadyLinked")]
    ItemSupplierAlreadyLinked{},

    #[error("ItemSupplierNotLinked")]
    ItemSupplierNotLinked{},

    #[error("InvalidTokenReplyId")]
    InvalidTokenReplyId{},

    #[error("InvalidEndTime")]
    InvalidEndTime{},

    #[error("MysteryBoxNotStart")]
    MysteryBoxNotStart{},

    #[error("DecimalOperationFail")]
    DecimalOperationFail{},
    
    #[error("MysteryBoxExpired")]
    MysteryBoxExpired{},

    #[error("TokenNotRecognized")]
    TokenNotRecognized{},

    #[error("BoxOpened")]
    BoxOpened{},

    #[error("MysteryBoxNotInitialized")]
    MysteryBoxNotInitialized{},

    #[error("MysteryBoxInitialized")]
    MysteryBoxInOperation{},

    #[error("MysteryBoxNotUpdated")]
    MysteryBoxNotUpdated{},

    #[error("SoldOut")]
    SoldOut{},
    
    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
}