use cosmwasm_schema::{cw_serde,QueryResponses};
use cosmwasm_std::Uint128;
use crate::state::RarityDistribution;

#[cw_serde]
pub struct InstantiateMsg {
    // address of randomness provider (aurand)
    pub aurand_address: String,

    // onwer of this contract
    pub owner: String,

    // code id of a cw-721 base contract
    pub supplier_code_id: u64,

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
    },

    CreateMysteryBox {
        name: String,
        start_time: String,
        end_time: String,
        rarity_distribution: RarityDistribution,
        tokens_uri: Vec<String>,
        price: Uint128,
        denom: String,
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