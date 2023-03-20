use sha2::{Sha256,Digest};
use cosmwasm_std::Timestamp;
use chrono::{DateTime, Local};
use crate::error::ContractError;

/// calculate sha256 hash value
pub fn sha256_hash(string: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    // write input message
    hasher.update(string);
    // read hash digest and consume hasher
    let result = hasher.finalize();

    return result.to_vec();
}

/// make id string from param1 and param2
pub fn make_id(params: Vec<String>) -> String {
    let seed = params.join("");
    return hex::encode(sha256_hash(seed.as_bytes()));
}

/// convert time with format "D:M:Y s:m:hZ" to Timestamp
pub fn convert_datetime_string(data: String) -> Result<Timestamp, ContractError> {
    let date_time = data.parse::<DateTime<Local>>()
        .map_err(|_| ContractError::CustomError{val: String::from("Invalid date string format!")})?;
    return Ok(Timestamp::from_nanos(date_time.timestamp_nanos() as u64));
}

/* /// max usize value 2 ^ 64 - 1
const MAX_USIZE: u128 = 18_446_744_073_709_551_615u128;
/// convert uint256 type to usize type
pub fn uint256_2_usize(u: Uint256) -> Result<usize, ContractError> {
    if u > Uint256::from_u128(MAX_USIZE) {
        return Err(ContractError::CustomError{val: String::from("to large number")});
    }

    let bytes = u.to_le_bytes();
    return Ok(usize::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], 
                bytes[4], bytes[5], bytes[6], bytes[7]]));
} */

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn hash_256_success() {
        let data = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20];
        assert_eq!(sha256_hash(&data),
        vec![117, 174, 233, 220, 201, 251, 231, 221, 201, 57, 79, 91, 197, 211, 141, 159, 90, 211, 97, 240, 82, 15, 124, 234, 181, 150, 22, 227, 143, 89, 80, 181]);
    }

    #[test]
    fn convert_datetime_success() {
        let time: String = String::from(r#"2023-01-09 02:01:26Z"#);
        assert_eq!(convert_datetime_string(time).unwrap().seconds(),1673229686);
    }

    #[test]
    fn convert_datetime_fail_with_invalid_format() {
        let time: String = String::from(r#"2023/01/09 02:01:26Z"#);
        let date = convert_datetime_string(time).unwrap_err();
        match date {
            ContractError::CustomError{val: v} => {assert_eq!(v, String::from("Invalid date string format!"))},
            _ => panic!(),
        }
    }

    #[test]
    fn make_id_success() {
        let params: Vec<String> = vec![String::from("param1"), String::from("param2"), String::from("param3")];
        
        assert_eq!(make_id(params), String::from("ff692cfd3061d86038f245597ae55a8161f9840488feab19a9ccd0da6a86c019"));
    }

    /* #[test]
    fn uint256_2_usize_success() {
        let big_int = Uint256::from_u128(18_446_744_073_709_551_615u128);

        assert_eq!(uint256_2_usize(big_int).unwrap(), 18_446_744_073_709_551_615usize);
    }

    #[test]
    fn uint256_2_usize_fail_with_too_big_uint() {
        let big_int = Uint256::from_u128(18_446_744_073_709_551_616u128);

        let res = uint256_2_usize(big_int).unwrap_err();

        match res {
            ContractError::CustomError{val: v} => assert_eq!(v, String::from("to large number")),
            _ => {}
        }
    } */
}