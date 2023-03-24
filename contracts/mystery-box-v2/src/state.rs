use std::str::FromStr;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Decimal, Coin};
use cw_storage_plus::{Item, Map};

use crate::msg::RateDistributionMsg;
use crate::ContractError;

#[cw_serde]
pub struct Job {
    pub sender: Addr,
}
pub const JOBS: Map<String, Job> = Map::new("jobs");

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub aurand_address: Addr,
    pub box_supplier: Option<Addr>,
    pub item_supplier: Option<Addr>,
}
pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct ItemType {
    pub name: String,
    pub rate: Decimal,
    max_rate: Decimal,
    pub slip_rate: u32,
    pub supply: u32,
    max_supply: u32,
} 

impl ItemType {
    fn default(name: Option<String>) -> ItemType {
        // default item type 
        ItemType { 
            name: if name.is_some() {
                name.unwrap()
            } else {
                String::from("common")
            }, 
            rate: Decimal::zero(), 
            max_rate: Decimal::one(), 
            slip_rate: 0u32, 
            supply: u32::MAX,
            max_supply: u32::MAX,
        }
    }
}

#[cw_serde]
pub struct RateDistribution {
    pub vec: Vec<ItemType>,
}

/// random number (recommend 1.5 - 3.0)
const E: &str = "1.5";
const RATE_MODIFY_EXPONENT: u32 = 2u32;
// E ^ (MAX_EXPONENT + 1) case multiplication overflow
const MAX_EXPONENT: u32 = 116u32;

/// Calculate rate modify for a type of rarity using equation:
///     
///    let rate_modifier = (1 / (1 + e ^-(n / slip_rate))) ^ RATE_MODIFY_EXPONENT 
///
///  
/// 
/// e: random number (1.5)
/// 
/// n: number of rarity's supply
/// 
/// slip_rate: the magnitude of the difference between rate reductions when applying rate_modifier
/// 
/// RATE_MODIFY_EXPONENT: increase rate modify impact
fn rate_modifier(n: u32, slip_rate: u32) -> Result<Decimal, ContractError> {
    // if n (total_supply) is 0, rate will be zero
    if n == 0 {
        return Ok(Decimal::zero());
    }
    
    // if slip rate is 0, not apply rate_modifier
    if slip_rate == 0 {
        return Ok(Decimal::one());
    }

    // n_div = n / slip_rate
    let n_div = if n < slip_rate {
        0
    } else {
        let d = n / slip_rate;

        if d > MAX_EXPONENT {
            MAX_EXPONENT
        }else {
            d - 1
        }
    };
    
    // epow = e ^ n_div
    let mut epow: Decimal = Decimal::from_str(E).unwrap();
    epow = epow.pow(n_div);
    
    // l = 1 + 1 / epow
    let one = Decimal::one();
    let l = one.checked_div(epow)
            .map_err(|_| ContractError::DecimalOperationFail{})?
            .checked_add(one)
            .map_err(|_| ContractError::DecimalOperationFail{})?;

    // m = (1 / l) ^ RATE_MODIFY_EXPONENT
    let m = one.checked_div(l)
        .map_err(|_| ContractError::DecimalOperationFail{})?
        .pow(RATE_MODIFY_EXPONENT);

    Ok(m)
}

impl RateDistribution {
    /// init rate distribution
    pub fn new(init_distribution: RateDistributionMsg, default_type: Option<String>) -> Result<RateDistribution,ContractError> {
        let one = Decimal::one();
        let zero = Decimal::zero();
        let mut distribution: RateDistribution = RateDistribution {
            vec: Vec::new()
        };
        
        // total rate of all item type
        let mut total_rate = zero;
        for item_msg in init_distribution.vec.iter() {

            // check if 0 < rate < 1
            if item_msg.rate >= one || item_msg.rate <= zero {
                return Err(ContractError::CustomError {
                    val: format!("Rate of item {} error !!! must 0 < rate < 1",item_msg.name) 
                });
            }
            
            let item: ItemType = ItemType { 
                name: item_msg.name.clone(), 
                rate: item_msg.rate, 
                max_rate: item_msg.rate, 
                slip_rate: item_msg.slip_rate, 
                supply: item_msg.supply,
                max_supply: item_msg.supply
            };

            distribution.vec.push(item);

            // calculate new total_rate
            total_rate = total_rate.checked_add(item_msg.rate)
                .map_err(|_| ContractError::DecimalOperationFail{})?;
        }

        // check if total_rate greater than 1
        if total_rate > one {
            return Err(ContractError::CustomError { 
                val: String::from("total rate greater than 1") 
            })
        }

        // add default item_type to distribution
        distribution.vec.push(ItemType::default(default_type));
        
        // sort distribution by item_type's max_rate
        distribution.sort_item_type();

        Ok(distribution)
    }

