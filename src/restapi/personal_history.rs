use crate::models::tablenames::{ACCOUNT, INTERNALTX, ORDERHISTORY};
use crate::models::{DateTimeMilliseconds, DecimalDbType, OrderHistory, TimestampDbType};
use crate::restapi::errors::RpcError;
use crate::restapi::state::AppState;
use core::cmp::min;
use paperclip::actix::web::{self, HttpRequest, Json};
use paperclip::actix::{api_v2_operation, Apiv2Schema};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize, Apiv2Schema)]
pub struct OrderResponse {
    total: i64,
    orders: Vec<OrderHistory>,
}

#[api_v2_operation]
pub async fn my_orders(req: HttpRequest, data: web::Data<AppState>) -> Result<Json<OrderResponse>, actix_web::Error> {
    let market = req.match_info().get("market").unwrap();
    let user_id = req.match_info().get("user_id").unwrap_or_default().parse::<String>();
    let user_id = match user_id {
        Err(_) => {
            return Err(RpcError::bad_request("invalid user_id").into());
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
        sqlx::query_as(&order_query).bind(user_id.clone())
    } else {
        sqlx::query_as(&order_query).bind(market).bind(user_id.clone())
    }
    .fetch_all(&data.db)
    .await
    .map_err(|err| actix_web::Error::from(RpcError::from(err)))?;
    let count_query = format!("select count(*) from {} where {}", table, condition);
    let total: i64 = if market == "all" {
        sqlx::query_scalar(&count_query).bind(user_id)
    } else {
        sqlx::query_scalar(&count_query).bind(market).bind(user_id)
    }
    .fetch_one(&data.db)
    .await
    .map_err(|err| actix_web::Error::from(RpcError::from(err)))?;
    Ok(Json(OrderResponse { total, orders }))
}

#[derive(sqlx::FromRow, Serialize, Apiv2Schema)]
pub struct InternalTxResponse {
    #[serde(with = "DateTimeMilliseconds")]
    time: TimestampDbType,
    user_from: String,
    user_to: String,
    asset: String,
    amount: DecimalDbType,
}

#[derive(Copy, Clone, Debug, Deserialize, Apiv2Schema)]
pub enum Order {
    #[serde(rename = "lowercase")]
    Asc,
    #[serde(rename = "lowercase")]
    Desc,
}

impl Default for Order {
    fn default() -> Self {
        Self::Desc
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Apiv2Schema)]
pub enum Side {
    #[serde(rename = "lowercase")]
    From,
    #[serde(rename = "lowercase")]
    To,
    #[serde(rename = "lowercase")]
    Both,
}

impl Default for Side {
    fn default() -> Self {
        Self::Both
    }
}

#[derive(Debug, Deserialize, Apiv2Schema)]
pub struct InternalTxQuery {
    /// limit with default value of 20 and max value of 100.
    #[serde(default = "default_limit")]
    limit: usize,
    /// offset with default value of 0.
    #[serde(default = "default_zero")]
    offset: usize,
    #[serde(default, deserialize_with = "u64_timestamp_deserializer")]
    start_time: Option<TimestampDbType>,
    #[serde(default, deserialize_with = "u64_timestamp_deserializer")]
    end_time: Option<TimestampDbType>,
    #[serde(default)]
    order: Order,
    #[serde(default)]
    side: Side,
}

fn u64_timestamp_deserializer<'de, D>(deserializer: D) -> Result<Option<TimestampDbType>, D::Error>
where
    D: Deserializer<'de>,
{
    let timestamp = Option::<u64>::deserialize(deserializer)?;
    Ok(timestamp.map(|ts| TimestampDbType::from_timestamp(ts as i64, 0)))
}

const fn default_limit() -> usize {
    20
}
const fn default_zero() -> usize {
    0
}

/// `/internal_txs/{user_id}`
#[api_v2_operation]
pub async fn my_internal_txs(
    user_id: web::Path<String>,
    query: web::Query<InternalTxQuery>,
    data: web::Data<AppState>,
) -> Result<Json<Vec<InternalTxResponse>>, actix_web::Error> {
    let user_id = user_id.into_inner();
    let limit = min(query.limit, 100);

    let base_query: &'static str = const_format::formatcp!(
        r#"
select i.time       as time,
       af.l2_pubkey as user_from,
       at.l2_pubkey as user_to,
       i.asset      as asset,
       i.amount     as amount
from {} i
inner join {} af on af.id = i.user_from
inner join {} at on at.id = i.user_to
where "#,
        INTERNALTX,
        ACCOUNT,
        ACCOUNT
    );
    let (user_condition, args_n) = match query.side {
        Side::From => ("i.user_from = $1", 1),
        Side::To => ("i.user_to = $1", 1),
        Side::Both => ("i.user_from = $1 or i.user_to = $2", 2),
    };

    let time_condition = match (query.start_time, query.end_time) {
        (Some(_), Some(_)) => Some(format!("i.time >= ${} and i.time <= ${}", args_n + 1, args_n + 2)),
        (Some(_), None) => Some(format!("i.time >= ${}", args_n + 1)),
        (None, Some(_)) => Some(format!("i.time <= ${}", args_n + 1)),
        (None, None) => None,
    };

    let condition = match time_condition {
        Some(time_condition) => format!("({}) and {}", user_condition, time_condition),
        None => user_condition.to_string(),
    };

    let constraint = format!("limit {} offset {}", limit, query.offset);
    let sql_query = format!("{}{}{}", base_query, condition, constraint);

    let query_as = sqlx::query_as(sql_query.as_str());

    let query_as = match query.side {
        Side::To | Side::From => query_as.bind(user_id.clone()),
        Side::Both => query_as.bind(user_id.clone()).bind(user_id),
    };

    let query_as = match (query.start_time, query.end_time) {
        (Some(start_time), Some(end_time)) => query_as.bind(start_time).bind(end_time),
        (Some(start_time), None) => query_as.bind(start_time),
        (None, Some(end_time)) => query_as.bind(end_time),
        (None, None) => query_as,
    };

    let txs: Vec<InternalTxResponse> = query_as
        .fetch_all(&data.db)
        .await
        .map_err(|err| actix_web::Error::from(RpcError::from(err)))?;

    Ok(Json(txs))
}
