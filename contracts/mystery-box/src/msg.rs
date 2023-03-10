use cosmwasm_schema::{cw_serde,QueryResponses};
use cosmwasm_std::Coin;
use crate::state::RarityDistribution;

#[cw_serde]
pub struct InstantiateMsg {
    /// address of randomness provider (aurand)
    pub aurand_address: String,

    /// onwer of this contract
    pub owner: String,

    /// cw-721 contract for box
    pub box_supplier_code_id: u64,

    /// name of box NFT contract
    pub box_name: String,

    /// symbol of box NFT contract
    pub box_symbol: String,

    /// cw-721 contract for gift
    pub gift_supplier_code_id: u64,

    /// name of box NFT contract
    pub gift_name: String,

    /// symbol of box NFT contract
    pub gift_symbol: String,
}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    /// unbox mystery box
    OpenBox {
        box_id: String,
        token_id: String,
    },

    /// buy a mystery box
    BuyBox {
        box_id: String,
    },

    /// generate a mystery box
    CreateMysteryBox {
        name: String,
        start_time: String,
        end_time: String,
        rarity_distribution: RarityDistribution,
        token_uri: String,
        fund: Coin,
    },

    RemoveMysteryBox {
        box_id: String, // id of mystery box
    },

    SetWhiteList {
        list: Vec<String> // list of wallet that can create a mystery box
    },

    /// receive aurand randomness
    ReceiveHexRandomness {
        request_id: String,
        randomness: Vec<i32>
    },
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
}

#[cw_serde]
pub struct Metadata {
    pub rarity: String,
}