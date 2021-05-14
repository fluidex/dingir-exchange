use super::errors::RpcError;
use super::state::AppState;
use super::types::UserInfo;
use crate::models::{tablenames::USER, UserDesc};
use actix_web::{
    web::{self, Json},
    HttpRequest, Responder,
};

pub async fn get_user(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    let mut is_debug: bool = false;
    if req.match_info().get("debug").unwrap_or("false").to_string() == "true".to_string() {
        is_debug = true;
    }

    if is_debug {
        let user_id = req.match_info().get("id_or_addr").unwrap();
        if user_id.starts_with("0x") {
            let mut user_map = data.user_addr_map.lock().unwrap();
            if !user_map.contains_key(user_id) {
                // TODO: real query from DB
                let count = user_map.len();
                user_map.insert(user_id.to_string(), UserInfo { user_id: count as i64 });
            }
            let user_info = *user_map.get(user_id).unwrap();
            Ok(web::Json(user_info))
        } else {
            // TODO: get_by_user_id still fails
            Err(RpcError::bad_request("invalid user id or address"))
        }
    } else {
        // TODO:
        Err(RpcError::bad_request("must debug mode"));

        // TODO: this API result should be cached, either in-memory or using redis
        let sql_query = format!("select * from {} where market = $1 order by time desc", USER);
        // TODO: fecth_one? fecth_optional?
        let trades: Vec<UserDesc> = sqlx::query_as(&sql_query).fetch_all(&data.db).await?;
        log::debug!("query {} recent_trades records", trades.len());

        Ok(Json(trades))
    }
}
