use std::collections::HashMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, Addr, Api, SubMsg, QueryRequest,
    MessageInfo, Response, StdResult, WasmMsg, Uint256, ReplyOn, WasmQuery,
    Reply, Timestamp, Uint128, Coin, BankMsg
};
use cw2::set_contract_version;
use cw721::{Cw721QueryMsg, Expiration as Cw721Expiration};
use cw721_base::MintMsg;
use cw721_rarity::{
    ExecuteMsg as Cw721RarityExecuteMsg,
    InstantiateMsg as Cw721RarityInstantiateMsg,
    Metadata as Cw721RarityMetadata,
};
use cw721_base::{
    InstantiateMsg as Cw721InstantiateMsg,
    ExecuteMsg as Cw721ExecuteMsg,
    Extension as Cw721Extension,
};
use cw_utils::parse_reply_instantiate_data;

use crate::error::ContractError;
use crate::msg::{
    InstantiateMsg, ExecuteMsg, QueryMsg, AurandExecuteMsg
};
use crate::state::{
    CONFIG, Config,
    JOBS,Job, RarityDistribution,
    MYSTERY_BOXS, MysteryBox, WHITE_LIST, BoxPurchase, BOX_PURCHASES,
};
use crate::utils::{
    make_id,
    convert_datetime_string
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:mystery-box";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_BOX_NFT_REPLY_ID: u64 = 1;
const INSTANTIATE_GIFT_NFT_REPLY_ID: u64 = 2;

const NUMBER_OF_RANDOM: u32 = 1u32;
const MIN_RANGE_RANDOM: i32 = 0i32;
const MAX_RANGE_RANDOM: i32 = 10000i32;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let aurand_addr = optional_addr_validate(deps.api, msg.aurand_address.clone())?;
    let owner_addr = optional_addr_validate(deps.api, msg.owner.clone())?;


    CONFIG.save(deps.storage, &Config{
        owner: owner_addr,
        aurand_address: aurand_addr,
        gift_supplier: None,
        box_supplier: None,
    })?;

    let sub_msg: Vec<SubMsg> = vec![SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: msg.box_supplier_code_id,
            msg: to_binary(&Cw721InstantiateMsg {
                name: msg.box_name,
                symbol: msg.box_symbol,
                minter: env.contract.address.to_string(),
            })?,
            funds: vec![],
            admin: None,
            label: String::from("Instantiate box NFT contract"),
        }
        .into(),
        id: INSTANTIATE_BOX_NFT_REPLY_ID,
        gas_limit: None,
        reply_on: ReplyOn::Success,
    },SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: msg.gift_supplier_code_id,
            msg: to_binary(&Cw721RarityInstantiateMsg {
                name: msg.gift_name,
                symbol: msg.gift_symbol,
                minter: env.contract.address.to_string(),
            })?,
            funds: vec![],
            admin: None,
            label: String::from("Instantiate item NFT contract"),
        }
        .into(),
        id: INSTANTIATE_GIFT_NFT_REPLY_ID,
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }];

    Ok(Response::new().add_submessages(sub_msg)
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner)
        .add_attribute("aurand_address", msg.aurand_address)
        .add_attribute("gift_supplier_code_id", msg.gift_supplier_code_id.to_string())
        .add_attribute("box_supplier_code_id", msg.box_supplier_code_id.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::OpenBox {
            box_id,
            token_id,
        } => execute_open_box(deps,env,info,box_id, token_id),

        ExecuteMsg::MintBox { 
            box_id 
        }   => execute_mint_box(deps,env,info,box_id),

        ExecuteMsg::BurnBox { 
            box_id, 
            token_id 
        } => execute_burn_box(deps,env,info,box_id,token_id),

        ExecuteMsg::CreateMysteryBox { 
            name, 
            start_time, 
            end_time, 
            rarity_distribution, 
            max_supply,
            fund,
        } => execute_create_mystery_box(deps,env,info,name,start_time,end_time,
                    rarity_distribution,max_supply,fund),
        
        ExecuteMsg::UpdateMysteryBox {
            box_id,
            token_uri
        } => execute_update_mystery_box(deps,env,info,box_id,token_uri),

        ExecuteMsg::RemoveMysteryBox {
            box_id,
        } => execute_remove_mystery_box(deps,env,info,box_id),

        ExecuteMsg::SetWhiteList { 
            list 
        } => execute_set_white_list(deps,info,list),

        ExecuteMsg::ReceiveHexRandomness {
            request_id,
            randomness,
        } => execute_receive_hex_randomness(deps,info,request_id,randomness),
    }
}

