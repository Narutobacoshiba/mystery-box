use cosmwasm_schema::{cw_serde,QueryResponses};
use cosmwasm_std::{Coin,Decimal, Addr};
use crate::state::MysteryBox;

#[cw_serde]
pub struct InstantiateMsg {
    /// address of randomness provider (aurand)
    pub aurand_address: String,

    /// onwer of this contract
    pub owner: String,

    /// cw-721 contract for box
    pub box_supplier_code_id: u64,

    /// name of box NFT contract
    pub box_supplier_name: String,

    /// symbol of box NFT contract
    pub box_supplier_symbol: String,

    /// cw-721 contract for gift
    pub item_supplier_code_id: u64,

    /// name of box NFT contract
    pub item_supplier_name: String,

    /// symbol of box NFT contract
    pub item_supplier_symbol: String,
}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    /// unbox mystery box
    OpenBox {
        token_id: String,
    },

    /// buy a mystery box
    MintBox {},

    /*/// burn a mystery box and get refund if success
    BurnBox {
        token_id: String,
    }, */

    /// generate a mystery box
    CreateMysteryBox {
        box_info: BoxInfo,
        rate_distribution: RateDistributionMsg,
        default_type: Option<String>
    },

    /// update prefix uri for mystery box
    UpdateMysteryBox {
        prefix_uri: String,
    },

    /// receive aurand randomness
    ReceiveHexRandomness {
        request_id: String,
        randomness: Vec<i32>
    },

    /// withdraw coin
    Withdraw{
        amount: Coin,
        receiver: String,
    },
}

#[cw_serde]
pub struct BoxInfo {
    pub name: String,
    pub description: String,
    pub start_time: String,
    pub end_time: String,
    pub total_supply: u64,
    pub max_item_supply: Option<u64>,
    pub price: Coin
}

//
#[cw_serde]
pub struct ItemTypeMsg {
    pub name: String,
    pub rate: Decimal,
    pub slip_rate: u32,
    pub supply: u32,
} 

#[cw_serde]
pub struct RateDistributionMsg {
    pub vec: Vec<ItemTypeMsg>,
}

#[cw_serde]
pub enum AurandExecuteMsg {
    RequestIntRandomness{
        request_id: String,
        num: u32,
        min: i32,
        max: i32,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Option<MysteryBox>)]
    GetMysteryBoxInformation {},

    #[returns(Option<MysteryBox>)]
    GetMysteryBoxHistoryById {id: String},

    #[returns(LinkedArress)]
    GetLinkedAddres {},
}


#[cw_serde]
pub struct LinkedArress {
    pub aurand_address: Addr,
    pub box_supplier_address: Option<Addr>,
    pub item_supplier_address: Option<Addr>,
}