#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, Addr, Api, SubMsg,
    MessageInfo, Response, StdResult, WasmMsg, Uint256, ReplyOn,
    Reply, Timestamp, Uint128, Coin,
};
use cw2::set_contract_version;

use cw721_base::MintMsg ;
use cw721_rarity::{
    ExecuteMsg as Cw721RarityExecuteMsg,
    Metadata as Cw721RarityMetadata,
};
use cw721_base::{
    msg::InstantiateMsg as Cw721InstantiateMsg
};

use cw_utils::parse_reply_instantiate_data;

use crate::error::ContractError;
use crate::msg::{
    InstantiateMsg, ExecuteMsg, QueryMsg, AurandExecuteMsg
};
use crate::state::{
    CONFIG, Config,
    JOBS,Job, RarityDistribution,
    MYSTERY_BOXS, MysteryBox, WHITE_LIST,
};
use crate::utils::{
    make_id,
    convert_datetime_string
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:mystery-box";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_TOKEN_REPLY_ID: u64 = 1;

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
        supplier_address: None,
    })?;

    let sub_msg: Vec<SubMsg> = vec![SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: msg.supplier_code_id,
            msg: to_binary(&Cw721InstantiateMsg {
                name: msg.name,
                symbol: msg.symbol,
                minter: env.contract.address.to_string(),
            })?,
            funds: vec![],
            admin: None,
            label: String::from("Instantiate fixed price NFT contract"),
        }
        .into(),
        id: INSTANTIATE_TOKEN_REPLY_ID,
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }];

    Ok(Response::new().add_submessages(sub_msg)
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner)
        .add_attribute("aurand_address", msg.aurand_address))
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
            contract_address,
            token_id,
        } => execute_open_box(deps,env,info,box_id, contract_address, token_id),

        ExecuteMsg::BuyBox { 
            box_id 
        }   => execute_buy_box(deps,env,info,box_id),

        ExecuteMsg::CreateMysteryBox { 
            name, 
            start_time, 
            end_time, 
            rarity_distribution, 
            token_uri,
            total_supply,
            fund,
        } => execute_create_mystery_box(deps,env,info,name,start_time,end_time,
                    rarity_distribution,token_uri,total_supply,fund),

        ExecuteMsg::RemoveMysteryBox {
            box_id,
        } => execute_remove_mystery_box(deps,env,info,box_id),

        ExecuteMsg::SetWhiteList { 
            list 
        } => execute_set_white_list(deps,info,list),

        ExecuteMsg::ReceiveHexRandomness {
            request_id,
            randomness,
        } => execute_receive_hex_randomness(deps, info, request_id, randomness),
    }
}

// Reply callback triggered from cw721 contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if config.supplier_address.is_some() {
        return Err(ContractError::SupplierAlreadyLinked{});
    }

    if msg.id != INSTANTIATE_TOKEN_REPLY_ID {
        return Err(ContractError::InvalidTokenReplyId{});
    }

    let reply = parse_reply_instantiate_data(msg).unwrap();
    config.supplier_address = Some(optional_addr_validate(deps.api, reply.contract_address)?);
    CONFIG.save(deps.storage, &config)?;

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
    token_uri: String,
    total_supply: u32,
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
        env.block.time.to_string()
    ]);
    
    // list of nft id
    let tokens_id = (0u64..=rarity_distribution.total_supply()).collect::<Vec<_>>();

    MYSTERY_BOXS.save(deps.storage, box_id.clone(), &MysteryBox { 
        name, 
        start_time, 
        end_time, 
        rarity_distribution, 
        token_uri, 
        tokens_id,
        fund,
        create_time: block_time, 
    })?;

    Ok(Response::new().add_attribute("action", "create_mystery_box")
                .add_attribute("box_id", box_id)
                .add_attribute("create_time", block_time.to_string()))

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