// Reply callback triggered from cw721 contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    let reply = parse_reply_instantiate_data(msg.clone()).unwrap();

    match msg.id {
        INSTANTIATE_BOX_NFT_REPLY_ID => {
            if config.box_supplier.is_some() {
                return Err(ContractError::BoxSupplierAlreadyLinked{});
            }

            config.box_supplier = Some(optional_addr_validate(deps.api, reply.contract_address)?);
            CONFIG.save(deps.storage, &config)?;
        },

        INSTANTIATE_GIFT_NFT_REPLY_ID => {
            if config.gift_supplier.is_some() {
                return Err(ContractError::GiftSupplierAlreadyLinked{});
            }

            config.gift_supplier = Some(optional_addr_validate(deps.api, reply.contract_address)?);
            CONFIG.save(deps.storage, &config)?;
        },

        _ => {}
    }

    Ok(Response::new())
}

fn optional_addr_validate(api: &dyn Api, addr: String) -> Result<Addr, ContractError> {
    let addr = api.addr_validate(&addr).map_err(|_| ContractError::InvalidAddress{})?;
    Ok(addr)
}

fn execute_create_mystery_box(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
    start_time: String,
    end_time: String,
    rarity_distribution: RarityDistribution,
    max_supply: Option<u64>,
    fund: Coin,
) -> Result<Response, ContractError> {

    let config = CONFIG.load(deps.storage)?;

    if config.owner != info.sender {
        return Err(ContractError::Unauthorized{});
    }

    let start_time: Timestamp = convert_datetime_string(start_time)?;
    let end_time: Timestamp = convert_datetime_string(end_time)?;

    let block_time: Timestamp = env.block.time;

    if end_time <= block_time {
        return Err(ContractError::InvalidEndTime{});
    } 

    if !rarity_distribution.check_rate()? {
        return Err(ContractError::InvalidRarityRate{});
    }

    let box_id = make_id(vec![
        info.sender.to_string(), 
        env.block.time.to_string(),
    ]);
    
    let max_supply = if max_supply.is_none() {
        rarity_distribution.total_supply()
    } else{
        max_supply.unwrap()
    };
    // list of nft id
    let tokens_id = (0u64..=max_supply).collect::<Vec<_>>();

    MYSTERY_BOXS.save(deps.storage, box_id.clone(), &MysteryBox {
        name, 
        start_time, 
        end_time, 
        rarity_distribution,  
        tokens_id,
        fund,
        max_supply,
        token_uri: None,
        create_time: block_time, 
        owner: info.sender,
    })?;

    let init_box_purchases: HashMap<String, BoxPurchase> = HashMap::new();
    BOX_PURCHASES.save(deps.storage, box_id.clone(), &(0usize, init_box_purchases))?;

    Ok(Response::new().add_attribute("action", "create_mystery_box")
                .add_attribute("box_id", box_id)
                .add_attribute("create_time", block_time.to_string()))
}


fn execute_update_mystery_box(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    box_id: String,
    token_uri: String,      
) -> Result<Response, ContractError> {

    if !MYSTERY_BOXS.has(deps.storage, box_id.clone()) {
        return Err(ContractError::BoxWithIdNotExist{}); 
    }

    let mut mystery_box = MYSTERY_BOXS.load(deps.storage, box_id.clone())?;

    if mystery_box.owner != info.sender.clone() {
        return Err(ContractError::Unauthorized{});
    }

    if mystery_box.token_uri.is_some() {
        return Err(ContractError::CustomError{val: String::from("Token uri already set")});
    }

    let block_time = env.block.time;
    if mystery_box.start_time <= block_time {
        return Err(ContractError::CustomError{val: String::from("Mystery box cannot update now")});
    }

    mystery_box.token_uri = Some(token_uri);

    MYSTERY_BOXS.save(deps.storage, box_id.clone(), &mystery_box)?;

    Ok(Response::new().add_attribute("action", "update_mystery_box"))
}


