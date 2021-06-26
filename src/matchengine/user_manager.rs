pub use crate::models::AccountDesc;
use crate::matchengine::rpc::*;
use crate::types::ConnectionType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn order_hash(_req: &OrderPutRequest) -> String {
    String::default()
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
pub struct UserInfo {
    pub l1_address: String,
    pub l2_pubkey: String,
}

#[derive(Clone)]
pub struct UserManager {
    pub users: HashMap<u32, UserInfo>,
}

impl UserManager {
    pub fn new() -> Self {
        Self { users: HashMap::new() }
    }
    pub fn reset(&mut self) {
        self.users.clear();
    }

    pub async fn load_users_from_db(&mut self, conn: &mut ConnectionType) -> anyhow::Result<()> {
        let users: Vec<AccountDesc> = sqlx::query_as::<_, AccountDesc>("SELECT * FROM account").fetch_all(conn).await?;
        // lock?
        for user in users {
            self.users.insert(
                user.id as u32,
                UserInfo {
                    l1_address: user.l1_address,
                    l2_pubkey: user.l2_pubkey,
                },
            );
        }
        Ok(())
    }

    // TODO: use proper types?
    pub fn verify_signature(&self, user_id: u32, msg: &str, signature: &str) -> bool {
        match self.users.get(&user_id) {
            None => false,
            Some(user) => {                
                babyjubjub_rs::verify(user.l2_pubkey.into(), signature.clone().into(), msg.clone().into())
            },
        }
    }
}

impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for Point {
   fn from(s: String) -> Self {
        unimplemented!()
   }
}

impl From<String> for SignatureBJJ {
   fn from(s: String) -> Self {
        unimplemented!()
   }
}

impl From<String> for Fr {
   fn from(s: String) -> Self {
        unimplemented!()
   }
}
