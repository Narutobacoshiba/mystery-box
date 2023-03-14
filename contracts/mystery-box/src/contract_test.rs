#[cfg(test)]
mod unit_tests {
    use crate::contract::{instantiate, execute, query, reply};
    use nois::{NoisCallback, ProxyExecuteMsg};

    use crate::error::ContractError;
    use crate::msg::{
        ExecuteMsg, InstantiateMsg, CallbackExecuteMsg, QueryMsg,
        PendingCommitmentsQuery, BotInfoQuery, CommitmentsQuery, 
        NumberOfCommitmentQuery, ConfigsQuery,
    };
    use crate::utils::{
        make_commit_id,
    };
    use crate::state::{
        COMMITMENTS, Commitment, PENDING_COMMITMENTS, DataRequest
    };

    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{
        Uint128, OwnedDeps, Env, Response,BlockInfo, ContractInfo, Timestamp, 
        Addr, SubMsg, Coin, coins, to_binary, WasmMsg, ReplyOn, HexBinary, BankMsg,
        SubMsgResult,
    };

    const NOIS_CALLBACK_REPLY_ID: u64 = 1;
    const COMMITMENT_CALLBACK_REPLY_ID: u64 = 2;

    const CONTRACT_ADDR: &str = "contract";
    const CREATOR: &str = "creator";
    const BOT: &str = "bot";
    const USER: &str = "user";
    const NOIS_PROXY_ADDR: &str = "aura19z2hv8l87qwg8nnq6v76efjm2rm78rkdghq4rkxfgqrzv3usw8lq26rmwt";

    const TIME_EXPIRED: u64 = 5u64;
    const TIME_PER_BLOCK: u64 = 5u64;
    const DENOM: &str = "ueaura";
    const FEE: u128 = 300u128;
    const NOIS_FEE: u128 = 300u128;
    const CALLBACK_LIMIT_GAS: u64 = 1500000u64; 
    const MAX_CALLBACK: u32 = 5u32;

