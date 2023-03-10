use std::collections::HashMap;
use cw_storage_plus::Map;

pub const TEST: Map<u32, HashMap<u32,String>> = Map::new("test"); 