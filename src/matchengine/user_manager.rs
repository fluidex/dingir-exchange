use crate::models::AccountDesc;
use crate::types::ConnectionType;
use fluidex_common::babyjubjub_rs;
use fluidex_common::types::{BigInt, PubkeyExt, SignatureExt};
use std::collections::HashMap;
use std::fmt::Display;

#[derive(Clone)]
pub struct UserManager {
    max_user_id: i32,
    users: HashMap<String, AccountDesc>,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            max_user_id: 0,
            users: HashMap::new(),
        }
    }
    pub fn reset(&mut self) {
        self.users.clear();
    }

    pub fn contains(&self, user_id: u32) -> bool {
        self.users.contains_key(&format_user_id_key(user_id))
    }

    pub fn add_user(&mut self, l1_address: String, l2_pubkey: String) -> AccountDesc {
        let user_id = self.max_user_id + 1;
        let l1_address_key = format_l1_address_key(&l1_address);
        let l2_pubkey_key = format_l2_pubkey_key(&l2_pubkey);

        let user_info = AccountDesc {
            id: user_id,
            l1_address,
            l2_pubkey,
        };

        self.users.insert(format_user_id_key(&user_id), user_info.clone());
        self.users.insert(l1_address_key, user_info.clone());
        self.users.insert(l2_pubkey_key, user_info.clone());
        self.max_user_id = user_id;

        user_info
    }

    pub fn get_user(&self, user_id: Option<u32>, l1_address: Option<String>, l2_pubkey: Option<String>) -> Option<&AccountDesc> {
        user_id
            .and_then(|val| self.users.get(&format_user_id_key(val)))
            .or_else(|| l1_address.and_then(|val| self.users.get(&format_l1_address_key(val))))
            .or_else(|| l2_pubkey.and_then(|val| self.users.get(&format_l2_pubkey_key(val))))
    }

    pub async fn load_users_from_db(&mut self, conn: &mut ConnectionType) -> anyhow::Result<()> {
        let users: Vec<AccountDesc> = sqlx::query_as::<_, AccountDesc>("SELECT * FROM account").fetch_all(conn).await?;

        for user in users {
            let user_id = user.id;
            self.users.insert(format_user_id_key(user_id), user.clone());
            self.users.insert(format_l1_address_key(&user.l1_address), user.clone());
            self.users.insert(format_l2_pubkey_key(&user.l2_pubkey), user);
            if user_id > self.max_user_id {
                self.max_user_id = user_id;
            }
        }

        Ok(())
    }

    pub fn verify_signature(&self, user_id: u32, msg: BigInt, signature: &str) -> bool {
        match self.users.get(&format_user_id_key(&user_id)) {
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

fn format_user_id_key<T: Display>(val: T) -> String {
    format!("id:{}", val)
}

fn format_l1_address_key<T: Display>(val: T) -> String {
    format!("l1_addr:{}", val)
}

fn format_l2_pubkey_key<T: Display>(val: T) -> String {
    format!("l2_pubkey:{}", val)
}
