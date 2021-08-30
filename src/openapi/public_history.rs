use crate::models::tablenames::{MARKETTRADE, USERTRADE};
use crate::models::{self, DecimalDbType, TimestampDbType};
use crate::restapi::errors::RpcError;
use crate::restapi::state::AppState;
use crate::restapi::types;
use chrono::{DateTime, SecondsFormat, Utc};
use core::cmp::min;
use paperclip::actix::api_v2_operation;
use paperclip::actix::web::{self, HttpRequest, Json};

fn check_market_exists(_market: &str) -> bool {
    // TODO
    true
}

#[api_v2_operation]
pub async fn recent_trades(req: HttpRequest, data: web::Data<AppState>) -> Result<Json<Vec<models::MarketTrade>>, actix_web::Error> {
    let market = req.match_info().get("market").unwrap();
    let qstring = qstring::QString::from(req.query_string());
    let limit = min(100, qstring.get("limit").unwrap_or_default().parse::<usize>().unwrap_or(20));
    log::debug!("recent_trades market {} limit {}", market, limit);
    if !check_market_exists(market) {
        return Err(RpcError::bad_request("invalid market").into());
    }

    // TODO: this API result should be cached, either in-memory or using redis

    // Here we use the kline trade table, which is more market-centric
    // and more suitable for fetching latest trades on a market.
    // models::UserTrade is designed for a user to fetch his trades.

    let sql_query = format!("select * from {} where market = $1 order by time desc limit {}", MARKETTRADE, limit);

    let trades: Vec<models::MarketTrade> = match sqlx::query_as(&sql_query).bind(market).fetch_all(&data.db).await {
        Ok(trades) => trades,
        Err(error) => {
            let error: RpcError = error.into();
            return Err(error.into());
        }
    };

    log::debug!("query {} recent_trades records", trades.len());
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
fn sqlverf_ticker() -> impl std::any::Any {
    sqlx::query_as!(
        QueriedUserTrade,
        "select time, user_id, trade_id, order_id,
        price, amount, quote_amount, fee
        from user_trade where market = $1 and order_id = $2 
        order by trade_id, time asc",
        "USDT_ETH",
        10000,
    )
}

#[api_v2_operation]
pub async fn order_trades(
    app_state: web::Data<AppState>,
    path: web::Path<(String, i64)>,
) -> Result<Json<types::OrderTradeResult>, actix_web::Error> {
    let (market_name, order_id): (String, i64) = path.into_inner();
    log::debug!("order_trades market {} order_id {}", market_name, order_id);

    let sql_query = format!(
        "
    select time, user_id, trade_id, order_id,
    price, amount, quote_amount, fee
    from {} where market = $1 and order_id = $2 
    order by trade_id, time asc",
        USERTRADE
    );

    let trades: Vec<QueriedUserTrade> = match sqlx::query_as(&sql_query)
        .bind(market_name)
        .bind(order_id)
        .fetch_all(&app_state.db)
        .await
    {
        Ok(trades) => trades,
        Err(error) => {
            let error: RpcError = error.into();
            return Err(error.into());
        }
    };

    Ok(Json(types::OrderTradeResult {
        trades: trades
            .into_iter()
            .map(|v| types::MarketTrade {
                trade_id: v.trade_id,
                time: DateTime::<Utc>::from_utc(v.time, Utc).to_rfc3339_opts(SecondsFormat::Secs, true),
                amount: v.amount.to_string(),
                quote_amount: v.quote_amount.to_string(),
                price: v.price.to_string(),
                fee: v.fee.to_string(),
            })
            .collect(),
    }))
}
