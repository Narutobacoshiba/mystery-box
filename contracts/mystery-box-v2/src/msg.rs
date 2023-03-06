use cosmwasm_schema::{cw_serde, QueryResponses};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    CreateBox{
        name: String,
        start_time: String,
        end_time: String,
        rarity_distribution: RarityDistribution,
        tokens_uri: Vec<String>,
        price: Uint128,
        denom: String,
    },

    ReceiveHexRandomness {
        job_id: String,
        randomness: Vec<String>,
    }
}

/// Message type for `migrate` entry_point
#[cw_serde]
pub enum MigrateMsg {}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
}

