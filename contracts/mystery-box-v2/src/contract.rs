#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, Addr, Api, SubMsg, QueryRequest,
    MessageInfo, Response, StdResult, WasmMsg, ReplyOn, WasmQuery,
    Reply, Timestamp, Uint128, Coin, BankMsg
};
use cw2::set_contract_version;
use cw721::{Cw721QueryMsg, Expiration as Cw721Expiration};
use cw721_rarity::{
    MintMsg as Cw721RarityMintMsg,
    ExecuteMsg as Cw721RarityExecuteMsg,
    InstantiateMsg as Cw721RarityInstantiateMsg,
    Metadata as Cw721RarityMetadata,
};
use cw721_box::{
    MintMsg as Cw721MintMsg,
    InstantiateMsg as Cw721InstantiateMsg,
    ExecuteMsg as Cw721ExecuteMsg
};
use cw_utils::parse_reply_instantiate_data;

use crate::error::ContractError;
use crate::msg::{
    InstantiateMsg, ExecuteMsg, QueryMsg, AurandExecuteMsg,
    BoxInfo, RateDistributionMsg, LinkedArress,
};
use crate::state::{
    CONFIG, Config,
    JOBS, Job, RateDistribution,
    MYSTERY_BOX, MysteryBox, 
    PurchasedBox, PURCHASED_BOXES, MYSTERY_BOX_HISTORY
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

const NUMBER_OF_RANDOM: u32 = 2u32;
const MIN_RANGE_RANDOM: i32 = 0i32;
const MAX_RANGE_RANDOM: i32 = 10000i32;

const SECONDS_PER_HOUR: u64 = 3600u64;

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
                return Err(ContractError::ItemSupplierAlreadyLinked{});
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

        ExecuteMsg::ReRequestRandomness {
            token_id
        } => execute_re_request_randomness(deps, info, env, token_id),

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
    // check if sender is owner of this contract
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized{});
    }

    // current block timestamp
    let block_time: Timestamp = env.block.time;

    // check if mystery box has been initialized
    if let Some(mystery_box) = MYSTERY_BOX.may_load(deps.storage)?{
        // if mystery-box has expired, save current mystery-box to history and create the new one
        // else return error
        if mystery_box.end_time < block_time {
            MYSTERY_BOX_HISTORY.save(deps.storage, mystery_box.id.clone(), &mystery_box)?;
        }else{
            return Err(ContractError::MysteryBoxInOperation{});
        }
    }

    let BoxInfo{
        name,
        description,
        start_time,
        end_time,
        total_supply,
        max_item_supply,
        replacement,
        price
    } = box_info;

    // convert start_time and end_time string to Timestamp
    let start_time: Timestamp = convert_datetime_string(start_time)?;
    let end_time: Timestamp = convert_datetime_string(end_time)?;

    // check if the end time is past and start_time <= end_time
    if end_time <= block_time && start_time <= end_time {
        return Err(ContractError::InvalidTime{});
    } 

    // generate unique id using contract address and current block timestamp
    let id = make_id(vec![
        env.contract.address.to_string(),
        block_time.to_string(),
    ]);

    // init rate distribution for mystery box
    let rate_distribution: RateDistribution = RateDistribution::new(rate_distribution, default_type)?;
    
    // list of nft id from 0 to total_supply
    let tokens_id = (0u64..=total_supply).collect::<Vec<_>>();

    MYSTERY_BOX.save(deps.storage, &MysteryBox {  
        description,
        start_time, 
        end_time, 
        rate_distribution,  
        tokens_id,
        total_supply,
        replacement,
        price,
        id: id.clone(),
        name: name.clone(),
        max_item_supply: if max_item_supply.is_some(){
            max_item_supply.unwrap()
        }else{
            u64::MAX
        },
        prefix_uri: None,
        created_time: block_time, 
    })?;

    Ok(Response::new().add_attribute("action", "create_mystery_box")
                .add_attribute("id", id)
                .add_attribute("name", name)
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
    // check if sender is owner of this contract
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized{});
    }

    // check if mystery box has been initialized
    let mut mystery_box = if let Some(mb) = MYSTERY_BOX.may_load(deps.storage)?{
        mb
    }else{
        return Err(ContractError::MysteryBoxNotInitialized{});
    };

    // check if mystery box already updated
    if mystery_box.prefix_uri.is_some() {
        return Err(ContractError::CustomError {val: String::from("mystery box already updated!")});
    }

    // check if mystery box has expired
    let block_time = env.block.time;
    if block_time > mystery_box.end_time {
        return Err(ContractError::CustomError {val: String::from("mystery box has expired!")});
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
    
    // check if mystery box has been initialized
    let mut mystery_box = if let Some(mb) = MYSTERY_BOX.may_load(deps.storage)?{
        mb
    }else{
        return Err(ContractError::MysteryBoxNotInitialized{});
    };

    //  check if mystery box sold-out
    if mystery_box.max_item_supply == 0 {
        return Err(ContractError::SoldOut{});
    }

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

    // if send amount smaller than mystery-box price, return fail
    let price = mystery_box.price.amount;
    if sent_amount < price {
        return Err(ContractError::CustomError{
            val: String::from("Insufficient fee! required ")                        
            + &price.to_string() + &denom}); 
    }

    // check if mystery box expired
    let block_time = env.block.time;
    if block_time >= mystery_box.end_time {
        return Err(ContractError::MysteryBoxExpired{});
    }

    // prefix_token_id is box id
    // can be used to check which mystery box the NFT item belongs to
    let prefix_token_id = mystery_box.id.clone();
    // generate suffix token id using mystery box address, block time, current box supply
    let suffix_token_id = make_id(vec![
        env.contract.address.to_string(),
        block_time.to_string(), 
        mystery_box.max_item_supply.to_string()
    ]);
    
    // unique token id is combine of prefix_token_id and suffix_token_id
    let token_id = format!("{}_{}",prefix_token_id,suffix_token_id);

    // create mint message NFT for the sender
    let mint_msg = WasmMsg::Execute {
        contract_addr: box_supplier.to_string(),
        msg: to_binary(&Cw721ExecuteMsg::Mint(Cw721MintMsg {
            token_id: token_id.clone(),
            owner: info.sender.to_string(),
            token_uri: None,
            extension: None,
        }))?,
        funds: vec![],
    };
    
    // increase box selled by 1
    mystery_box.max_item_supply -= 1;
    MYSTERY_BOX.save(deps.storage, &mystery_box)?;

    // update purchased box history
    PURCHASED_BOXES.save(deps.storage, token_id.clone(), &PurchasedBox { 
        is_opened: false,
        open_time: None,
        is_received_randomness: false,
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

    // check if mystery box has been initialized
    let mystery_box = if let Some(mb) = MYSTERY_BOX.may_load(deps.storage)?{
        mb
    }else{
        return Err(ContractError::MysteryBoxNotInitialized{});
    };
    
    // check if mystery box has been updated
    if mystery_box.prefix_uri.is_none(){
        return Err(ContractError::MysteryBoxNotUpdated{});
    }

    // check if box with id exist
    if !PURCHASED_BOXES.has(deps.storage, token_id.clone()) {
        return Err(ContractError::TokenNotRecognized{});
    }
    let PurchasedBox{
        is_opened,
        open_time: _,
        is_received_randomness
    } = PURCHASED_BOXES.load(deps.storage, token_id.clone())?;

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
    };

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

    // generate request id for receiving randomness
    let request_id = token_id.clone();
    
    // request randomness from aurand contract
    let random_msg = WasmMsg::Execute {
        contract_addr: config.aurand_address.to_string(),
        msg: to_binary(&AurandExecuteMsg::RequestIntRandomness { 
                        request_id: request_id.clone(),
                        num: NUMBER_OF_RANDOM,
                        min: MIN_RANGE_RANDOM,
                        max: MAX_RANGE_RANDOM,
                    })?,
        funds: info.funds,
    };

    let burn_msg = WasmMsg::Execute {
        contract_addr: box_supplier.to_string(),
        msg: to_binary(&Cw721ExecuteMsg::Burn{
            token_id: token_id.clone()
        })?,
        funds: vec![],
    };

    // save request open box job, wait for randomness
    JOBS.save(deps.storage, request_id.clone(), &Job{
        sender: info.sender,
    })?;

    PURCHASED_BOXES.save(deps.storage, token_id.clone(), &PurchasedBox { 
        is_opened: true,
        open_time: Some(block_time),
        is_received_randomness
    })?;

    Ok(Response::new()
        .add_message(random_msg)
        .add_message(burn_msg)
        .add_attribute("action","open_box")
        .add_attribute("request_id",request_id)
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
        return Err(ContractError::ItemSupplierNotLinked{});
    }
    let item_supplier = config.item_supplier.unwrap();

    // check if mystery box has been initialized
    let mut mystery_box = if let Some(mb) = MYSTERY_BOX.may_load(deps.storage)?{
        mb
    }else{
        return Err(ContractError::MysteryBoxNotInitialized{});
    };

    // check if a job with job_id exist
    if !JOBS.has(deps.storage, request_id.clone()) {
        return Err(ContractError::CustomError{val:"Job with id does't exist!".to_string()});
    }

    // get job by request id
    let Job{sender}= if let Some(job) = JOBS.may_load(deps.storage, request_id.clone())?{
        job
    }else{
        return Err(ContractError::JobNotExist{});
    };
    
    // request id is also id for user's purchased box
    // it will be use to provide unique token id for nft item
    let item_token_id = request_id.clone();

    // check if a box with an ID exists and hasn't been opened
    let purchased_box = if let Some(pb) = PURCHASED_BOXES.may_load(
        deps.storage, 
        item_token_id.clone()
    )?{
        if !pb.is_opened {
            return Err(ContractError::BoxNotOpened{})
        }
        pb
    }else{
        return Err(ContractError::TokenNotRecognized{});
    };

    // check if randomness valid
    if randomness.len() != 2 || 
    randomness[0] < MIN_RANGE_RANDOM || randomness[0] > MAX_RANGE_RANDOM ||
    randomness[1] < MIN_RANGE_RANDOM || randomness[1] > MAX_RANGE_RANDOM {
        return Err(ContractError::InvalidRandomness{});
    }

    // randomness received from aurand contract 
    // it between MIN_RANGE_RANDOM..MAX_RANGE_RANDOM
    let (random_type, random_index) = (randomness[0] as usize, randomness[1] as usize);
    
    // get index of item_type based on aurand randomness
    let index = mystery_box.rate_distribution.get_item_type_index(
        random_type as u128, 
        MAX_RANGE_RANDOM as u128
    )?;

    // get current purity of item_type at specified index
    let purity = mystery_box.rate_distribution.purity(index)?;

    // update supply and rate for item_type at specifed index
    mystery_box.rate_distribution.update_item_type(index)?;

    // get item_type by index
    let item_type = mystery_box.rate_distribution.vec[index].clone();

    // get a list of token ids provided
    let tokens_id = mystery_box.tokens_id.clone();

    // random tokens id index using aurand randomnesss
    let tokens_id_index = random_index % tokens_id.len();

    // get item token id by index
    let token_id = tokens_id[tokens_id_index];

    // prefix_uri of NFTs resource collection
    let prefix_uri = mystery_box.prefix_uri.clone();
    
    // token uri made by combining prefix_uri and token_id
    let unique_token_uri = format!("{}{}",prefix_uri.unwrap(),token_id);

    // if replacement == true, replace selected uri from tokens_id
    // to make all minted Item NFTs unique
    if mystery_box.replacement {
        mystery_box.remove_token_id(tokens_id_index);
    }

    // cw721rarity metadata
    let extension = Some(Cw721RarityMetadata {
        rarity: item_type.name,  
        purity: purity.to_string(),
        ..Cw721RarityMetadata::default()
    });

    // create mint message NFT for the sender
    let mint_msg = WasmMsg::Execute {
        contract_addr: item_supplier.to_string(),
        msg: to_binary(&Cw721RarityExecuteMsg::Mint(Cw721RarityMintMsg {
            token_id: item_token_id.clone(), // unique token id
            owner: sender.clone().to_string(),
            token_uri: Some(unique_token_uri.clone()), // unique token uri
            extension,
        }))?,
        funds: vec![],
    };

    MYSTERY_BOX.save(deps.storage, &mystery_box)?;
    PURCHASED_BOXES.save(deps.storage, item_token_id.clone(),&PurchasedBox{ 
        is_opened: true,
        open_time: purchased_box.open_time,
        is_received_randomness: true,
    })?;

    Ok(Response::new().add_message(mint_msg)
                .add_attribute("action", "receive_hex_randomness")
                .add_attribute("token_id", item_token_id)
                .add_attribute("token_uri", unique_token_uri)
                .add_attribute("minter", sender))
}

fn execute_re_request_randomness(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    token_id: String
) -> Result<Response, ContractError> {
    // check if box with id exist
    if !PURCHASED_BOXES.has(deps.storage, token_id.clone()) {
        return Err(ContractError::TokenNotRecognized{});
    }
    let PurchasedBox{
        is_opened,
        open_time,
        is_received_randomness
    } = PURCHASED_BOXES.load(deps.storage, token_id.clone())?;

    // only allow to re-request randomness if box was opened but not yet receive randomness
    if !(is_opened == true && is_received_randomness == false) {
        return Err(ContractError::InvalidCondition{});
    }

    let open_time = open_time.unwrap();
    let block_time = env.block.time;

    // only allow to re-request randomness if box was opened more than 1 hour ago
    if block_time < open_time.plus_seconds(SECONDS_PER_HOUR) {
        return Err(ContractError::InvalidCondition{});
    }

    // generate request id for receiving randomness
    let request_id = token_id.clone();
    
    // request randomness from aurand contract
    let random_msg = WasmMsg::Execute {
        contract_addr: CONFIG.load(deps.storage)?.aurand_address.to_string(),
        msg: to_binary(&AurandExecuteMsg::RequestIntRandomness { 
                        request_id: request_id.clone(),
                        num: NUMBER_OF_RANDOM,
                        min: MIN_RANGE_RANDOM,
                        max: MAX_RANGE_RANDOM,
                    })?,
        funds: info.funds,
    };

    Ok(Response::new().add_message(random_msg)
        .add_attribute("action", "request_randomness")
        .add_attribute("token_id", token_id))
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

    // check if contract sufficient balance
    let contract_balance: StdResult<Coin> = deps.querier.query_balance(
        env.contract.address.to_string(),
        amount.denom.clone(),
    );
    match contract_balance {
        Ok(balance) => {
            // if current balance smaller than required amount
            if balance.amount < amount.amount {
                return Err(ContractError::InsufficientAmount{});
            }
        }
        // if not found balance for denom
        Err(_) => {
            return Err(ContractError::InsufficientAmount{});
        }
    }

    // create bank msg to send amount from contract to receiver
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // TODO: add query for MarketplaceInfo here
    match msg {
        QueryMsg::GetMysteryBoxInformation{} => to_binary(&query_mystery_box_information(deps)?),
        QueryMsg::GetMysteryBoxHistoryById { id } => to_binary(&query_mystery_box_history_by_id(deps, id)?),
        QueryMsg::GetLinkedAddres{} => to_binary(&query_linked_address(deps)?),
    }
}

pub fn query_mystery_box_information(deps: Deps) -> StdResult<Option<MysteryBox>> {
    Ok(MYSTERY_BOX.may_load(deps.storage)?)
}

pub fn query_mystery_box_history_by_id(deps: Deps, id: String) -> StdResult<Option<MysteryBox>> {
    Ok(MYSTERY_BOX_HISTORY.may_load(deps.storage, id)?)
}

pub fn query_linked_address(deps: Deps) -> StdResult<LinkedArress> {
    let config = CONFIG.load(deps.storage)?;

    Ok(LinkedArress { 
        aurand_address: config.aurand_address, 
        box_supplier_address: config.box_supplier, 
        item_supplier_address: config.item_supplier,
    })
}