    fn default_setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            nois_proxy: NOIS_PROXY_ADDR.to_string(),
            time_expired: TIME_EXPIRED,
            time_per_block: TIME_PER_BLOCK,
            bounty_denom: String::from(DENOM),
            fee: Uint128::from(FEE),
            nois_fee: Uint128::from(NOIS_FEE),
            callback_limit_gas: CALLBACK_LIMIT_GAS,
            max_callback: MAX_CALLBACK,
        };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        
        return deps;
    }

    // EXECUTE

    /***** Instantiate *****/
    
    #[test]
    fn instantiate_fail_with_invalid_noisproxy() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            nois_proxy: "".to_string(),
            time_expired: 5,
            time_per_block: 5,
            bounty_denom: "ueaura".to_string(),
            fee: Uint128::from(300u128),
            nois_fee: Uint128::from(300u128),
            callback_limit_gas: 150000,
            max_callback: MAX_CALLBACK,
        };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        match res {
            ContractError::InvalidProxyAddress {} => {},
            _ => panic!(),
        };
    }


    #[test]
    fn instantiate_works() {
        default_setup();
    }

    /***** Set Configs *****/
    #[test]
    fn set_configs_fail_with_unauthorized() {
        let mut deps = default_setup();

        let request_set_configs = ExecuteMsg::SetConfigs{
            bounty_denom: "ueaura".to_string(),
            fee: Uint128::from(300u128),
            callback_limit_gas: 150000,
            max_callback: 5u32,
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(USER, &[]), request_set_configs).unwrap_err();
        match res {
            ContractError::Unauthorized{} => {},
            _ => panic!(),
        };
    }
    

    #[test]
    fn set_configs_success() {
        let mut deps = default_setup();

        let bounty_denom = String::from("ueaura");
        let fee = Uint128::from(300u128);
        let callback_limit_gas = 150000;
        let max_callback= 5u32;

        let request_set_configs = ExecuteMsg::SetConfigs{
            bounty_denom,
            fee,
            callback_limit_gas,
            max_callback
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(CREATOR, &[]), request_set_configs).unwrap();
        assert_eq!(res, Response::new().add_attribute("action","set_config")
                                    .add_attribute("bounty_denom", "ueaura".to_string())
                                    .add_attribute("fee", fee)
                                    .add_attribute("callback_limit_gas", callback_limit_gas.to_string())
                                    .add_attribute("max_callback", max_callback.to_string())
                                    .add_attribute("owner",  CREATOR));
    }

    /***** Set Time Configs *****/
    #[test]
    fn set_time_configs_fail_with_unauthorized() {
        let mut deps = default_setup();

        let request_set_time_configs = ExecuteMsg::SetTimeConfigs{
            time_expired: 5,
            time_per_block: 5
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(USER, &[]), request_set_time_configs).unwrap_err();
        match res {
            ContractError::Unauthorized{} => {},
            _ => panic!(),
        };
    }

    #[test]
    fn set_time_configs_success() {
        let mut deps = default_setup();

        let time_expired = 5u64;
        let time_per_block = 5u64;

        let request_set_time_configs = ExecuteMsg::SetTimeConfigs{
            time_expired,
            time_per_block
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(CREATOR, &[]), request_set_time_configs).unwrap();
        assert_eq!(res, Response::new().add_attribute("action","set_time_config")
                                    .add_attribute("time_expired", time_expired.to_string())
                                    .add_attribute("time_per_block", time_per_block.to_string())
                                    .add_attribute("owner",  CREATOR));
    }

    /***** Set Nois Configs *****/
    #[test]
    fn set_nois_configs_fail_with_unauthorized() {
        let mut deps = default_setup();

        let request_set_nois_configs = ExecuteMsg::SetNoisConfigs{
            nois_proxy: String::from(NOIS_PROXY_ADDR),
            nois_fee: Uint128::from(300u128),
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(USER, &[]), request_set_nois_configs).unwrap_err();
        match res {
            ContractError::Unauthorized{} => {},
            _ => panic!(),
        };
    }
    

    #[test]
    fn set_nois_configs_fail_with_invalid_noisproxy() {
        let mut deps = default_setup();

        let request_set_nois_configs = ExecuteMsg::SetNoisConfigs{
            nois_proxy: String::from(""),
            nois_fee: Uint128::from(300u128),
        };


        let res = execute(deps.as_mut(), mock_env(), mock_info(USER, &[]), request_set_nois_configs).unwrap_err();
        match res {
            ContractError::InvalidAddress {} => {},
            _ => panic!(),
        };
    }

    #[test]
    fn set_nois_configs_success() {
        let mut deps = default_setup();

        let nois_proxy = String::from(NOIS_PROXY_ADDR);
        let nois_fee = Uint128::from(300u128);

        let request_set_nois_configs = ExecuteMsg::SetNoisConfigs{
            nois_proxy: nois_proxy.clone(),
            nois_fee,
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(CREATOR, &[]), request_set_nois_configs).unwrap();
        assert_eq!(res, Response::new().add_attribute("action","set_nois_config")
                                    .add_attribute("nois_proxy", nois_proxy)
                                    .add_attribute("nois_fee", nois_fee)
                                    .add_attribute("owner",  CREATOR));
    }

    /***** Register Bot *****/
    fn register_bot(deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>) {
        let hashed_api_key = String::from("elv0PecZXTaHIbs+3PvmJIz2BO9mtakvSpznFkhgfe/EtmPPCVAqpBDIT6ZeQ3TEsCvdxymXnDuSPKoqjlxZ/Q==");
        let moniker = String::from("test bot");

        let request_register_bot = ExecuteMsg::RegisterBot {
            hashed_api_key: hashed_api_key.clone(),
            moniker: moniker.clone(),
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info(BOT, &[]), request_register_bot).unwrap();
        assert_eq!(res, Response::new().add_attribute("action","register_bot")
                                .add_attribute("hashed_api_key", hashed_api_key)
                                .add_attribute("moniker", moniker)
                                .add_attribute("bot_address", BOT));
    }

    #[test]
    fn register_bot_success() {
        let mut deps = default_setup();

        register_bot(&mut deps);
    }


    #[test]
    fn register_bot_fail_with_duplicate_address() {
        let mut deps = default_setup();

        let hashed_api_key = String::from("test hashed api key");
        let moniker = String::from("test bot");

        let request_register_bot = ExecuteMsg::RegisterBot {
            hashed_api_key: hashed_api_key.clone(),
            moniker: moniker.clone(),
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info(BOT, &[]), request_register_bot.clone()).unwrap();
        assert_eq!(res, Response::new().add_attribute("action","register_bot")
                            .add_attribute("hashed_api_key", hashed_api_key)
                            .add_attribute("moniker", moniker)
                            .add_attribute("bot_address", BOT));
        
        let res = execute(deps.as_mut(), mock_env(), mock_info(BOT, &[]), request_register_bot).unwrap_err();
        match res {
            ContractError::AddressAlreadyRegistered{} => {},
            _ => panic!(),
        };
    }

    /***** Update Bot *****/
    #[test]
    fn update_bot_success() {
        let mut deps = default_setup();

        let env: Env = Env {
            block: BlockInfo {
                height: 0,
                time: Timestamp::from_seconds(0),
                chain_id: "euphoria-2".to_string()
            },
            contract: ContractInfo {
                address: Addr::unchecked(CONTRACT_ADDR)
            },
            transaction: None,
        };
        let hashed_api_key = String::from("test hashed api key");
        let moniker = String::from("test bot");

        let request_register_bot = ExecuteMsg::RegisterBot {
            hashed_api_key: hashed_api_key.clone(),
            moniker: moniker.clone(),
        };
        let res = execute(deps.as_mut(), env, mock_info(BOT, &[]), request_register_bot).unwrap();
        assert_eq!(res, Response::new().add_attribute("action","register_bot")
                                .add_attribute("hashed_api_key", hashed_api_key)
                                .add_attribute("moniker", moniker)
                                .add_attribute("bot_address", BOT));


        let env: Env = Env {
            block: BlockInfo {
                height: 2,
                time: Timestamp::from_seconds(10),
                chain_id: "euphoria-2".to_string()
            },
            contract: ContractInfo {
                address: Addr::unchecked(CONTRACT_ADDR)
            },
            transaction: None,
        };

        let hashed_api_key = String::from("test hashed api key 2");
        let moniker = String::from("test bot 2");

        let request_update_bot = ExecuteMsg::UpdateBot {
            hashed_api_key: hashed_api_key.clone(),
            moniker: moniker.clone(),
        };
        let res = execute(deps.as_mut(), env, mock_info(BOT, &[]), request_update_bot).unwrap();
        assert_eq!(res, Response::new().add_attribute("action","update_bot")
        .add_attribute("hashed_api_key", hashed_api_key)
        .add_attribute("moniker", moniker)
        .add_attribute("bot_address", BOT));
    }

    #[test]
    fn update_bot_fail_with_unregister_bot() {
        let mut deps = default_setup();

        let request_update_bot = ExecuteMsg::UpdateBot {
            hashed_api_key: "hashed_api_key".to_string(),
            moniker: "test bot".to_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info(BOT, &[]), request_update_bot).unwrap_err();
        match res {
            ContractError::UnregisteredAddress{} => {},
            _ => panic!(),
        };
    }

    #[test]
    fn update_bot_fail_with_to_many_update_action() {
        let mut deps = default_setup();

        let env: Env = Env {
            block: BlockInfo {
                height: 2,
                time: Timestamp::from_seconds(10),
                chain_id: "euphoria-2".to_string()
            },
            contract: ContractInfo {
                address: Addr::unchecked(CONTRACT_ADDR)
            },
            transaction: None,
        };
        let hashed_api_key = String::from("test hashed api key");
        let moniker = String::from("test bot");

        let request_register_bot = ExecuteMsg::RegisterBot {
            hashed_api_key: hashed_api_key.clone(),
            moniker: moniker.clone(),
        };
        let res = execute(deps.as_mut(), env, mock_info(BOT, &[]), request_register_bot).unwrap();
        assert_eq!(res, Response::new().add_attribute("action","register_bot")
                                .add_attribute("hashed_api_key", hashed_api_key)
                                .add_attribute("moniker", moniker)
                                .add_attribute("bot_address", BOT));


        let env: Env = Env {
            block: BlockInfo {
                height: 3,
                time: Timestamp::from_seconds(15),
                chain_id: "euphoria-2".to_string()
            },
            contract: ContractInfo {
                address: Addr::unchecked(CONTRACT_ADDR)
            },
            transaction: None,
        };

        let hashed_api_key = String::from("test hashed api key");
        let moniker = String::from("test bot");

        let request_update_bot = ExecuteMsg::UpdateBot {
            hashed_api_key: hashed_api_key.clone(),
            moniker: moniker.clone(),
        };
        let res = execute(deps.as_mut(), env, mock_info(BOT, &[]), request_update_bot).unwrap_err();
        match res {
            ContractError::ToManyAction{} => {},
            _ => panic!(),
        };
    }

    /***** Remove Bot *****/
    #[test]
    fn remove_bot_success() {
        let mut deps = default_setup();

        register_bot(&mut deps);

        let request_remove_bot = ExecuteMsg::RemoveBot{
            address: BOT.to_string()
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info(CREATOR, &[]), request_remove_bot).unwrap();
        assert_eq!(res, Response::new().add_attribute("action","remove_bot")
                            .add_attribute("bot_addr", String::from(BOT))
                            .add_attribute("owner", String::from(CREATOR)));
    }

    #[test]
    fn remove_bot_fail_with_unauthorized() {
        let mut deps = default_setup();

        register_bot(&mut deps);

        let request_remove_bot = ExecuteMsg::RemoveBot{
            address: BOT.to_string()
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info(USER, &[]), request_remove_bot).unwrap_err();
        match res {
            ContractError::Unauthorized{} => {},
            _ => panic!(),
        };
    }

    #[test]
    fn remove_bot_fail_with_invalid_bot_address() {
        let mut deps = default_setup();

        register_bot(&mut deps);

        let request_remove_bot = ExecuteMsg::RemoveBot{
            address: "".to_string()
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info(CREATOR, &[]), request_remove_bot);
        match res {
            Err(_) =>{},
            Ok(_) => panic!(),
        };
    }

    /***** Request Randomness *****/
    fn request_hex_randomness(deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>) {
        let request_id: String = String::from("test id 1");
        let request_hex_randomness = ExecuteMsg::RequestHexRandomness {
            request_id: request_id.clone(),
            num: 1
        };

        let env: Env = Env {
            block: BlockInfo {
                height: 0,
                time: Timestamp::from_seconds(1675739151),
                chain_id: "euphoria-2".to_string()
            },
            contract: ContractInfo {
                address: Addr::unchecked(CONTRACT_ADDR)
            },
            transaction: None,
        };
        let coin: Coin = Coin{
            denom: "ueaura".to_string(),
            amount: Uint128::from(600u128),
        };
        let res = execute(deps.as_mut(), env, mock_info(USER, &[coin]), request_hex_randomness).unwrap();
        
        let nonce: u64 = 0u64;
        let commit_id = make_commit_id(USER.to_string(), nonce);

        let sub_msg: SubMsg = SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: NOIS_PROXY_ADDR.to_string(),
                msg: to_binary(&ProxyExecuteMsg::GetNextRandomness { 
                                job_id: commit_id.clone() }).unwrap(),
                funds: coins(NOIS_FEE, DENOM),
            }
            .into(),
            id: NOIS_CALLBACK_REPLY_ID,
            gas_limit: None,
            reply_on: ReplyOn::Always,
        };

        assert_eq!(res, Response::new().add_submessage(sub_msg)
                            .add_attribute("action", "request_randomness")
                            .add_attribute("commitment_id",commit_id)
                            .add_attribute("request_id",request_id)
                            .add_attribute("user", String::from(USER)));
    }

    fn request_int_randomness(deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>) {
        let request_id: String = String::from("test id 1");
        let request_int_randomness = ExecuteMsg::RequestIntRandomness {
            request_id: request_id.clone(),
            num: 1,
            min: 0,
            max: 255,
        };
        let env: Env = Env {
            block: BlockInfo {
                height: 0,
                time: Timestamp::from_seconds(1675739151),
                chain_id: "euphoria-2".to_string()
            },
            contract: ContractInfo {
                address: Addr::unchecked(CONTRACT_ADDR)
            },
            transaction: None,
        };
        let coin: Coin = Coin{
            denom: "ueaura".to_string(),
            amount: Uint128::from(600u128),
        };
        let res = execute(deps.as_mut(), env, mock_info(USER, &[coin]), request_int_randomness).unwrap();

        let nonce: u64 = 0u64;
        let commit_id = make_commit_id(USER.to_string(), nonce);

        let sub_msg: SubMsg = SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: NOIS_PROXY_ADDR.to_string(),
                msg: to_binary(&ProxyExecuteMsg::GetNextRandomness { 
                                job_id: commit_id.clone() }).unwrap(),
                funds: coins(NOIS_FEE, DENOM),
            }
            .into(),
            id: NOIS_CALLBACK_REPLY_ID,
            gas_limit: None,
            reply_on: ReplyOn::Always,
        };

        assert_eq!(res, Response::new().add_submessage(sub_msg)
                                .add_attribute("action", "request_randomness")
                                .add_attribute("commitment_id",commit_id)
                                .add_attribute("request_id",request_id)
                                .add_attribute("user", String::from(USER)));
    }

    #[test]
    fn request_hex_randomness_success() {
        let mut deps = default_setup();
        
        request_hex_randomness(&mut deps);
    }

    #[test]
    fn request_int_randomness_success(){
        let mut deps = default_setup();

        request_int_randomness(&mut deps);
    }


    #[test]
    fn request_int_randomness_success_multi_time() {
        let mut deps = default_setup();

        let request_id: String = String::from("test id 1");
        let request_int_randomness = ExecuteMsg::RequestIntRandomness {
            request_id: request_id.clone(),
            num: 1,
            min: 0,
            max: 255,
        };

        let coin: Coin = Coin{
            denom: "ueaura".to_string(),
            amount: Uint128::from(600u128),
        };

        // first time
        let res = execute(deps.as_mut(), mock_env(), mock_info(USER, &[coin]), request_int_randomness).unwrap();

        let nonce: u64 = 0u64;
        let commit_id = make_commit_id(USER.to_string(), nonce);

        let sub_msg: SubMsg = SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: NOIS_PROXY_ADDR.to_string(),
                msg: to_binary(&ProxyExecuteMsg::GetNextRandomness { 
                                job_id: commit_id.clone() }).unwrap(),
                funds: coins(NOIS_FEE, DENOM),
            }
            .into(),
            id: NOIS_CALLBACK_REPLY_ID,
            gas_limit: None,
            reply_on: ReplyOn::Always,
        };

        assert_eq!(res, Response::new().add_submessage(sub_msg)
                            .add_attribute("action", "request_randomness")
                            .add_attribute("commitment_id",commit_id)
                            .add_attribute("request_id",request_id)
                            .add_attribute("user", String::from(USER)));

        
        //second time
        let request_id: String = String::from("test id 2");
        let request_int_randomness = ExecuteMsg::RequestIntRandomness {
            request_id: request_id.clone(),
            num: 1,
            min: 0,
            max: 255,
        };

        let coin: Coin = Coin{
            denom: "ueaura".to_string(),
            amount: Uint128::from(600u128),
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info(USER, &[coin]), request_int_randomness).unwrap();

        let nonce: u64 = 1u64;
        let commit_id = make_commit_id(USER.to_string(), nonce);

        let sub_msg: SubMsg = SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: NOIS_PROXY_ADDR.to_string(),
                msg: to_binary(&ProxyExecuteMsg::GetNextRandomness { 
                                job_id: commit_id.clone() }).unwrap(),
                funds: coins(NOIS_FEE, DENOM),
            }
            .into(),
            id: NOIS_CALLBACK_REPLY_ID,
            gas_limit: None,
            reply_on: ReplyOn::Always,
        };

        assert_eq!(res, Response::new().add_submessage(sub_msg)
                        .add_attribute("action", "request_randomness")
                        .add_attribute("commitment_id",commit_id)
                        .add_attribute("request_id",request_id)
                        .add_attribute("user", String::from(USER)));

    }

    #[test]
    fn request_int_randomness_fail_with_invalid_num() {
        let mut deps = default_setup();
        
        let request_int_randomness = ExecuteMsg::RequestIntRandomness {
            request_id: "test id 1".to_string(),
            num: 0,
            min: 0,
            max: 255,
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(USER, &[]), request_int_randomness).unwrap_err();

        match res {
            ContractError::CustomError{val: v} => {assert_eq!(v, "number of randomness must be in range 1..256".to_string())},
            _ => panic!(),
        };
    }

    #[test]
    fn request_int_randomness_fail_with_invalid_denom() {
        let mut deps = default_setup();

        let request_int_randomness = ExecuteMsg::RequestIntRandomness {
            request_id: "test id 1".to_string(),
            num: 1,
            min: 0,
            max: 255,
        };

        let coin: Coin = Coin{
            denom: "eaura".to_string(),
            amount: Uint128::from(1u128),
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info(USER, &[coin]), request_int_randomness).unwrap_err();

        match res {
            ContractError::CustomError{val: v} => {assert_eq!(v, "Expected denom ueaura".to_string())},
            _ => panic!(),
        };
    }

    #[test]
    fn request_int_randomness_fail_with_insufficent_fee() {
        let mut deps = default_setup();

        let request_int_randomness = ExecuteMsg::RequestIntRandomness {
            request_id: "test id 1".to_string(),
            num: 1,
            min: 0,
            max: 255,
        };

        let coin: Coin = Coin{
            denom: "ueaura".to_string(),
            amount: Uint128::from(1u128),
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info(USER, &[coin]), request_int_randomness).unwrap_err();

        match res {
            ContractError::CustomError{val: v} => {assert_eq!(v, "Insufficient fee! required 600ueaura".to_string())},
            _ => panic!(),
        };
    }

    /***** Add Randomness *****/

    const RANDOM_VALUE_TEST: &str = r#"{"method":"generateSignedIntegers","hashedApiKey":"elv0PecZXTaHIbs+3PvmJIz2BO9mtakvSpznFkhgfe/EtmPPCVAqpBDIT6ZeQ3TEsCvdxymXnDuSPKoqjlxZ/Q==","n":32,"min":0,"max":255,"replacement":true,"base":10,"pregeneratedRandomization":null,"data":[127,12,177,76,70,175,6,221,126,220,251,62,125,122,39,146,236,173,173,240,28,197,116,202,130,36,88,171,55,232,75,86],"license":{"type":"developer","text":"Random values licensed strictly for development and testing only","infoUrl":null},"licenseData":null,"userData":null,"ticketData":null,"completionTime":"2023-02-07 03:05:57Z","serialNumber":489}"#;
    const SIGNATURE_TEST: &str = "kITMbucgIRih+606JH/zfYDIBqOYbB4VEyjCkLJIteIqMMRMZrFRBPmP4Lm+AXNSr4pl2j5fGBXcBJJUdLb4i1p/o4yI7XMg3B3lxhxbZc0fLQ4oWfPniM7El8T6AzxSgBl+OzPU08A+628j7D88IxaGXk5nzrCOmyYhTElfJNwe7erT2SJu9ydA0bC8OypRxJvfBAq4repxhsYFOG32ZhiTQ60BrjB2cTkgTTsLtBYipvp/sTfMZtUAwZ4wrYmSnBqgAFhM9IvpasrYp/4b2wej4AOKwMD34iipg84+29JwwapRBdWizzUm/TdKMvHUMAnwfyWkGs48mMtVjQstWA6A/gWkQILC5DnWJwF0DG1xOUSWO3lc3ETCDt9kNzO6y43ybYZaTma65w3xlLmuMaJAj1tIRAgHcMIHrlC0nmy9FLKVUf/drjsF5BlKbCIG6mWFuQcG4rNCsLu+3l1DjP5QeJZul9DEREHRtbkPsLCAN/Vxe/M6jieKGEJzoE2FEqeeQZCV5n7ihYVOmcJwvO2e4rBpVuu6/giqB2qd+mNqnwyPoTRn60uZPNpyxzLA+L5VRbzNHIsukQHjAB1wZO7KFomHV0xT8WOHDsTO7QKLE8T5UaEeJZVYFLduj1Eg+b05YvqRV4dW6L6/5oVnHpDEYYsJaS+HeRXrcBS3/Lk=";

    #[test]
    fn add_randomness_success_with_none_commitment() {
        let mut deps = default_setup();

        //register bot
        register_bot(&mut deps);

        //add randomness
        let request_add_randomness = ExecuteMsg::AddRandomness {
            random_value: String::from(RANDOM_VALUE_TEST), 
            signature: String::from(SIGNATURE_TEST),
        };
        
        let res = execute(deps.as_mut(), mock_env(), mock_info(BOT, &[]), request_add_randomness).unwrap();

        let messages: Vec<SubMsg> = Vec::new();
        assert_eq!(res, Response::new().add_attribute("action","add_randomness")
                                .add_attribute("random_value", String::from(RANDOM_VALUE_TEST))
                                .add_attribute("signature", String::from(SIGNATURE_TEST))
                                .add_attribute("bot", String::from(BOT))
                                .add_submessages(messages));
    }

    #[test]
    fn add_randomness_success_with_request_hex_randomness_commitment() {
        let mut deps = default_setup();

        //user request randomness
        request_hex_randomness(&mut deps);

                                
        //register bot
        register_bot(&mut deps);

        //add randomness
        let request_add_randomness = ExecuteMsg::AddRandomness {
            random_value: String::from(RANDOM_VALUE_TEST), 
            signature: String::from(SIGNATURE_TEST),
        };

        let env: Env = Env {
            block: BlockInfo {
                height: 0,
                time: Timestamp::from_seconds(1675739157),
                chain_id: "euphoria-2".to_string()
            },
            contract: ContractInfo {
                address: Addr::unchecked(CONTRACT_ADDR)
            },
            transaction: None,
        };
        let res = execute(deps.as_mut(), env, mock_info(BOT, &[]), request_add_randomness).unwrap();

        let mut messages: Vec<SubMsg> = Vec::new();

        messages.push(SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: USER.to_string(),
                msg: to_binary(&CallbackExecuteMsg::ReceiveHexRandomness{ 
                    request_id: String::from("test id 1"), 
                    randomness: vec![String::from("308fb245ab064a15992c03d944baa9e0b9cb253f11f39a697beb021bcb863d9f")],
                }).unwrap(),
                funds: vec![],
            }
            .into(),
            id: COMMITMENT_CALLBACK_REPLY_ID,
            gas_limit: Some(CALLBACK_LIMIT_GAS),
            reply_on: ReplyOn::Always,
        });

        messages.push(SubMsg::new(BankMsg::Send {
            to_address: String::from(BOT),
            amount: coins(FEE, String::from(DENOM)),
        }));

        assert_eq!(res, Response::new().add_attribute("action","add_randomness")
                                .add_attribute("random_value", String::from(RANDOM_VALUE_TEST))
                                .add_attribute("signature", String::from(SIGNATURE_TEST))
                                .add_attribute("bot", String::from(BOT))
                                .add_submessages(messages));
    }

    #[test]
    fn add_randomness_success_with_request_int_randomness_commitment() {
        let mut deps = default_setup();

        //user request randomness
        request_int_randomness(&mut deps);

        //register bot
        register_bot(&mut deps);

        //add randomness
        let request_add_randomness = ExecuteMsg::AddRandomness {
            random_value: String::from(RANDOM_VALUE_TEST), 
            signature: String::from(SIGNATURE_TEST),
        };

        let env: Env = Env {
            block: BlockInfo {
                height: 0,
                time: Timestamp::from_seconds(1675739157),
                chain_id: "euphoria-2".to_string()
            },
            contract: ContractInfo {
                address: Addr::unchecked(CONTRACT_ADDR)
            },
            transaction: None,
        };
        let res = execute(deps.as_mut(), env, mock_info(BOT, &[]), request_add_randomness).unwrap();

        let mut messages: Vec<SubMsg> = Vec::new();

        messages.push(SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: USER.to_string(),
                msg: to_binary(&CallbackExecuteMsg::ReceiveIntRandomness{ 
                    request_id: String::from("test id 1"), 
                    randomness: vec![208i32],
                }).unwrap(),
                funds: vec![],
            }
            .into(),
            id: COMMITMENT_CALLBACK_REPLY_ID,
            gas_limit: Some(CALLBACK_LIMIT_GAS),
            reply_on: ReplyOn::Always,
        });

        messages.push(SubMsg::new(BankMsg::Send {
            to_address: String::from(BOT),
            amount: coins(FEE, String::from(DENOM)),
        }));

        assert_eq!(res, Response::new().add_attribute("action","add_randomness")
                                .add_attribute("random_value", String::from(RANDOM_VALUE_TEST))
                                .add_attribute("signature", String::from(SIGNATURE_TEST))
                                .add_attribute("bot", String::from(BOT))
                                .add_submessages(messages));
    }

    #[test]
    fn add_randomness_fail_with_bot_not_register() {
        let mut deps = default_setup();

        //add randomness
        let request_add_randomness = ExecuteMsg::AddRandomness {
            random_value: String::from(RANDOM_VALUE_TEST), 
            signature: String::from(SIGNATURE_TEST),
        };
        
        let res = execute(deps.as_mut(), mock_env(), mock_info(BOT, &[]), request_add_randomness).unwrap_err();
        match res {
            ContractError::UnregisteredAddress{} => {},
            _ => panic!(),
        };
    }

    #[test]
    fn add_randomness_fail_with_verification_fail() {
        let mut deps = default_setup();

        //register bot
        register_bot(&mut deps);

        //add randomness
        let request_add_randomness = ExecuteMsg::AddRandomness {
            random_value: String::from(RANDOM_VALUE_TEST), 
            signature: String::from("eITMbucgIRih+606JH/zfYDIBqOYbB4VEyjCkLJIteIqMMRMZrFRBPmP4Lm+AXNSr4pl2j5fGBXcBJJUdLb4i1p/o4yI7XMg3B3lxhxbZc0fLQ4oWfPniM7El8T6AzxSgBl+OzPU08A+628j7D88IxaGXk5nzrCOmyYhTElfJNwe7erT2SJu9ydA0bC8OypRxJvfBAq4repxhsYFOG32ZhiTQ60BrjB2cTkgTTsLtBYipvp/sTfMZtUAwZ4wrYmSnBqgAFhM9IvpasrYp/4b2wej4AOKwMD34iipg84+29JwwapRBdWizzUm/TdKMvHUMAnwfyWkGs48mMtVjQstWA6A/gWkQILC5DnWJwF0DG1xOUSWO3lc3ETCDt9kNzO6y43ybYZaTma65w3xlLmuMaJAj1tIRAgHcMIHrlC0nmy9FLKVUf/drjsF5BlKbCIG6mWFuQcG4rNCsLu+3l1DjP5QeJZul9DEREHRtbkPsLCAN/Vxe/M6jieKGEJzoE2FEqeeQZCV5n7ihYVOmcJwvO2e4rBpVuu6/giqB2qd+mNqnwyPoTRn60uZPNpyxzLA+L5VRbzNHIsukQHjAB1wZO7KFomHV0xT8WOHDsTO7QKLE8T5UaEeJZVYFLduj1Eg+b05YvqRV4dW6L6/5oVnHpDEYYsJaS+HeRXrcBS3/Lk="),
        };
        
        let res = execute(deps.as_mut(), mock_env(), mock_info(BOT, &[]), request_add_randomness).unwrap_err();

        match res {
            ContractError::RSAVerificationFail{} => {},
            _ => panic!(),
        };
    }

    #[test]
    fn add_randomness_with_invalid_api_key() {
        let mut deps = default_setup();

        //register bot
        let hashed_api_key = String::from("test hashed api key");
        let moniker = String::from("test bot");

        let request_register_bot = ExecuteMsg::RegisterBot {
            hashed_api_key: hashed_api_key.clone(),
            moniker: moniker.clone(),
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info(BOT, &[]), request_register_bot).unwrap();
        assert_eq!(res, Response::new().add_attribute("action","register_bot")
                                .add_attribute("hashed_api_key", hashed_api_key)
                                .add_attribute("moniker", moniker)
                                .add_attribute("bot_address", BOT));

        //add randomness
        let request_add_randomness = ExecuteMsg::AddRandomness {
            random_value: String::from(RANDOM_VALUE_TEST), 
            signature: String::from(SIGNATURE_TEST),
        };
        
        let res = execute(deps.as_mut(), mock_env(), mock_info(BOT, &[]), request_add_randomness).unwrap_err();

        match res {
            ContractError::InvalidApiKey{} => {},
            _ => panic!(),
        };
    }

    /***** Nois Receive *****/
    #[test]
    fn nois_receive_success_with_none_commitment() {
        let mut deps = default_setup();

        let request_nois_receive = ExecuteMsg::NoisReceive {
            callback: NoisCallback{
                job_id: String::from("test callback 1"),
                randomness: HexBinary::from(&[0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00]),
            }
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(NOIS_PROXY_ADDR, &[]), request_nois_receive).unwrap();

        assert_eq!(res, Response::new().add_attribute("action","nois_receive")
                                    .add_attribute("message","commitment has been made")
                                    .add_attribute("nois_proxy_address", String::from(NOIS_PROXY_ADDR)));
    }   

    
    #[test]
    fn nois_receive_success_with_commitment() {
        let mut deps = default_setup(); 

        // request randomness 
        request_hex_randomness(&mut deps);

        let nonce: u64 = 0u64;
        let commit_id = make_commit_id(USER.to_string(), nonce);

        // nois callback
        let randomness =  [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00];
        let request_nois_receive = ExecuteMsg::NoisReceive {
            callback: NoisCallback{
                job_id: commit_id.clone(),
                randomness: HexBinary::from(&randomness),
            }
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(NOIS_PROXY_ADDR, &[]), request_nois_receive).unwrap();

        let mut sub_messages: Vec<SubMsg> = Vec::new(); 
        sub_messages.push(SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: USER.to_string(),
                msg: to_binary(&CallbackExecuteMsg::ReceiveHexRandomness{ 
                    request_id: String::from("test id 1"), 
                    randomness: vec![String::from("daebd62c597c8b55f72695073d3172aa641c6ef4aa8876546b5186933ca5fa50")],
                }).unwrap(),
                funds: vec![],
            }
            .into(),
            id: COMMITMENT_CALLBACK_REPLY_ID,
            gas_limit: Some(CALLBACK_LIMIT_GAS),
            reply_on: ReplyOn::Always,
        });

        // send bounty to contract owner
        sub_messages.push(SubMsg::new(BankMsg::Send {
            to_address: String::from(CREATOR),
            amount: coins(FEE, String::from(DENOM)),
        }));

        assert_eq!(res, Response::new().add_submessages(sub_messages)
                            .add_attribute("job_id", commit_id.clone())
                            .add_attribute("randomness", hex::encode(randomness))
                            .add_attribute("action","nois_receive")
                            .add_attribute("nois_proxy_address", String::from(NOIS_PROXY_ADDR)));
    } 

    #[test]
    fn nois_receive_fail_with_unauthorized_receive() {
        let mut deps = default_setup();

        let request_nois_receive = ExecuteMsg::NoisReceive {
            callback: NoisCallback{
                job_id: String::from("test callback 1"),
                randomness: HexBinary::from(&[0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00]),
            }
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(USER, &[]), request_nois_receive).unwrap_err();

        match res {
            ContractError::UnauthorizedReceive{} => {},
            _ => panic!(),
        }; 
    }

    #[test]
    fn nois_receive_fail_with_invalid_randomness() {
        let mut deps = default_setup();

        let request_nois_receive = ExecuteMsg::NoisReceive {
            callback: NoisCallback{
                job_id: String::from("test callback 1"),
                randomness: HexBinary::from(&[0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00]),
            }
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(NOIS_PROXY_ADDR, &[]), request_nois_receive).unwrap_err();

        match res {
            ContractError::InvalidRandomness{} => {},
            _ => panic!(),
        }; 
    }

    // QUERY

    fn add_commitments(deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>, commit_id: String, commit_time: u64, expired_time: u64) {
        let commitment: Commitment = Commitment {
            id: commit_id.clone(),
            request_id: String::from("request id"),
            owner: Addr::unchecked(String::from(USER)),
            commit_time: Timestamp::from_seconds(commit_time),
            expired_time: Timestamp::from_seconds(expired_time),
            data_request: DataRequest{
                min: 0,
                max: 255,
                num: 32,
                data_type: String::from("test data type"),
            },
        };
        COMMITMENTS.push_front(&mut deps.storage, &commitment).unwrap();
        PENDING_COMMITMENTS.save(&mut deps.storage, commit_id, &commitment).unwrap();
    }


    #[test]
    fn query_get_number_of_commitments_success() {
        let mut deps = default_setup();

        add_commitments(&mut deps, String::from("1"), 0u64, 5u64);
        add_commitments(&mut deps, String::from("2"), 0u64, 5u64);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetNumberOfCommitment{}).unwrap();

        assert_eq!(res, to_binary(&NumberOfCommitmentQuery{num: 2}).unwrap());
    }

    #[test]
    fn query_get_pending_commitments_success() {
        let mut deps = default_setup();

        add_commitments(&mut deps, String::from("1"), 0u64, 5u64);
        add_commitments(&mut deps, String::from("2"), 0u64, 5u64);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetPendingCommitments{ limit: 1}).unwrap();

        let mut commitments: Vec<Commitment> = Vec::new();
        commitments.push(Commitment {
            id: String::from("1"),
            request_id: String::from("request id"),
            owner: Addr::unchecked(String::from(USER)),
            commit_time: Timestamp::from_seconds(0u64),
            expired_time: Timestamp::from_seconds(5u64),
            data_request: DataRequest{
                min: 0,
                max: 255,
                num: 32,
                data_type: String::from("test data type"),
            },
        });
        assert_eq!(res, to_binary(&CommitmentsQuery{commitments}).unwrap());
    }

    #[test]
    fn query_get_commitments_success() {
        let mut deps = default_setup();

        add_commitments(&mut deps, String::from("1"), 0u64, 5u64);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCommitments{ limit: 1}).unwrap();

        let mut commitments: Vec<Commitment> = Vec::new();
        commitments.push(Commitment {
            id: String::from("1"),
            request_id: String::from("request id"),
            owner: Addr::unchecked(String::from(USER)),
            commit_time: Timestamp::from_seconds(0u64),
            expired_time: Timestamp::from_seconds(5u64),
            data_request: DataRequest{
                min: 0,
                max: 255,
                num: 32,
                data_type: String::from("test data type"),
            },
        });
        assert_eq!(res, to_binary(&PendingCommitmentsQuery{commitments}).unwrap());
    }
    
    #[test]
    fn get_configs_success() {
        let deps = default_setup();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetConfigs{}).unwrap();

        assert_eq!(res, to_binary(&ConfigsQuery{
            nois_proxy: String::from(NOIS_PROXY_ADDR),
            nois_fee: Uint128::from(NOIS_FEE),
            bounty_denom: String::from(DENOM),
            fee: Uint128::from(FEE),
            callback_limit_gas: CALLBACK_LIMIT_GAS,
            time_expired: TIME_EXPIRED,
            time_per_block: TIME_PER_BLOCK,
        }).unwrap());
    }

    #[test]
    fn get_bot_info_success() { 
        let mut deps = default_setup();

        let hashed_api_key = String::from("elv0PecZXTaHIbs+3PvmJIz2BO9mtakvSpznFkhgfe/EtmPPCVAqpBDIT6ZeQ3TEsCvdxymXnDuSPKoqjlxZ/Q==");
        let moniker = String::from("test bot");

        let env: Env = Env {
            block: BlockInfo {
                height: 0,
                time: Timestamp::from_seconds(100),
                chain_id: "euphoria-2".to_string()
            },
            contract: ContractInfo {
                address: Addr::unchecked(CONTRACT_ADDR)
            },
            transaction: None,
        };
        let request_register_bot = ExecuteMsg::RegisterBot {
            hashed_api_key: hashed_api_key.clone(),
            moniker: moniker.clone(),
        };
        let res = execute(deps.as_mut(), env, mock_info(BOT, &[]), request_register_bot).unwrap();
        assert_eq!(res, Response::new().add_attribute("action","register_bot")
                                .add_attribute("hashed_api_key", hashed_api_key.clone())
                                .add_attribute("moniker", moniker.clone())
                                .add_attribute("bot_address", BOT));
    

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetBotInfo{ address: String::from(BOT) }).unwrap();
        assert_eq!(res, to_binary(&Some(BotInfoQuery{
            address: String::from(BOT),
            hashed_api_key,
            moniker,
            last_update: Timestamp::from_seconds(100),
        })).unwrap());
    }

    #[test]
    fn get_bot_info_success_with_none() { 
        let mut deps = default_setup();

        let hashed_api_key = String::from("elv0PecZXTaHIbs+3PvmJIz2BO9mtakvSpznFkhgfe/EtmPPCVAqpBDIT6ZeQ3TEsCvdxymXnDuSPKoqjlxZ/Q==");
        let moniker = String::from("test bot");

        let env: Env = Env {
            block: BlockInfo {
                height: 0,
                time: Timestamp::from_seconds(100),
                chain_id: "euphoria-2".to_string()
            },
            contract: ContractInfo {
                address: Addr::unchecked(CONTRACT_ADDR)
            },
            transaction: None,
        };
        let request_register_bot = ExecuteMsg::RegisterBot {
            hashed_api_key: hashed_api_key.clone(),
            moniker: moniker.clone(),
        };
        let res = execute(deps.as_mut(), env, mock_info(BOT, &[]), request_register_bot).unwrap();
        assert_eq!(res, Response::new().add_attribute("action","register_bot")
                                .add_attribute("hashed_api_key", hashed_api_key.clone())
                                .add_attribute("moniker", moniker.clone())
                                .add_attribute("bot_address", BOT));
    
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetBotInfo{ address: String::from(USER) }).unwrap();
        assert_eq!(res, to_binary(&None::<BotInfoQuery>).unwrap());
    }

    // REPLY
    #[test]
    fn reply_success(){
        let mut deps = default_setup();
        let res = reply(deps.as_mut(), mock_env(), cosmwasm_std::Reply { id: 1, result: SubMsgResult::Err(String::from(""))}).unwrap();
        assert_eq!(res, Response::new());
    }
}