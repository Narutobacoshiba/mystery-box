use std::str::FromStr;
use std::collections::HashMap;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Decimal, Uint256, Coin};
use cw_storage_plus::{Item, Map};

use crate::ContractError;

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
    pub box_supplier: Option<Addr>,
    pub gift_supplier: Option<Addr>,
}
pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct Rarity {
    pub name: String,
    pub rate: Decimal,
    pub slip_rate: u32,
    pub supply: u32,
} 

#[cw_serde]
pub struct RarityDistribution {
    pub vecs: Vec<Rarity>,
}
/// random number (recommend 1.5 - 3.0)
const E: &str = "1.5";
const RATE_MODIFY_EXPONENT: u32 = 3u32;
const MAX_EXPONENT: u32 = 47u32;

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
/// slip_rate: how fast the rate modify will change when n change
/// 
/// RATE_MODIFY_EXPONENT: increase rate modify impact
pub fn rate_modifier(n: u32, slip_rate: u32) -> Result<Decimal, ContractError> {
    if n == 0 {
        return Ok(Decimal::zero());
    }
    
    if slip_rate == 0 {
        return Ok(Decimal::one());
    }

    let n = if n < slip_rate {
        0
    } else {
        let d = n / slip_rate;

        if d > MAX_EXPONENT {
            MAX_EXPONENT
        }else {
            d - 1
        }
    };
    
    // epow = e ^ n
    let mut epow: Decimal = Decimal::from_str(E)
        .map_err(|_| ContractError::InvalidDecimalFormat{})?;
    epow = epow.pow(n);
    
    // n = 1 + 1 / epow
    let one = Decimal::one();
    let n = one.checked_div(epow)
            .map_err(|_| ContractError::DecimalOperationFail{})?
            .checked_add(one)
            .map_err(|_| ContractError::DecimalOperationFail{})?;

    // m = (1 / n) ^ RATE_MODIFY_EXPONENT
    let m = one.checked_div(n)
        .map_err(|_| ContractError::DecimalOperationFail{})?
        .pow(RATE_MODIFY_EXPONENT);

    Ok(m)
}

impl RarityDistribution {
    /// sort rarity by rate
    pub fn sort_rarity(&mut self) {
        self.vecs.sort_by(|a, b| a.rate.cmp(&b.rate));
    }

    /// check if total rate of rarity in range 0..1
    pub fn check_rate(&self) -> Result<bool, ContractError> {

        let mut total_rate = Decimal::zero();
        for r in self.vecs.iter() {
            total_rate = total_rate.checked_add(r.rate)
                .map_err(|_| ContractError::DecimalOperationFail{})?;
        }

        Ok(total_rate <= Decimal::one() && total_rate >= Decimal::zero())
    }    

    /// get rarity using random rarity_check number and max_range number
    pub fn get_rarity(&self, rarity_check: Uint256, max_range: Uint256) -> Result<(usize, Rarity), ContractError>{

        let max_range_decimal = Decimal::from_str(&max_range.clone().to_string())
            .map_err(|_| ContractError::InvalidDecimalFormat{})?;
        let mut current_max_bound = max_range;

        for index in 0..self.vecs.len() {
            let rarity = self.vecs[index].clone();

            let range = max_range_decimal * rarity.rate;
            let min_bound = current_max_bound - Uint256::from_uint128(range.to_uint_ceil());

            if rarity_check < current_max_bound && rarity_check >= min_bound {
                return Ok((index, rarity));
            }

            current_max_bound = min_bound;
        }


        /* let mut count = self.vecs.len();
        let mut rev_iter = self.vecs.iter().rev();
        while let Some(com) = rev_iter.next() {

            count -= 1;

            if com.supply > 0 {
                return Ok((count, com.to_owned()));
            }
        }

        Err(ContractError::PriceInsufficient{}) */

        if let Some(rarity) = self.vecs.last() {
            return Ok((self.vecs.len() - 1, rarity.to_owned()));
        }

        Err(ContractError::PriceInsufficient{})
    }


    pub fn update_rarity(&mut self, index: usize, consumed: u32) -> Result<(),ContractError>{
        let mut rarity = &mut self.vecs[index];
    
        if consumed >= rarity.supply {
            rarity.rate = Decimal::zero();
            rarity.supply = 0;
        }else {
            let remain_supply = rarity.supply - consumed;
            rarity.rate = rarity.rate.checked_mul(rate_modifier(remain_supply, rarity.slip_rate)?)
                .map_err(|_| ContractError::DecimalOperationFail{})?;
            rarity.supply = remain_supply;
        }
    
        Ok(())
    }


    pub fn total_supply(&self) -> u64 {
    
        let mut total_supply = 0u64;
        for r in self.vecs.iter() {
            total_supply += r.supply as u64;
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
    pub token_uri: Option<String>,
    pub tokens_id: Vec<u64>,
    pub max_supply: u64,
    pub fund: Coin,
    pub create_time: Timestamp,
    pub owner: Addr,
}

impl MysteryBox {
    pub fn remove_token_id(&mut self, index: usize) {
        self.tokens_id.swap_remove(index);
    }
}
pub const MYSTERY_BOXS: Map<String, MysteryBox> = Map::new("mystery boxs");

#[cw_serde]
pub struct BoxPurchase {
    pub buyer: Addr,
    pub time: Timestamp,
    pub is_opened: bool,
}

pub const BOX_PURCHASES: Map<String, (usize, HashMap<String, BoxPurchase>)> = Map::new("box purchase");

pub const WHITE_LIST: Map<Addr, bool> = Map::new("white list");

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test() {
        let mut a = vec![1,2,3,4,5];
        let b: &mut Vec<i32> = a.as_mut();

        b[3] = 6;

        print!("{:?}",a);
    }

    #[test]
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
        assert!(dist.vecs[0].rate == Decimal::from_str("0.09").unwrap());

        dist.update_rarity(0, 885).unwrap();

        assert_eq!(dist.vecs[0].supply, 5);
        assert!(dist.vecs[0].rate < Decimal::from_str("0.09").unwrap());

        dist.update_rarity(0, 5).unwrap();

        assert_eq!(dist.vecs[0].supply, 0);
        assert!(dist.vecs[0].rate == Decimal::zero());
    }
}