fn execute_remove_mystery_box (
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    box_id: String,
) -> Result<Response, ContractError> {

    let config = CONFIG.load(deps.storage)?;

    if config.owner != info.sender {
        return Err(ContractError::Unauthorized{});
    }

    if !MYSTERY_BOXS.has(deps.storage, box_id.clone()) {
        return Err(ContractError::CustomError{val: String::from("box with id don't exist")});
    }

    let mystery_box = MYSTERY_BOXS.load(deps.storage, box_id.clone())?;

    let block_time = env.block.time;

    if block_time >= mystery_box.start_time && block_time <= mystery_box.end_time {
        return Err(ContractError::CustomError{val: String::from("mystery box in process, cannot remove")});
    }

    MYSTERY_BOXS.remove(deps.storage, box_id.clone());

    Ok(Response::new().add_attribute("action", "remove_mystery_box")
                .add_attribute("box_id", box_id)
                .add_attribute("owner", config.owner))
}

fn execute_set_white_list(
    deps: DepsMut,
    info: MessageInfo,
    list: Vec<String>
) -> Result<Response, ContractError> {

    let config = CONFIG.load(deps.storage)?;

    if config.owner != info.sender {
        return Err(ContractError::Unauthorized{});
    }

    for it in list {
        let addr = optional_addr_validate(deps.api, it)?;
        
        WHITE_LIST.save(deps.storage, addr, &true)?;
    }

    Ok(Response::new().add_attribute("action", "set_white_list")
            .add_attribute("owner", config.owner))
}

