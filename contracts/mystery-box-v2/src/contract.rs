#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, Addr, Api, SubMsg, QueryRequest,
    MessageInfo, Response, StdResult, WasmMsg, ReplyOn, WasmQuery,
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
    InstantiateMsg, ExecuteMsg, QueryMsg, AurandExecuteMsg,
    BoxInfo, RateDistributionMsg,
};
use crate::state::{
    CONFIG, Config,
    JOBS, Job, RateDistribution,
    MYSTERY_BOX, MysteryBox, 
    PurchasedBox, PURCHASED_BOXES
};
use crate::utils::{
    make_id,
    convert_datetime_string
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:mystery-box";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_BOX_NFT_REPLY_ID: u64 = 1;
const INSTANTIATE_ITEM_NFT_REPLY_ID: u64 = 2;

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
        item_supplier: None,
        box_supplier: None,
    })?;

    let sub_msg: Vec<SubMsg> = vec![SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: msg.box_supplier_code_id,
            msg: to_binary(&Cw721InstantiateMsg {
                name: msg.box_supplier_name,
                symbol: msg.box_supplier_symbol,
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
            code_id: msg.item_supplier_code_id,
            msg: to_binary(&Cw721RarityInstantiateMsg {
                name: msg.item_supplier_name,
                symbol: msg.item_supplier_symbol,
                minter: env.contract.address.to_string(),
            })?,
            funds: vec![],
            admin: None,
            label: String::from("Instantiate item NFT contract"),
        }
        .into(),
        id: INSTANTIATE_ITEM_NFT_REPLY_ID,
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }];

    Ok(Response::new().add_submessages(sub_msg)
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner)
        .add_attribute("aurand_address", msg.aurand_address)
        .add_attribute("item_supplier_code_id", msg.item_supplier_code_id.to_string())
        .add_attribute("box_supplier_code_id", msg.box_supplier_code_id.to_string()))
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

        INSTANTIATE_ITEM_NFT_REPLY_ID => {
            if config.item_supplier.is_some() {
                return Err(ContractError::GiftSupplierAlreadyLinked{});
            }

            config.item_supplier = Some(optional_addr_validate(deps.api, reply.contract_address)?);
            CONFIG.save(deps.storage, &config)?;
        },
        _ => {}
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateMysteryBox { 
            box_info,
            rate_distribution, 
            default_type,
        } => execute_create_mystery_box(deps,env,info,box_info,rate_distribution,default_type),

        ExecuteMsg::UpdateMysteryBox {
            prefix_uri
        } => execute_update_mystery_box(deps,env,info,prefix_uri),

        ExecuteMsg::MintBox {} => execute_mint_box(deps,env,info),

        ExecuteMsg::OpenBox {
            token_id,
        } => execute_open_box(deps,env,info,token_id),

        ExecuteMsg::ReceiveHexRandomness {
            request_id,
            randomness,
        } => execute_receive_hex_randomness(deps, info, request_id, randomness),

        ExecuteMsg::Withdraw {
            amount,
            receiver,
        } => execute_withdraw(deps,env,info,amount,receiver),
    }
}


fn optional_addr_validate(api: &dyn Api, addr: String) -> Result<Addr, ContractError> {
    let addr = api.addr_validate(&addr).map_err(|_| ContractError::InvalidAddress{})?;
    Ok(addr)
}

fn execute_create_mystery_box(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    box_info: BoxInfo,
    rate_distribution: RateDistributionMsg,
    default_type: Option<String>
) -> Result<Response, ContractError> {

    let config = CONFIG.load(deps.storage)?;

    if config.owner != info.sender {
        return Err(ContractError::Unauthorized{});
    }

    let BoxInfo{
        name,
        start_time,
        end_time,
        total_supply,
        price,
    } = box_info;

    let start_time: Timestamp = convert_datetime_string(start_time)?;
    let end_time: Timestamp = convert_datetime_string(end_time)?;

    let block_time: Timestamp = env.block.time;

    if end_time <= block_time {
        return Err(ContractError::InvalidEndTime{});
    } 

    let rate_distribution: RateDistribution = RateDistribution::new(rate_distribution, default_type)?;
    
    // list of nft id
    let tokens_id = (0u64..=total_supply).collect::<Vec<_>>();

    MYSTERY_BOX.save(deps.storage, &MysteryBox {  
        start_time, 
        end_time, 
        rate_distribution,  
        tokens_id,
        price,
        total_supply,
        name: name.clone(),
        selled: 0u64,
        prefix_uri: None,
        created_time: block_time, 
    })?;

    Ok(Response::new().add_attribute("action", "create_mystery_box")
                .add_attribute("box_name", name)
                .add_attribute("start_time", start_time.to_string())
                .add_attribute("end_time", end_time.to_string())
                .add_attribute("create_time", block_time.to_string()))
}

