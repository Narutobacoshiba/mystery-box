#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, Addr, Api, SubMsg,
    MessageInfo, Response, StdResult, WasmMsg, Uint256, ReplyOn,
    Reply, Timestamp, Uint128,
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
    convert_datetime_string,
    uint256_2_usize
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:mystery-box";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_TOKEN_REPLY_ID: u64 = 1;

const MAX_RANGE_RANDOM: u128 = 10000u128;

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
        } => execute_open_box(deps,env,info,box_id),

        ExecuteMsg::CreateMysteryBox { 
            name, 
            start_time, 
            end_time, 
            rarity_distribution, 
            tokens_uri,
            price,
            denom,
        } => execute_create_mystery_box(deps,env,info,name,start_time,end_time,
                    rarity_distribution,tokens_uri,price,denom),

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
    tokens_uri: Vec<String>,
    price: Uint128,
    denom: String,
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

    MYSTERY_BOXS.save(deps.storage, box_id.clone(), &MysteryBox { 
        name, 
        start_time, 
        end_time, 
        rarity_distribution, 
        tokens_uri, 
        price, 
        denom, 
        create_time: block_time 
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

    return Ok(Response::new().add_attribute("action", "set_white_list")
                .add_attribute("owner", config.owner))
}

fn execute_open_box(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    box_id: String,
) -> Result<Response, ContractError> {

    if !MYSTERY_BOXS.has(deps.storage, box_id.clone()) {
        return Err(ContractError::CustomError{val: String::from("box with id don't exist")});
    }

    let mystery_box = MYSTERY_BOXS.load(deps.storage, box_id.clone())?;

    // check denom and get amount
    let denom = mystery_box.denom;
    let matching_coin = info.funds.iter().find(|fund| fund.denom.eq(&denom));
    let sent_amount: Uint128 = match matching_coin {
        Some(coin) => coin.amount,
        None => {
            return Err(ContractError::CustomError {
                val: "Expected denom ".to_string() + &denom,
            });
        }
    };

    let fee = mystery_box.price;
    if sent_amount < fee {
        return Err(ContractError::CustomError{val: String::from("Insufficient fee! required") 
                                                + &fee.to_string() + &denom});
    }

    let block_time = env.block.time;

    // user only allowed to open box when time start
    if mystery_box.start_time > block_time {
        return Err(ContractError::MysteryBoxNotStart{})
    }

    // user cannot open box when time out
    if mystery_box.end_time <= block_time {
        return Err(ContractError::MysteryBoxTimeOut{})
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
        msg: to_binary(&AurandExecuteMsg::RequestHexRandomness { 
                        request_id: job_id.clone(),
                        num: 1,
                    })?,
        funds: info.funds,
    };

    return Ok(Response::new().add_message(msg)
                        .add_attribute("action","un_box")
                        .add_attribute("job_id",job_id)
                        .add_attribute("box_id", box_id))
}

fn execute_receive_hex_randomness(
    deps: DepsMut,
    info: MessageInfo,
    request_id: String,
    randomness: Vec<String>,
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
    if randomness.len() != 1 {
        return Err(ContractError::InvalidRandomness{});
    }

    let token_id = make_id(vec![box_id.clone(), randomness[0].clone()]);

    let randomness: [u8; 32] = hex::decode(randomness[0].clone())
            .map_err(|_| ContractError::InvalidRandomness{})?
            .as_slice().try_into()
            .map_err(|_| ContractError::InvalidRandomness{})?;

    let random_number: Uint256 = Uint256::new(randomness);

    let rarity_check = random_number.checked_rem(Uint256::from_u128(MAX_RANGE_RANDOM))
                    .map_err(|_| ContractError::Uint256OperatorError{})?;

    let (index, rarity) = mystery_box.rarity_distribution.get_rarity(rarity_check, Uint256::from_u128(MAX_RANGE_RANDOM))?;
    mystery_box.rarity_distribution.update_rarity(index, 1)?;

    let tokens_uri = mystery_box.tokens_uri.clone();

    // random token url
    let tokens_uri_index = random_number.checked_rem(Uint256::from_u128(tokens_uri.len() as u128))
                    .map_err(|_| ContractError::Uint256OperatorError{})?;

    let extension = Some(Cw721RarityMetadata {
        rarity: rarity.name, 
        ..Cw721RarityMetadata::default()
    });
    // create mint message NFT for the sender
    let mint_msg = WasmMsg::Execute {
        contract_addr: supplier_address.to_string(),
        msg: to_binary(&Cw721RarityExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: sender.clone().to_string(),
            token_uri: Some(tokens_uri[uint256_2_usize(tokens_uri_index)?].clone()),
            extension,
        }))?,
        funds: vec![],
    };

    MYSTERY_BOXS.save(deps.storage, box_id.clone(), &mystery_box)?;
    
    Ok(Response::new().add_message(mint_msg)
                .add_attribute("action", "receive_hex_randomness")
                .add_attribute("token_id", token_id)
                .add_attribute("minter", sender.to_string())
                )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // TODO: add query for MarketplaceInfo here
    match msg {
    }
}

//https://academy.binance.com/en/articles/what-are-nft-mystery-boxes-and-how-do-they-work