fn execute_burn_box(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    box_id: String,
    token_id: String,
) -> Result<Response, ContractError> {

    if !MYSTERY_BOXS.has(deps.storage, box_id.clone()){
        return Err(ContractError::BoxWithIdNotExist{});
    }

    let mystery_box = MYSTERY_BOXS.load(deps.storage, box_id.clone())?;
    let (count, mut box_purchases) = BOX_PURCHASES.load(deps.storage, box_id.clone())?;

    // check if user already buy this box
    if !box_purchases.contains_key(&token_id) {
        return Err(ContractError::CustomError{val: String::from("Token not recognized!")});
    } 
    let buyer = &box_purchases[&token_id];

    if buyer.buyer != info.sender {
        return Err(ContractError::Unauthorized{});
    }
    
    let block_time = env.block.time;
    if !(mystery_box.token_uri.is_none() && mystery_box.start_time <= block_time) {
        return Err(ContractError::CustomError {
            val: String::from("Only can burn when token uri not set 
                            before start time of mystery box!")});
    }

    let msg = BankMsg::Send {
        to_address: buyer.buyer.to_string(),
        amount: vec![mystery_box.fund],
    };

    // Remove token IDs from purchase history
    box_purchases.remove(&token_id);
    BOX_PURCHASES.save(deps.storage, box_id.clone(),&(
        count, 
        box_purchases
    ))?;

    Ok(Response::new().add_message(msg)
        .add_attribute("action", "burn_box"))
}

fn execute_mint_box(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    box_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check if box supplier is not yet linked
    if config.box_supplier.is_none(){
        return Err(ContractError::BoxSupplierNotLinked{});
    }
    let box_supplier = config.box_supplier.unwrap();
    
    // check if mystery box with box_id exist
    if !MYSTERY_BOXS.has(deps.storage, box_id.clone()) {
        return Err(ContractError::BoxWithIdNotExist{});
    }

    let mut mystery_box = MYSTERY_BOXS.load(deps.storage, box_id.clone())?;
    let (count, mut box_purchases) = BOX_PURCHASES.load(deps.storage, box_id.clone())?;

    // check if mystery_box soldout
    if mystery_box.max_supply == 0 {
        return Err(ContractError::SoldOut{});
    }

    // check denom and get amount
    let denom = mystery_box.fund.denom.clone();
    let matching_coin = info.funds.iter().find(|fund| fund.denom.eq(&denom));
    let sent_amount: Uint128 = match matching_coin {
        Some(coin) => coin.amount,
        None => {
            return Err(ContractError::CustomError {
                val: "Expected denom ".to_string() + &denom,
            });
        }
    };

    let price = mystery_box.fund.amount;
    if sent_amount < price {
        return Err(ContractError::CustomError{
            val: String::from("Insufficient fee! required")                        
            + &price.to_string() + &denom});
    }

    // check if mystery box expired
    let block_time = env.block.time;
    if block_time >= mystery_box.end_time {
        return Err(ContractError::MysteryBoxExpired{});
    }

    let token_id = make_id(vec![box_id.clone(), count.to_string()]);
    // create mint message NFT for the sender
    let mint_msg = WasmMsg::Execute {
        contract_addr: box_supplier.to_string(),
        msg: to_binary(&Cw721ExecuteMsg::<Cw721Extension, Cw721Extension>::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: info.sender.to_string(),
            token_uri: None,
            extension: None,
        }))?,
        funds: vec![],
    };

    mystery_box.max_supply -= 1;
    MYSTERY_BOXS.save(deps.storage, box_id.clone(), &mystery_box)?;

    box_purchases.insert(token_id.clone(), BoxPurchase { 
        buyer: info.sender.clone(), 
        time: env.block.time,
        is_opened: false
    });
    BOX_PURCHASES.save(deps.storage, box_id.clone(),&(
        count + 1, 
        box_purchases
    ))?;

    Ok(Response::new().add_message(mint_msg)
            .add_attribute("action", "buy_box")
            .add_attribute("box_id", box_id)
            .add_attribute("token_id", token_id)
            .add_attribute("buyer", info.sender))
}

fn execute_open_box(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    box_id: String,
    token_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check if box supplier is not yet linked
    if config.box_supplier.is_none(){
        return Err(ContractError::BoxSupplierNotLinked{});
    }
    let box_supplier = config.box_supplier.unwrap();

    if !MYSTERY_BOXS.has(deps.storage, box_id.clone()) {
        return Err(ContractError::BoxWithIdNotExist{});
    }

    let mystery_box = MYSTERY_BOXS.load(deps.storage, box_id.clone())?;
    let (count, mut box_purchases) = BOX_PURCHASES.load(deps.storage, box_id.clone())?;

    // check if user already buy this box
    if !box_purchases.contains_key(&box_id) {
        return Err(ContractError::CustomError{val: String::from("Token not recognized!")});
    }
    let buyer = &box_purchases[&box_id];

    if buyer.buyer != info.sender.to_string() {
        return Err(ContractError::Unauthorized{});
    }

    if buyer.is_opened {
        return  Err(ContractError::CustomError{val: String::from("Box has opened!")});
    }

    if mystery_box.token_uri.is_some() {
        return Err(ContractError::CustomError{val: String::from("Token uri not set")});
    }

    let block_time = env.block.time;
    // user only allowed to open box when time start
    if mystery_box.start_time > block_time {
        return Err(ContractError::MysteryBoxNotStart{});
    }

    // user cannot open box when time out
    if mystery_box.end_time <= block_time {
        return Err(ContractError::MysteryBoxExpired{});
    }

    // check if user is the owner of the token
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: token_id.clone(),
        include_expired: Some(false),
    };
    let owner_response: StdResult<cw721::OwnerOfResponse> =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: box_supplier.to_string(),
            msg: to_binary(&query_owner_msg)?,
        }));
    match owner_response {
        Ok(owner) => {
            if owner.owner != info.sender {
                return Err(ContractError::Unauthorized {});
            }
        }
        Err(_) => {
            return Err(ContractError::Unauthorized {});
        }
    }

    // check that user approves this contract to manage this token
    // for now, we require never expired approval
    let query_approval_msg = Cw721QueryMsg::Approval {
        token_id: token_id.clone(),
        spender: env.contract.address.to_string(),
        include_expired: Some(true),
    };
    let approval_response: StdResult<cw721::ApprovalResponse> =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: box_supplier.to_string(),
            msg: to_binary(&query_approval_msg)?,
        }));

    // check if approval is never expired
    match approval_response {
        Ok(approval) => match approval.approval.expires {
            Cw721Expiration::Never {} => {}
            _ => return Err(ContractError::Unauthorized {}),
        },
        Err(_) => {
            return Err(ContractError::CustomError {
                val: "Require never expired approval".to_string(),
            });
        }
    }

    // generate job id for receiving randomness
    let job_id = make_id(vec![box_id.clone(), info.sender.to_string()]);
    
    // request randomness from aurand contract
    let random_msg = WasmMsg::Execute {
        contract_addr: config.aurand_address.to_string(),
        msg: to_binary(&AurandExecuteMsg::RequestIntRandomness { 
                        request_id: job_id.clone(),
                        num: NUMBER_OF_RANDOM,
                        min: MIN_RANGE_RANDOM,
                        max: MAX_RANGE_RANDOM,
                    })?,
        funds: info.funds,
    };

    let burn_msg = WasmMsg::Execute {
        contract_addr: box_supplier.to_string(),
        msg: to_binary(&Cw721ExecuteMsg::<Cw721Extension, Cw721Extension>::Burn{
            token_id: token_id.clone()
        })?,
        funds: vec![],
    };

    // save request open box job, wait for randomness
    JOBS.save(deps.storage, job_id.clone(), &Job{
        box_id: box_id.clone(),
        sender: info.sender,
    })?;

    // Remove token IDs from purchase history
    box_purchases.remove(&token_id);
    BOX_PURCHASES.save(deps.storage, box_id.clone(),&(
        count, 
        box_purchases
    ))?;

    Ok(Response::new()
        .add_message(random_msg)
        .add_message(burn_msg)
        .add_attribute("action","un_box")
        .add_attribute("job_id",job_id)
        .add_attribute("box_id", box_id)
        .add_attribute("token_id", token_id))
}

