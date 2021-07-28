use actix_web::{
    web::{self, Json},
    HttpRequest,
};
use core::cmp::min;
use serde::Serialize;

use crate::models::{
    tablenames::{INTERNALTX, ORDERHISTORY},
    DecimalDbType, InternalTx, OrderHistory, TimestampDbType,
};

use super::{errors::RpcError, state::AppState};

#[derive(Serialize)]
pub struct OrderResponse {
    total: i64,
    orders: Vec<OrderHistory>,
}

pub async fn my_orders(req: HttpRequest, data: web::Data<AppState>) -> Result<Json<OrderResponse>, RpcError> {
    let market = req.match_info().get("market").unwrap();
    let user_id = req.match_info().get("user_id").unwrap_or_default().parse::<i32>();
    let user_id = match user_id {
        Err(_) => {
            return Err(RpcError::bad_request("invalid user_id"));
        }
        _ => user_id.unwrap(),
    };
    let qstring = qstring::QString::from(req.query_string());
    let limit = min(100, qstring.get("limit").unwrap_or_default().parse::<usize>().unwrap_or(20));
    let offset = qstring.get("offset").unwrap_or_default().parse::<usize>().unwrap_or(0);

    let table = ORDERHISTORY;
    let condition = if market == "all" {
        "user_id = $1".to_string()
    } else {
        "market = $1 and user_id = $2".to_string()
    };
    let order_query = format!(
        "select * from {} where {} order by id desc limit {} offset {}",
        table, condition, limit, offset
    );
    let orders: Vec<OrderHistory> = if market == "all" {
        sqlx::query_as(&order_query).bind(user_id)
    } else {
        sqlx::query_as(&order_query).bind(market).bind(user_id)
    }
    .fetch_all(&data.db)
    .await?;
    let count_query = format!("select count(*) from {} where {}", table, condition);
    let total: i64 = if market == "all" {
        sqlx::query_scalar(&count_query).bind(user_id)
    } else {
        sqlx::query_scalar(&count_query).bind(market).bind(user_id)
    }
    .fetch_one(&data.db)
    .await?;
    Ok(Json(OrderResponse { total, orders }))
}

#[derive(Serialize)]
pub struct InternalTxResponse {
    time: TimestampDbType,
    user_from: String,
    user_to: String,
    asset: String,
    amount: DecimalDbType,
}

// TODO:
// 1. filter from/to user
// 2. use user's l2_pubkey
// 3. limit & offset
// 4. filter time interval
// 5. update transfer.ts test
pub async fn my_internal_txs(req: HttpRequest, data: web::Data<AppState>) -> Result<Json<Vec<InternalTxResponse>>, RpcError> {
    let user_id = req.match_info().get("user_id").unwrap_or_default().parse::<i32>();
    let user_id = match user_id {
        Err(_) => {
            return Err(RpcError::bad_request("invalid user_id"));
        }
        _ => user_id.unwrap(),
    };

    let txs_query = format!("select * from {} where user_from=$1 or user_to=$2", INTERNALTX);
    let txs: Vec<InternalTx> = sqlx::query_as(&txs_query).bind(user_id).bind(user_id).fetch_all(&data.db).await?;
    let resp: Vec<InternalTxResponse> = txs
        .iter()
        .map(|tx| InternalTxResponse {
            time: tx.time,
            user_from: tx.user_from.to_string(),
            user_to: tx.user_to.to_string(),
            asset: tx.asset.clone(),
            amount: tx.amount,
        })
        .collect();

    Ok(Json(resp))
}
