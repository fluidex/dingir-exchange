use actix_web::{web, HttpRequest, Responder};

use actix_web::web::Json;

use crate::models::{
    self,
    tablenames::{MARKETTRADE, USERTRADE},
};
use core::cmp::min;

use super::{errors::RpcError, state::AppState, types};
use models::{DecimalDbType, TimestampDbType};
use rust_decimal::prelude::*;

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
    // models::UserTrade is designed for a user to fetch his trades.

    let sql_query = format!("select * from {} where market = $1 order by time desc limit {}", MARKETTRADE, limit);

    let trades: Vec<models::MarketTrade> = sqlx::query_as(&sql_query).bind(market).fetch_all(&data.db).await?;

    Ok(Json(trades))
}

#[derive(sqlx::FromRow, Debug, Clone)]
struct QueriedUserTrade {
    pub time: TimestampDbType,
    pub user_id: i32,
    pub trade_id: i64,
    pub order_id: i64,
    pub price: DecimalDbType,
    pub amount: DecimalDbType,
    pub quote_amount: DecimalDbType,
    pub fee: DecimalDbType,
}

#[cfg(sqlxverf)]
fn sqlverf_ticker() {
    sqlx::query_as!(
        QueriedUserTrade,
        "select time, user_id, trade_id, order_id,
        price, amount, quote_amount, fee
        from user_trade where market = $1 and order_id = $2 
        order by trade_id, time asc",
        "USDT_ETH",
        10000,
    );
}

pub async fn order_trades(
    app_state: web::Data<AppState>,
    web::Path((market_name, order_id)): web::Path<(String, i64)>,
) -> Result<Json<types::OrderTradeResult>, RpcError> {
    log::debug!("order_trades market {} order_id {}", market_name, order_id);

    let sql_query = format!(
        "
    select time, user_id, trade_id, order_id,
    price, amount, quote_amount, fee
    from {} where market = $1 and order_id = $2 
    order by trade_id, time asc",
        USERTRADE
    );

    let trades: Vec<QueriedUserTrade> = sqlx::query_as(&sql_query)
        .bind(market_name)
        .bind(order_id)
        .fetch_all(&app_state.db)
        .await?;

    Ok(Json(types::OrderTradeResult {
        trades: trades
            .into_iter()
            .map(|v| types::MarketTrade {
                time: v.time.timestamp() as i32,
                amount: v.amount.to_f32().unwrap_or(0.0),
                quote_amount: v.quote_amount.to_f32().unwrap_or(0.0),
                price: v.price.to_f32().unwrap_or(0.0),
                fee: v.fee.to_f32().unwrap_or(0.0),
            })
            .collect(),
    }))
}
