use cosmwasm_schema::{cw_serde,QueryResponses};
use cosmwasm_std::{Coin,Decimal, Addr};
use crate::state::{MysteryBox, PurchasedBox};

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

    /// generate a mystery box
    CreateMysteryBox {
        box_info: BoxInfo
    },

    /// update prefix uri for mystery box
    UpdateMysteryBox {
        prefix_uri: String,
        rate_distribution: RateDistributionMsg
    },

    // update source of randomness
    UpdateConfig {
        aurand_address: String
    },

    /// receive aurand randomness
    ReceiveHexRandomness {
        request_id: String,
        randomness: Vec<i32>
    },

    // Re-request randomness for opening of box with token_id
    ReRequestRandomness {
        token_id: String,
    },

    /// withdraw coin
    Withdraw{
        amount: Coin,
        receiver: String,
    },
}

#[cw_serde]
pub struct BoxInfo {
    pub name: String, // name of mystery box event

    pub description: String, // some information about mystery box event

    pub start_time: String, // utc time format "YY-MM-DD hh:mm:ssZ"

    pub end_time: String, // it's required that start_time < end_time

    pub total_supply: u64, // number of unique uri

	// 'true' if you want NFTs uri map 1-1 with Item NFTs supply. 
	// Otherwish NFTs uri can be used to generate multiple NFTs
	pub replacement: bool, 

	// max Item NFTs supply, it is generated from NFTs uri with different rate
	// if replacement set to 'true', max_minted_box must <= total_supply
	// if not set, it will be almost limitless (u64::MAX)
    pub max_minted_box: Option<u64>,

		// price of one box
    pub price: Coin
}

#[cw_serde]
pub struct ItemTypeMsg {
    pub name: String, // name of item type (e.g 'supper rare', 'rare', 'limited')
  
    // rate at which a user can mint an item of this type
    // 0 <= rate <= 1
    pub rate: Decimal,
  
    // the magnitude of the difference between rate reductions when applying rate_modifier
    // when set to 0, the difference between rate reductions is zero. Means the item rate will always be the same
    pub slip_rate: u32, 
  
    pub supply: u32, // maximum number of item that can be minted
}

#[cw_serde]
pub struct RateDistributionMsg {
  pub vec: Vec<ItemTypeMsg>, // list of all type of items, must 0 <= total rate <= 1
      
    // name of default type 
    // if not set, it's will be 'common'
    // it's rate equal to one minus the sum of the rates of all of the above and not apply rate_modifier
    // its supply is almost limitless (u64::MAX)
    pub default_type: Option<String>
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
    GetMysteryBoxInformation {id: Option<u32>},

    #[returns(Option<PurchasedBox>)]
    GetBoxStatus {token_id: String},
    
    #[returns(LinkedArress)]
    GetLinkedAddres {},
}


#[cw_serde]
pub struct LinkedArress {
    pub aurand_address: Addr,
    pub box_supplier_address: Option<Addr>,
    pub item_supplier_address: Option<Addr>,
}