fn execute_update_mystery_box(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    prefix_url: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.owner != info.sender {
        return Err(ContractError::Unauthorized{});
    }

    let mut mystery_box = MYSTERY_BOX.load(deps.storage)?;

    if mystery_box.prefix_uri.is_some() {
        return Err(ContractError::CustomError {val: String::from("mystery box already updated!")});
    }

    // check if mystery box has been started
    let block_time = env.block.time;
    if block_time >= mystery_box.start_time {
        return Err(ContractError::CustomError {val: String::from("mystery box has been started!")});
    }

    mystery_box.prefix_uri = Some(prefix_url);

    MYSTERY_BOX.save(deps.storage, &mystery_box)?;

    Ok(Response::new())
}


fn execute_mint_box(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check if box supplier is not yet linked
    if config.box_supplier.is_none(){
        return Err(ContractError::BoxSupplierNotLinked{});
    }
    let box_supplier = config.box_supplier.unwrap();
    
    let mut mystery_box = MYSTERY_BOX.load(deps.storage)?;

    // check denom and get amount
    let denom = mystery_box.price.denom.clone();
    let matching_coin = info.funds.iter().find(|fund| fund.denom.eq(&denom));
    let sent_amount: Uint128 = match matching_coin {
        Some(coin) => coin.amount,
        None => {
            return Err(ContractError::CustomError {
                val: "Expected denom ".to_string() + &denom,
            });
        }
    };

    let price = mystery_box.price.amount;
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

    // generate unique token id using mystery box address, block time, current box supply
    let token_id = make_id(vec![
        env.contract.address.to_string(),
        block_time.to_string(), 
        mystery_box.selled.to_string()
    ]);

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
    
    // increase box selled by 1
    mystery_box.selled += 1;
    MYSTERY_BOX.save(deps.storage, &mystery_box)?;

    PURCHASED_BOXES.save(deps.storage, token_id.clone(), &PurchasedBox { 
        buyer: info.sender.clone(), 
        purchase_time: block_time, 
        item_id: None,
        is_opened: false 
    })?;

    Ok(Response::new().add_message(mint_msg)
            .add_attribute("action", "buy_box")
            .add_attribute("token_id", token_id)
            .add_attribute("buyer", info.sender))
}

