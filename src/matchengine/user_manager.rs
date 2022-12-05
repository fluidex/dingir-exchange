use crate::dto::UserIdentifier;
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
    pub users: HashMap<UserIdentifier, UserInfo>,
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
                UserIdentifier {
                    user_id: user.id,
                    broker_id: user.broker_id,
                    account_id: user.account_id,
                },
                UserInfo {
                    l1_address: user.l1_address,
                    l2_pubkey: user.l2_pubkey,
                },
            );
        }
        Ok(())
    }

    pub fn verify_signature(&self, user_info: UserIdentifier, msg: BigInt, signature: &str) -> bool {
        match self.users.get(&user_info) {
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
