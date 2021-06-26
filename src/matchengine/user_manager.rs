use crate::matchengine::rpc::*;
use crate::models::AccountDesc;
use crate::primitives::*;
use crate::types::ConnectionType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;

pub fn order_hash(_req: &OrderPutRequest) -> BigInt {
    // consistent with https://github.com/Fluidex/circuits/blob/d6e06e964b9d492f1fa5513bcc2295e7081c540d/helper.ts/state-utils.ts#L38
    // TxType::PlaceOrder
    let magic_head = u32_to_fr(4);
    let data = hash(&[
        magic_head,
        // TODO: sign nonce or order_id
        //u32_to_fr(self.order_id),

        // self.token_sell,
        // self.token_buy,
        // self.total_sell,
        // self.total_buy,
    ]);
    //data = hash([data, accountID, nonce]);
    // nonce and orderID seems redundant?

    // account_id is not needed if the hash is signed later?
    //data = hash(&[data, u32_to_fr(self.account_id)]);
    fr_to_bigint(&data)
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
