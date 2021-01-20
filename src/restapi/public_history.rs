use actix_web::{web, HttpRequest, Responder};

use actix_web::web::Json;

use core::cmp::min;

use super::{errors::RpcError, state::AppState};
fn check_market_exists(_market: &str) -> bool {
    // TODO
    true
}
pub async fn recent_trades(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    let market = req.match_info().get("market").unwrap();
    let qstring = qstring::QString::from(req.query_string());
    let limit = min(100, qstring.get("limit").unwrap_or_default().parse::<usize>().unwrap_or(20));
    log::debug!("recent_trades market {} limit {}", market, limit);
    if !check_market_exists(market) {
        return Err(RpcError::bad_request("invalid market"));
    }

    // TODO: this API result should be cached, either in-memory or using redis

    // Here we use the kline trade table, which is more market-centric
    // and more suitable for fetching latest trades on a market.
    // models::TradeHistory is designed for a user to fetch his trades.

    let trade_table = crate::models::tablenames::TRADERECORD;
    let sql_query = format!("select * from {} where market = $1 order by time desc limit {}", trade_table, limit);

    let trades: Vec<crate::models::TradeRecord> = sqlx::query_as(&sql_query).bind(market).fetch_all(&data.db).await?;

    Ok(Json(trades))
}