fn execute_buy_box(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    box_id: String,
) -> Result<Response, ContractError> {
    if !MYSTERY_BOXS.has(deps.storage, box_id.clone()) {
        return Err(ContractError::BoxWithIdNotExist{});
    }

    let mystery_box = MYSTERY_BOXS.load(deps.storage, box_id.clone())?;

    // check denom and get amount
    let denom = mystery_box.fund.denom;
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
        return Err(ContractError::CustomError{val: String::from("Insufficient fee! required") 
                                                + &price.to_string() + &denom});
    }

    let block_time = env.block.time;
    if block_time >= mystery_box.end_time {
        return Err(ContractError::MysteryBoxExpired{});
    }

    Ok(Response::new())
}

fn execute_open_box(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    box_id: String,
    contract_address: String,
    token_id: String,
) -> Result<Response, ContractError> {

    if !MYSTERY_BOXS.has(deps.storage, box_id.clone()) {
        return Err(ContractError::BoxWithIdNotExist{});
    }

    let mystery_box = MYSTERY_BOXS.load(deps.storage, box_id.clone())?;

    let block_time = env.block.time;

    // user only allowed to open box when time start
    if mystery_box.start_time > block_time {
        return Err(ContractError::MysteryBoxNotStart{});
    }

    // user cannot open box when time out
    if mystery_box.end_time <= block_time {
        return Err(ContractError::MysteryBoxExpired{});
    }

    // generate job id for receiving randomness
    let job_id = make_id(vec![box_id.clone(), info.sender.to_string()]);

    // save request open box job, wait for randomness
    JOBS.save(deps.storage, job_id.clone(), &Job{
        box_id: box_id.clone(),
        sender: info.sender,
    })?;

    let config = CONFIG.load(deps.storage)?;
    // request randomness from aurand contract
    let msg = WasmMsg::Execute {
        contract_addr: config.aurand_address.to_string(),
        msg: to_binary(&AurandExecuteMsg::RequestIntRandomness { 
                        request_id: job_id.clone(),
                        num: NUMBER_OF_RANDOM,
                        min: MIN_RANGE_RANDOM,
                        max: MAX_RANGE_RANDOM,
                    })?,
        funds: info.funds,
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action","un_box")
        .add_attribute("job_id",job_id)
        .add_attribute("box_id", box_id))
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
    if config.supplier_address.is_none() {
        return Err(ContractError::SupplierNotLinked{});
    }

    let supplier_address = config.supplier_address.unwrap();

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

    let tokens_id = mystery_box.tokens_id;

    // random token id index
    let tokens_id_index = random_index % tokens_id.len();

    let token_uri_index = tokens_id[tokens_id_index];

    // token uri made by combining token_uri
    let unique_token_uri = mystery_box.token_uri + &token_uri_index.to_string();

    // token id made by combining box id and token id 
    let unique_token_id = make_id(vec![box_id.clone(), token_uri_index.to_string()]);

    let extension = Some(Cw721RarityMetadata {
        rarity: rarity.name, 
        ..Cw721RarityMetadata::default()
    });
    // create mint message NFT for the sender
    let mint_msg = WasmMsg::Execute {
        contract_addr: supplier_address.to_string(),
        msg: to_binary(&Cw721RarityExecuteMsg::Mint(MintMsg {
            token_id: unique_token_id.clone(),
            owner: sender.clone().to_string(),
            token_uri: Some(unique_token_uri),
            extension,
        }))?,
        funds: vec![],
    };

    MYSTERY_BOXS.save(deps.storage, box_id.clone(), &mystery_box)?;

    Ok(Response::new().add_message(mint_msg)
                .add_attribute("action", "receive_hex_randomness")
                .add_attribute("token_id", unique_token_id)
                .add_attribute("token_uri", unique_token_uri)
                .add_attribute("minter", sender)
                )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // TODO: add query for MarketplaceInfo here
    match msg {
    }
}
//https://academy.binance.com/en/articles/what-are-nft-mystery-boxes-and-how-do-they-work


