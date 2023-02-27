use sha2::{Sha256,Digest};
use cosmwasm_std::Timestamp;
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

// generate job id from contract address and token id
pub fn make_job_id(box_id: String, user_address: String) -> String{
    let seed = box_id + &user_address;
    return hex::encode(sha256_hash(seed.as_bytes()));
}

// convert time with format "D:M:Y s:m:hZ" to Timestamp
pub fn convert_datetime_string(data: String) -> Result<Timestamp, ContractError> {
    let date_time = data.parse::<DateTime<Local>>()
        .map_err(|_| ContractError::CustomError{val: String::from("Invalid date string format!")})?;
    return Ok(Timestamp::from_nanos(date_time.timestamp_nanos() as u64));
}
