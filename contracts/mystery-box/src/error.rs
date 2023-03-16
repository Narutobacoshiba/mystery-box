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

    #[error("PriceInsufficient")]
    PriceInsufficient{},

    #[error("ZeroRarity")]
    ZeroRarity{},

    #[error("InvalidAddress")]
    InvalidAddress{},
    
    #[error("Uint256OperatorError")]
    Uint256OperatorError{},
    
    #[error("SupplierAlreadyLinked")]
    BoxSupplierAlreadyLinked{},

    #[error("BoxSupplierNotLinked")]
    BoxSupplierNotLinked{},

    #[error("GiftSupplierAlreadyLinked")]
    GiftSupplierAlreadyLinked{},

    #[error("GiftSupplierNotLinked")]
    GiftSupplierNotLinked{},

    #[error("InvalidTokenReplyId")]
    InvalidTokenReplyId{},

    #[error("InvalidEndTime")]
    InvalidEndTime{},

    #[error("InvalidRarityRate")]
    InvalidRarityRate{},

    #[error("MysteryBoxNotStart")]
    MysteryBoxNotStart{},
    
    #[error("InvalidDecimalFormat")]
    InvalidDecimalFormat{},

    #[error("DecimalOperationFail")]
    DecimalOperationFail{},

    #[error("BoxWithIdNotExist")]
    BoxWithIdNotExist{},

    #[error("SoldOut")]
    SoldOut{},
    
    #[error("MysteryBoxExpired")]
    MysteryBoxExpired{},

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
}