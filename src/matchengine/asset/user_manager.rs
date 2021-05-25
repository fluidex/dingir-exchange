use crate::config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
pub struct UserInfo {
    pub l1_address: String,
    pub l2_pubkey: String,
}

// TODO: combine with balance_manager?
#[derive(Clone)]
pub struct UserManager {
    pub users: HashMap<u32, UserInfo>,
}

impl UserManager {
    pub fn new(user_config: &[config::User]) -> Self {
        let mut users: HashMap<u32, UserInfo> = HashMap::new();
        for item in user_config.iter() {
            users.insert(
                item.id,
                UserInfo {
                    l1_address: item.l1_address,
                    l2_pubkey: item.l2_pubkey,
                },
            );
        }
        Self { users }
    }
}

// // TODO: select ... order by id desc limit 1?
// let query = format!("select count(*) from {}", tablenames::ACCOUNT);
// let last_user_id: (i32,) = sqlx::query_as(&query).fetch_one(self.dbg_pool).await.map_err(
//     |_| Err(Status::unavailable("")), // TODO:
// )?;
