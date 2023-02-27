use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Job {
    pub box_id: String,
    pub sender: Addr,
}
pub const JOBS: Map<String, Job> = Map::new("jobs");

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub aurand_address: Addr,
    pub supplier_address: Option<Addr>,
}
pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct Rarity {
    pub name: String,
    pub rate: Decimal,
    pub suplly: u32,
} 

impl Rarity {
    pub fn check_rate(&self) -> bool {
        return self.rate.to_uint_ceil() <= Uint128::new(100)
    }
}

#[cw_serde]
pub struct RarityDistribution {
    pub vecs: Vec<Rarity>,
}

impl RarityDistribution {
    pub fn check_rate(&self) -> bool {

        let total_rate = Decimal::zero();
        for r in self.vecs {
            total_rate = total_rate.checked_add(r.rate).unwrap();
        }

        return total_rate == Decimal::from_str("100").unwrap();
    }    

    pub fn total_supply(&self) -> u64 {

        let total_supply = 0u64;
        for r in self.vecs {
            total_supply += r.suplly as u64;
        }

        return total_supply;
    }
}

#[cw_serde]
pub struct MysteryBox {
    pub name: String,
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub rarity_distribution: RarityDistribution,
    pub tokens_uri: Vec<String>,
    pub price: Uint128,
    pub denom: String,
    pub create_time: Timestamp,
}

pub const MYSTERY_BOXS: Map<String, MysteryBox> = Map::new("mystery boxs");