fn get_token_uri(uri_prefix: String, token_id: String) -> String {
    // TODO: maybe we need the suffix of the token_uri, too
    // the token_uri is the uri_prefix + token_id
    let token_uri = format!("{}{}", uri_prefix, token_id);
    token_uri
}

fn execute_receive_hex_randomness(
    deps: DepsMut,
    info: MessageInfo,
    request_id: String,
    randomness: Vec<i32>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // only accept randomness from aurand contract
    if config.aurand_address != info.sender {
        return Err(ContractError::Unauthorized{});
    }

    // must link to a cw721 contract
    if config.gift_supplier.is_none() {
        return Err(ContractError::GiftSupplierNotLinked{});
    }
    let gift_supplier = config.gift_supplier.unwrap();

    // check if a job with job_id exist
    if !JOBS.has(deps.storage, request_id.clone()) {
        return Err(ContractError::CustomError{val:"Job with id does't exist!".to_string()});
    }

    // get job by request id
    let Job{box_id, sender}= JOBS.load(deps.storage, request_id.clone())?;

    // get mystery box by id
    let mut mystery_box = MYSTERY_BOXS.load(deps.storage, box_id.clone())?;

    // check if randomness valid
    if randomness.len() != 1 || randomness[0] < MIN_RANGE_RANDOM || randomness[0] > MAX_RANGE_RANDOM  {
        return Err(ContractError::InvalidRandomness{});
    }

    let random_index = randomness[0] as usize;

    let (index, rarity) = mystery_box.rarity_distribution.get_rarity(
        Uint256::from_u128(random_index as u128), 
        Uint256::from_u128(MAX_RANGE_RANDOM as u128)
    )?;
    mystery_box.rarity_distribution.update_rarity(index, 1)?;

    let tokens_id = &mystery_box.tokens_id;

    // random token id index
    let tokens_id_index = random_index % tokens_id.len();

    // get and remove choosen token id
    let token_uri_index = tokens_id[tokens_id_index];
    mystery_box.remove_token_id(tokens_id_index);

    let token_uri = &mystery_box.token_uri.clone().unwrap();
    // token uri made by combining token_uri
    let unique_token_uri = get_token_uri(token_uri.to_owned(), token_uri_index.to_string());

    // token id made by combining box id and token id 
    let unique_token_id = make_id(vec![box_id.clone(), token_uri_index.to_string()]);

    let extension = Some(Cw721RarityMetadata {
        rarity: rarity.name, 
        ..Cw721RarityMetadata::default()
    });

    // create mint message NFT for the sender
    let mint_msg = WasmMsg::Execute {
        contract_addr: gift_supplier.to_string(),
        msg: to_binary(&Cw721RarityExecuteMsg::Mint(MintMsg {
            token_id: unique_token_id.clone(),
            owner: sender.clone().to_string(),
            token_uri: Some(unique_token_uri.clone()),
            extension,
        }))?,
        funds: vec![],
    };

    MYSTERY_BOXS.save(deps.storage, box_id.clone(), &mystery_box)?;

    Ok(Response::new().add_message(mint_msg)
                .add_attribute("action", "receive_hex_randomness")
                .add_attribute("token_id", unique_token_id)
                .add_attribute("token_uri", unique_token_uri)
                .add_attribute("minter", sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // TODO: add query for MarketplaceInfo here
    match msg {
    }
}
//https://academy.binance.com/en/articles/what-are-nft-mystery-boxes-and-how-do-they-work


