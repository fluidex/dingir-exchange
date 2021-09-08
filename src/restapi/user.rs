use crate::models::{tablenames::ACCOUNT, AccountDesc};
use crate::restapi::errors::RpcError;
use crate::restapi::state::AppState;
use paperclip::actix::api_v2_operation;
use paperclip::actix::web::{self, HttpRequest, Json};

#[api_v2_operation]
pub async fn get_user(req: HttpRequest, data: web::Data<AppState>) -> Result<Json<AccountDesc>, actix_web::Error> {
    let user_id = req.match_info().get("l1addr_or_l2pubkey").unwrap().to_lowercase();
    let mut user_map = data.user_addr_map.lock().unwrap();
    if user_map.contains_key(&user_id) {
        let user_info = &*user_map.get(&user_id).unwrap();
        return Ok(Json(user_info.clone()));
    }

    let sql_query = format!("select * from {} where l1_address = $1 OR l2_pubkey = $1", ACCOUNT);
    let user: AccountDesc = sqlx::query_as(&sql_query).bind(user_id).fetch_one(&data.db).await.map_err(|e| {
        log::error!("{:?}", e);
        RpcError::bad_request("invalid user id or address")
    })?;

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