    /// sort item type by max rate
    fn sort_item_type(&mut self) {
        self.vec.sort_by(|a, b| a.max_rate.cmp(&b.max_rate));
    }

    /// get item type using random number and max_range number
    /// loop through all item_type and check if the random_number is in one of these item_type's range_bound
    /// range_bound = lower_bound..upper_bound
    pub fn get_item_type_index(&self, random_number: u128, max_range: u128) -> Result<usize, ContractError>{

        let max_range_decimal = Decimal::from_str(&max_range.to_string()).unwrap();
        let mut current_upper_bound = max_range; // upper bound for first item_type is max_range

        for index in 0..(self.vec.len()-1) {
            let item_type = self.vec[index].clone();
            
            // because of 0 < item_type.rate < 1 and total rate <= 1, below operation will never fail 
            // calculate lower_bound of this item type
            let range = max_range_decimal * item_type.rate;
            let lower_bound = current_upper_bound - range.to_uint_ceil().u128(); 
            
            // if random number in range lower_bound..current_upper_bound return current index
            if random_number < current_upper_bound && random_number >= lower_bound {
                return Ok(index);
            }

            // update upper_bound for next item_type
            current_upper_bound = lower_bound;
        }

        // if not find any item_type, return default item_type 
        if let Some(_) = self.vec.last() {
            return Ok(self.vec.len() - 1);
        }

        Err(ContractError::PriceInsufficient{})
    }

    /// update item_type rate and supply at specified index 
    pub fn update_item_type(&mut self, index: usize) -> Result<(),ContractError>{
        let mut item_type = &mut self.vec[index];
    
        if item_type.supply <= 1u32 {
            // if item_type's supply equal 0 after updated
            // set rate to 0
            item_type.rate = Decimal::zero();
            item_type.supply = 0;
        }else {
            let remain_supply = item_type.supply - 1u32;
            // calculate new rate for item_type using rate_modifier
            item_type.rate = item_type.max_rate.checked_mul(rate_modifier(remain_supply, item_type.slip_rate)?)
                .map_err(|_| ContractError::DecimalOperationFail{})?;
            // update new supply
            item_type.supply = remain_supply;
        }
    
        Ok(())
    }
    
    /// calculate current purity of item_type at specified index 
    ///     purity = (h - c) / (h - l) (0..1)
    /// 
    /// 
    /// h: highest rate of an item type
    /// l: lowest rate of an item type
    /// c: current rate of an item type
    pub fn purity(&self, index: usize) -> Result<Decimal,ContractError>{
        let item_type = &self.vec[index];

        let max_rate_modifier = rate_modifier(item_type.max_supply, item_type.slip_rate)?;
        let min_rate_modifier = rate_modifier(1u32, item_type.slip_rate)?;
        let current_rate_modifier = rate_modifier(item_type.supply, item_type.slip_rate)?;

        let purity = (max_rate_modifier - current_rate_modifier) / (max_rate_modifier - min_rate_modifier);
        Ok(purity) 
    }
}

#[cw_serde]
pub struct MysteryBox {
    pub id: String,
    pub name: String,
    pub description: String,
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub rate_distribution: RateDistribution,
    pub prefix_uri: Option<String>,
    pub tokens_id: Vec<u64>,
    pub total_supply: u64,
    pub max_item_supply: u64,
    pub replacement: bool,
    pub price: Coin,
    pub created_time: Timestamp,
}

impl  MysteryBox {
    pub fn remove_token_id(&mut self, index: usize) {
        self.tokens_id.swap_remove(index);
    }
}

pub const MYSTERY_BOX: Item<MysteryBox> = Item::new("mystery box");

pub const MYSTERY_BOX_HISTORY: Map<String, MysteryBox> = Map::new("mystery box history"); 

#[cw_serde]
pub struct PurchasedBox {
    pub is_opened: bool,
    pub open_time: Option<Timestamp>,
    pub is_received_randomness: bool,
}

