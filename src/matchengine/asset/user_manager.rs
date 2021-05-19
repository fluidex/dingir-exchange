use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
pub struct UserInfo {
    pub l1_address: String,
    pub l2_pubkey: String,
}

// TODO: combine with balance_manager/?
#[derive(Clone)]
pub struct UserManager {
    pub users: HashMap<u32, UserInfo>,
}

impl UserManager {
    // TODO:
    pub fn new() -> Self {
        Self { users: HashMap::new() }
    }
}

// // TODO: select ... order by id desc limit 1?
// let query = format!("select count(*) from {}", tablenames::ACCOUNT);
// let last_user_id: (i32,) = sqlx::query_as(&query).fetch_one(self.dbg_pool).await.map_err(
//     |_| Err(Status::unavailable("")), // TODO:
// )?;
