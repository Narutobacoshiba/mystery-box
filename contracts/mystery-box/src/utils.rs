use sha2::{Sha256,Digest};
use cosmwasm_std::{Timestamp, Uint256, StdResult, StdError};
use chrono::{DateTime, Local};
use crate::error::ContractError;

// calculate sha256 hash value
pub fn sha256_hash(string: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    // write input message
    hasher.update(string);
    // read hash digest and consume hasher
    let result = hasher.finalize();

    return result.to_vec();
}

// generate job id from user address and box id
pub fn make_job_id(box_id: String, user_address: String) -> String{
    let seed = box_id + &user_address;
    return hex::encode(sha256_hash(seed.as_bytes()));
}

// generate token id from box id and randomness
pub fn make_token_id(box_id: String, randomness: String) -> String {
    let seed = box_id + &randomness;
    return hex::encode(sha256_hash(seed.as_bytes())); 
}

// convert time with format "D:M:Y s:m:hZ" to Timestamp
pub fn convert_datetime_string(data: String) -> Result<Timestamp, ContractError> {
    let date_time = data.parse::<DateTime<Local>>()
        .map_err(|_| ContractError::CustomError{val: String::from("Invalid date string format!")})?;
    return Ok(Timestamp::from_nanos(date_time.timestamp_nanos() as u64));
}

pub fn uint256_2_usize(u: Uint256) -> StdResult<usize> {
    if u > Uint256::from_u128(18_446_744_073_709_551_615u128) {
        return Err(StdError::GenericErr { msg: String::from("to large number") });
    }

    let bytes = u.to_le_bytes();
    return Ok(u64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], 
                bytes[4], bytes[5], bytes[6], bytes[7]]) as usize);
}