pub const PURCHASED_BOXES: Map<String, PurchasedBox> = Map::new("purchased boxes");

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test() {
        let a = Decimal::from_str(E).unwrap();
        let b = a.pow(116u32);

        print!("{:?}",b);
    }

    /* #[test]
    fn test_sort_rarity() {
        let mut vecs: Vec<Rarity> = Vec::new();
        
        vecs.push(Rarity{
            name: "limited".to_string(),
            rate: Decimal::from_str("0.09").unwrap(),
            supply: 990u32,
            slip_rate: 1,
        });
        
        vecs.push(Rarity{
            name: "common".to_string(),
            rate: Decimal::from_str("0.90").unwrap(),
            supply: 9000u32,
            slip_rate: 0,
        });

        vecs.push(Rarity{
            name: "rare".to_string(),
            rate: Decimal::from_str("0.01").unwrap(),
            supply: 10u32,
            slip_rate: 1,
        });

        let mut dist: RarityDistribution = RarityDistribution {
            vecs
        };

        assert_eq!(dist.vecs, vec![
            Rarity{
                name: "limited".to_string(),
                rate: Decimal::from_str("0.09").unwrap(),
                supply: 990u32,
                slip_rate: 1,
            },Rarity{
                name: "common".to_string(),
                rate: Decimal::from_str("0.90").unwrap(),
                supply: 9000u32,
                slip_rate: 0,
            },Rarity{
                name: "rare".to_string(),
                rate: Decimal::from_str("0.01").unwrap(),
                supply: 10u32,
                slip_rate: 1,
            }
        ]);

        dist.sort_rarity();

        assert_eq!(dist.vecs, vec![
            Rarity{
                name: "rare".to_string(),
                rate: Decimal::from_str("0.01").unwrap(),
                supply: 10u32,
                slip_rate: 1,
            },Rarity{
                name: "limited".to_string(),
                rate: Decimal::from_str("0.09").unwrap(),
                supply: 990u32,
                slip_rate: 1,
            },Rarity{
                name: "common".to_string(),
                rate: Decimal::from_str("0.90").unwrap(),
                supply: 9000u32,
                slip_rate: 0,
            }
        ]);

        assert_eq!(dist.check_rate().unwrap(), true);

        assert_eq!(dist.get_rarity(Uint256::from_u128(1000u128), Uint256::from_u128(10000u128)).unwrap(), (2, Rarity{
            name: "common".to_string(),
            rate: Decimal::from_str("0.90").unwrap(),
            supply: 9000u32,
            slip_rate: 0,
        }));
    }

    #[test]
    fn test_check_rate_true() {
        let mut vecs: Vec<Rarity> = Vec::new();
        
        vecs.push(Rarity{
            name: "limited".to_string(),
            rate: Decimal::from_str("0.09").unwrap(),
            supply: 990u32,
            slip_rate: 1,
        });
        
        vecs.push(Rarity{
            name: "common".to_string(),
            rate: Decimal::from_str("0.90").unwrap(),
            supply: 9000u32,
            slip_rate: 0,
        });

        vecs.push(Rarity{
            name: "rare".to_string(),
            rate: Decimal::from_str("0.01").unwrap(),
            supply: 10u32,
            slip_rate: 1,
        });

        let dist: RarityDistribution = RarityDistribution {
            vecs
        };

        assert_eq!(dist.check_rate().unwrap(), true);
    }

    #[test]
    fn test_check_rate_false() {
        let mut vecs: Vec<Rarity> = Vec::new();
        
        vecs.push(Rarity{
            name: "limited".to_string(),
            rate: Decimal::from_str("0.1").unwrap(),
            supply: 990u32,
            slip_rate: 1,
        });
        
        vecs.push(Rarity{
            name: "common".to_string(),
            rate: Decimal::from_str("0.90").unwrap(),
            supply: 9000u32,
            slip_rate: 0,
        });

        vecs.push(Rarity{
            name: "rare".to_string(),
            rate: Decimal::from_str("0.01").unwrap(),
            supply: 10u32,
            slip_rate: 1,
        });

        let dist: RarityDistribution = RarityDistribution {
            vecs
        };

        assert_eq!(dist.check_rate().unwrap(), false);
    }

    #[test]
    fn test_total_supply() {
        let mut vecs: Vec<Rarity> = Vec::new();
        
        vecs.push(Rarity{
            name: "limited".to_string(),
            rate: Decimal::from_str("0.1").unwrap(),
            supply: 990u32,
            slip_rate: 1,
        });
        
        vecs.push(Rarity{
            name: "common".to_string(),
            rate: Decimal::from_str("0.90").unwrap(),
            supply: 9000u32,
            slip_rate: 0,
        });

        vecs.push(Rarity{
            name: "rare".to_string(),
            rate: Decimal::from_str("0.01").unwrap(),
            supply: 10u32,
            slip_rate: 1,
        });

        let dist: RarityDistribution = RarityDistribution {
            vecs
        };

        assert_eq!(dist.total_supply(), 10000u64);
    }

    #[test]
    fn test_total_supply_success_with_big_supply() {
        let mut vecs: Vec<Rarity> = Vec::new();
        
        vecs.push(Rarity{
            name: "limited".to_string(),
            rate: Decimal::from_str("0.09").unwrap(),
            supply: u32::MAX,
            slip_rate: 1,
        });
        
        vecs.push(Rarity{
            name: "common".to_string(),
            rate: Decimal::from_str("0.90").unwrap(),
            supply: u32::MAX,
            slip_rate: 0,
        });

        vecs.push(Rarity{
            name: "rare".to_string(),
            rate: Decimal::from_str("0.01").unwrap(),
            supply: u32::MAX,
            slip_rate: 1,
        });

        let dist: RarityDistribution = RarityDistribution {
            vecs
        };

        assert_eq!(dist.total_supply(), 12_884_901_885);
    }

    #[test]
    fn test_get_rarity() {
        let mut vecs: Vec<Rarity> = Vec::new();
        
        vecs.push(Rarity{
            name: "limited".to_string(),
            rate: Decimal::from_str("0.09").unwrap(),
            supply: 990u32,
            slip_rate: 1,
        });
        
        vecs.push(Rarity{
            name: "common".to_string(),
            rate: Decimal::from_str("0.90").unwrap(),
            supply: 9000u32,
            slip_rate: 0,
        });

        vecs.push(Rarity{
            name: "rare".to_string(),
            rate: Decimal::from_str("0.01").unwrap(),
            supply: 10u32,
            slip_rate: 1,
        });

        let mut dist: RarityDistribution = RarityDistribution {
            vecs
        };

        dist.sort_rarity();

        let rarity = dist.get_rarity(Uint256::from_u128(1u128), Uint256::from_u128(10000u128)).unwrap();
        assert_eq!(rarity, (2,Rarity{
            name: "common".to_string(),
            rate: Decimal::from_str("0.90").unwrap(),
            supply: 9000u32,
            slip_rate: 0,
        }));

        let rarity = dist.get_rarity(Uint256::from_u128(9000u128), Uint256::from_u128(10000u128)).unwrap();
        assert_eq!(rarity, (1,Rarity{
            name: "limited".to_string(),
            rate: Decimal::from_str("0.09").unwrap(),
            supply: 990u32,
            slip_rate: 1,
        }));

        let rarity = dist.get_rarity(Uint256::from_u128(9900u128), Uint256::from_u128(10000u128)).unwrap();
        assert_eq!(rarity, (0,Rarity{
            name: "rare".to_string(),
            rate: Decimal::from_str("0.01").unwrap(),
            supply: 10u32,
            slip_rate: 1,
        }));

    }

    #[test]
    fn test_update_rarity() {
        let mut vecs: Vec<Rarity> = Vec::new();
        
        vecs.push(Rarity{
            name: "limited".to_string(),
            rate: Decimal::from_str("0.09").unwrap(),
            supply: 990u32,
            slip_rate: 1,
        });
        
        vecs.push(Rarity{
            name: "common".to_string(),
            rate: Decimal::from_str("0.90").unwrap(),
            supply: 9000u32,
            slip_rate: 0,
        });

        vecs.push(Rarity{
            name: "rare".to_string(),
            rate: Decimal::from_str("0.01").unwrap(),
            supply: 10u32,
            slip_rate: 1,
        });

        let mut dist: RarityDistribution = RarityDistribution {
            vecs
        };

        dist.update_rarity(0, 100).unwrap();

        assert_eq!(dist.vecs[0].supply, 890);
        assert_eq!(dist.vecs[0].rate, Decimal::from_str("0.09").unwrap());

        dist.update_rarity(0, 885).unwrap();

        assert_eq!(dist.vecs[0].supply, 5);
        assert!(dist.vecs[0].rate < Decimal::from_str("0.09").unwrap());

        dist.update_rarity(0, 5).unwrap();

        assert_eq!(dist.vecs[0].supply, 0);
        assert!(dist.vecs[0].rate == Decimal::zero());
    } */
}