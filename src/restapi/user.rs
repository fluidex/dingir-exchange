use crate::models::{tablenames::ACCOUNT, AccountDesc};
use crate::restapi::errors::RpcError;
use crate::restapi::state::AppState;
use paperclip::actix::api_v2_operation;
use paperclip::actix::web::{self, HttpRequest, Json};
use std::fmt::Display;

#[api_v2_operation]
pub async fn get_user(req: HttpRequest, data: web::Data<AppState>) -> Result<Json<AccountDesc>, actix_web::Error> {
    let user_id = req.match_info().get("l1addr_or_l2pubkey").unwrap().to_lowercase();
    let mut user_map = data.user_addr_map.lock().unwrap();

    if let Some(user_info) = user_map
        .get(&format_user_id_key(&user_id))
        .or_else(|| user_map.get(&format_l1_address_key(&user_id)))
        .or_else(|| user_map.get(&format_l2_pubkey_key(&user_id)))
    {
        return Ok(Json(user_info.clone()));
    }

    let sql_query = format!("select * from {} where id = $1 OR l1_address = $2 OR l2_pubkey = $2", ACCOUNT);
    let user: AccountDesc = sqlx::query_as(&sql_query)
        .bind(user_id.parse::<i32>().unwrap_or(-1))
        .bind(user_id)
        .fetch_one(&data.db)
        .await
        .map_err(|e| {
            log::error!("{:?}", e);
            RpcError::bad_request("invalid user ID, l1 address or l2 public key")
        })?;

    let user_info = AccountDesc {
        id: user.id.clone(),
        broker_id: user.broker_id.clone(),
        account_id: user.account_id.clone(),
        l1_address: user.l1_address.clone(),
        l2_pubkey: user.l2_pubkey.clone(),
    };

    user_map.insert(format_user_id_key(&user_info.id), user_info.clone());
    user_map.insert(format_account_id_key(&user_info.account_id), user_info.clone());
    user_map.insert(format_broker_id_key(&user_info.broker_id), user_info.clone());
    user_map.insert(format_l1_address_key(&user_info.l1_address), user_info.clone());
    user_map.insert(format_l2_pubkey_key(&user_info.l2_pubkey), user_info.clone());

    Ok(Json(user))
}

fn format_broker_id_key<T: Display>(val: T) -> String {
    format!("broker_id:{}", val)
}
fn format_account_id_key<T: Display>(val: T) -> String {
    format!("account_id:{}", val)
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
