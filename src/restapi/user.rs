use super::errors::RpcError;
use super::state::AppState;
use crate::models::{tablenames::ACCOUNT, AccountDesc};
use actix_web::{
    web::{self, Json},
    HttpRequest, Responder,
};

pub async fn get_user(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    let mut is_debug: bool = false;
    if *req.match_info().get("debug").unwrap_or("false") == *"true" {
        is_debug = true;
    }

    let user_id: &str = req.match_info().get("l1addr_or_l2pubkey").unwrap();

    if is_debug {
        if user_id.starts_with("0x") {
            let mut user_map = data.user_addr_map.lock().unwrap();
            if !user_map.contains_key(user_id) {
                let count = user_map.len();
                user_map.insert(
                    user_id.to_string(),
                    AccountDesc {
                        id: count as i32,
                        l1_address: user_id.to_string(),
                        l2_pubkey: Default::default(),
                    },
                );
            }
            let user_info = &*user_map.get(user_id).unwrap();
            Ok(Json(user_info.clone()))
        } else {
            Err(RpcError::bad_request("invalid user address"))
        }
    } else {
        let mut user_map = data.user_addr_map.lock().unwrap();
        if user_map.contains_key(user_id) {
            let user_info = &*user_map.get(user_id).unwrap();
            return Ok(Json(user_info.clone()));
        }

        let sql_query = format!("select * from {} where l1_address = $1 OR l2_pubkey = $1", ACCOUNT);
        //let sql_query = format!("select * from {} where id = $1 OR l1_address = $1 OR l2_pubkey = $1", ACCOUNT);
        let user: AccountDesc = sqlx::query_as(&sql_query).bind(user_id).fetch_one(&data.db).await?;
        // the default error conversion is good enough.
        // converting any error into one type will lose information, eg: bug: inconsistent type in sql (restapi/user select) https://github.com/fluidex/dingir-exchange/issues/171
        //            .map_err(|e| {println!("{:?}", e); RpcError::bad_request("invalid user id or address")})?;

        // update cache
        user_map.insert(
            user.l1_address.clone(),
            AccountDesc {
                id: user.id,
                l1_address: user.l1_address.clone(),
                l2_pubkey: user.l2_pubkey.clone(),
            },
        );

        Ok(Json(user))
    }
}