fn execute_open_box(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check if box supplier is not yet linked
    if config.box_supplier.is_none(){
        return Err(ContractError::BoxSupplierNotLinked{});
    }
    let box_supplier = config.box_supplier.unwrap();

    let mystery_box = MYSTERY_BOX.load(deps.storage)?;

    // check if box with id exist
    if !PURCHASED_BOXES.has(deps.storage, token_id.clone()) {
        return Err(ContractError::TokenNotRecognized{});
    }
    let PurchasedBox{
        buyer,
        purchase_time,
        item_id,
        is_opened
    } = PURCHASED_BOXES.load(deps.storage, token_id.clone())?;

    // check if user is buyer
    if buyer != info.sender {
        return Err(ContractError::Unauthorized{});
    }

    // check if the box has been opened
    if is_opened {
        return Err(ContractError::BoxOpened{});
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
    let job_id = token_id.clone();
    
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
        sender: info.sender,
    })?;

    PURCHASED_BOXES.save(deps.storage, token_id.clone(), &PurchasedBox { 
        purchase_time,
        buyer: buyer.clone(), 
        item_id,
        is_opened: true
    })?;

    Ok(Response::new()
        .add_message(random_msg)
        .add_message(burn_msg)
        .add_attribute("action","open_box")
        .add_attribute("job_id",job_id)
        .add_attribute("token_id", token_id))
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

    // must link to a cw721 item contract
    if config.item_supplier.is_none() {
        return Err(ContractError::GiftSupplierNotLinked{});
    }
    let gift_supplier = config.item_supplier.unwrap();

    // check if a job with job_id exist
    if !JOBS.has(deps.storage, request_id.clone()) {
        return Err(ContractError::CustomError{val:"Job with id does't exist!".to_string()});
    }

    // get job by request id
    let Job{sender}= JOBS.load(deps.storage, request_id.clone())?;
    let box_token_id = request_id.clone();

    // get mystery box 
    let mut mystery_box = MYSTERY_BOX.load(deps.storage)?;
    
    // check if randomness valid
    if randomness.len() != 1 || randomness[0] < MIN_RANGE_RANDOM || randomness[0] > MAX_RANGE_RANDOM {
        return Err(ContractError::InvalidRandomness{});
    }

    let random_index = randomness[0] as usize;
    
    // get index of item_type based on aurand randomness
    let index = mystery_box.rate_distribution.get_item_type_index(
        random_index as u128, 
        MAX_RANGE_RANDOM as u128
    )?;

    let purity = mystery_box.rate_distribution.purity(index)?;

    mystery_box.rate_distribution.update_item_type(index)?;
    let item_type = mystery_box.rate_distribution.vec[index].clone();

    // get list of tokens id
    let tokens_id = mystery_box.tokens_id.clone();

    // random token id index using aurand randomnesss
    let tokens_id_index = random_index % tokens_id.len();

    // get item token id by index
    let item_token_id = tokens_id[tokens_id_index];

    let prefix_uri = mystery_box.prefix_uri.clone();
    // token uri made by combining prefix_uri and item_token_id
    let unique_token_uri = format!("{}{}",prefix_uri.unwrap(),item_token_id);

    // token id made by combining box id and token id 
    let unique_token_id = make_id(vec![box_token_id.clone(), item_token_id.to_string()]);

    let extension = Some(Cw721RarityMetadata {
        rarity: item_type.name,  
        purity: purity.to_string(),
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

    MYSTERY_BOX.save(deps.storage, &mystery_box)?;

    PURCHASED_BOXES.update(deps.storage, box_token_id.clone(),|p| -> StdResult<_> { Ok(PurchasedBox { 
        purchase_time: p.purchase_time,
        buyer: p.buyer.clone(), 
        item_id: Some(unique_token_id.clone()),
        is_opened: true
    })})?;

    Ok(Response::new().add_message(mint_msg)
                .add_attribute("action", "receive_hex_randomness")
                .add_attribute("token_id", unique_token_id)
                .add_attribute("token_uri", unique_token_uri)
                .add_attribute("box_token_id", box_token_id)
                .add_attribute("minter", sender))
}

fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Coin,
    receiver: String,
) -> Result<Response, ContractError> {
    
    let config = CONFIG.load(deps.storage)?;

    if config.owner != info.sender {
        return Err(ContractError::Unauthorized{});
    }

    let receiver_addr = optional_addr_validate(deps.api, receiver)?;

    // check if sufficient balance
    let contract_balance: StdResult<Coin> = deps.querier.query_balance(
        env.contract.address.to_string(),
        amount.denom.clone(),
    );
    match contract_balance {
        Ok(balance) => {
            if balance.amount < amount.amount {
                return Err(ContractError::InsufficientAmount{});
            }
        }
        Err(_) => {
            return Err(ContractError::InsufficientAmount{});
        }
    }

    // create bank msg to send amount to receiver
    let bank_msg = BankMsg::Send { 
        to_address: receiver_addr.to_string(), 
        amount: vec![amount.clone()] 
    };

    Ok(Response::new().add_message(bank_msg)
            .add_attribute("action", "withdraw")
            .add_attribute("amount", amount.to_string())
            .add_attribute("receiver", receiver_addr.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // TODO: add query for MarketplaceInfo here
    match msg {
    }
}
//https://academy.binance.com/en/articles/what-are-nft-mystery-boxes-and-how-do-they-work


