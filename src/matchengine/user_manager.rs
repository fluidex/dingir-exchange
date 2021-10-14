use crate::models::AccountDesc;
use crate::types::ConnectionType;
use fluidex_common::babyjubjub_rs;
use fluidex_common::types::{BigInt, PubkeyExt, SignatureExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
pub struct UserInfo {
    pub l1_address: String,
    pub l2_pubkey: String,
}

#[derive(Clone)]
pub struct UserManager {
    pub users: HashMap<u32, UserInfo>,
    pub pubkey_user_ids: HashMap<String, u32>,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            pubkey_user_ids: HashMap::new(),
        }
    }
    pub fn reset(&mut self) {
        self.users.clear();
    }

    pub async fn load_users_from_db(&mut self, conn: &mut ConnectionType) -> anyhow::Result<()> {
        let users: Vec<AccountDesc> = sqlx::query_as::<_, AccountDesc>("SELECT * FROM account").fetch_all(conn).await?;
        // lock?
        for user in users {
            let user_id = user.id as u32;
            let l2_pubkey = user.l2_pubkey;
            self.users.insert(
                user_id,
                UserInfo {
                    l1_address: user.l1_address,
                    l2_pubkey: l2_pubkey.clone(),
                },
            );
            self.pubkey_user_ids.insert(l2_pubkey, user_id);
        }
        Ok(())
    }

    pub fn verify_signature(&self, user_id: u32, msg: BigInt, signature: &str) -> bool {
        match self.users.get(&user_id) {
            None => false,
            Some(user) => {
                let pubkey = match PubkeyExt::from_str(&user.l2_pubkey) {
                    Ok(pubkey) => pubkey,
                    Err(_) => {
                        log::error!("invalid pubkey {:?}", user.l2_pubkey);
                        return false;
                    }
                };
                let signature = match SignatureExt::from_str(signature) {
                    Ok(signature) => signature,
                    Err(_) => {
                        log::error!("invalid signature {:?}", signature);
                        return false;
                    }
                };
                babyjubjub_rs::verify(pubkey, signature, msg)
            }
        }
    }
}

impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}
