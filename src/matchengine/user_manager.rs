use crate::matchengine::rpc::*;
use crate::models::AccountDesc;
use crate::types::ConnectionType;
use babyjubjub_rs::{Point, Signature};
use num_bigint::BigInt;
use poseidon_rs::Fr;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;

pub fn order_hash(_req: &OrderPutRequest) -> BigInt {
    BigInt::default()
    // BigInt::parse_bytes(to_hex(elem).as_bytes(), 16).unwrap()
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

    pub fn verify_signature(&self, user_id: u32, msg: BigInt, signature: &str) -> bool {
        match self.users.get(&user_id) {
            None => false,
            Some(user) => babyjubjub_rs::verify(str_to_pubkey(&user.l2_pubkey), str_to_signature(signature), msg),
        }
    }
}

impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: error handling
fn str_to_pubkey(pubkey: &str) -> Point {
    let pubkey_packed = hex::decode(pubkey).unwrap();
    babyjubjub_rs::decompress_point(pubkey_packed.try_into().unwrap()).unwrap()
}

// TODO: error handling
fn str_to_signature(signature: &str) -> Signature {
    let sig_packed_vec = hex::decode(signature).unwrap();
    babyjubjub_rs::decompress_signature(&sig_packed_vec.try_into().unwrap()).unwrap()
}
