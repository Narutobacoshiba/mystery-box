use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Decimal, Uint128, Uint256};
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
// [{"rare","1",100},{"limited","10",1000},{"common","89",8900}]

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
    pub fn check_rate_and_sort(&self) -> bool {

        let total_rate = Decimal::zero();
        for r in self.vecs {
            total_rate = total_rate.checked_add(r.rate).unwrap();
        }

        self.vecs.sort_by(|a, b| a.rate.cmp(&b.rate));

        return total_rate == Decimal::from_str("100").unwrap();
    }    

    pub fn get_rate(&self, rarity_check: Uint256, max_range: Uint256) -> Option<Rarity>{
        let max_range_decimal = Decimal::from_str(&max_range.clone().to_string()).unwrap();
        let mut current_max_range = max_range;
        for r in self.vecs {
            let bound_range = max_range_decimal * r.rate;
            let min_range = current_max_range - Uint256::from_uint128(bound_range.to_uint_ceil());

            if rarity_check <= current_max_range && rarity_check >= min_range {
                return Some(r);
            }

            current_max_range = min_range;
        }

        return None;
    }

    pub fn update_rate(&self) {
        
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