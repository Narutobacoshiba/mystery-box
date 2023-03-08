use cosmwasm_schema::{cw_serde,QueryResponses};
use cosmwasm_std::{Uint128, Coin};
use crate::state::RarityDistribution;

#[cw_serde]
pub struct InstantiateMsg {
    // address of randomness provider (aurand)
    pub aurand_address: String,

    // onwer of this contract
    pub owner: String,

    // code id of a cw-721 base contract
    pub box_supplier: String,

    pub nft_supplier: String,

    // name of cw721 contract
    pub name: String,

    // symbol of c2721 contract
    pub symbol: String,
}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    //unbox mystery box
    OpenBox {
        box_id: String,
        contract_address: String,
        token_id: String,
    },

    // buy a mystery box
    BuyBox {
        box_id: String,
    },

    // generate a mystery box
    CreateMysteryBox {
        name: String,
        start_time: String,
        end_time: String,
        rarity_distribution: RarityDistribution,
        token_uri: String,
        total_supply: u32,
        fund: Coin,
    },

    RemoveMysteryBox {
        box_id: String, // id of mystery box
    },

    SetWhiteList {
        list: Vec<String> // list of wallet that can create a mystery box
    },

    //receive aurand randomness
    ReceiveHexRandomness {
        request_id: String,
        randomness: Vec<String>
    },
}

#[cw_serde]
pub enum AurandExecuteMsg {
    RequestHexRandomness{
        request_id: String,
        num: u32,
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