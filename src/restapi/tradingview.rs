use actix_web::web::{self, Json, Data};
use actix_web::{HttpRequest, Responder};
use serde_json::json;
use std::{
    time::{SystemTime, UNIX_EPOCH},
    vec,
};

use super::errors::RpcError;
use super::types::{KlineReq, KlineResult};

use super::mock;

const TRADERECORD: &str = "trade_record";

// All APIs here follow https://zlq4863947.gitbook.io/tradingview/3-shu-ju-bang-ding/udf

pub async fn unix_timestamp(_req: HttpRequest) -> impl Responder {
    format!("{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs())
}
pub async fn chart_config(_req: HttpRequest) -> impl Responder {
    let value = json!({
        "supports_search": true,
        "supports_group_request": false,
        "supports_marks": false,
        "supports_timescale_marks": false,
        "supports_time": true,
        "exchanges": [
            {

                "value": "STOCK",
                "name": "Exchange",
                "desc": ""
            }
        ],
        "symbols_types": [{"name": "ETH_BTC", "value": "ETH_BTC"}],
        "supported_resolutions": [1, 5, 15, 30, 60, 120, 240, 360, 720, 1440, 4320, 10080] // minutes
    });
    value.to_string()
}

// TODO: Result<web::Json<T>, RpcError>
pub async fn symbols(req: HttpRequest) -> Result<String, RpcError> {
    let qstring = qstring::QString::from(req.query_string());
    let symbol = qstring.get("symbol");
    if symbol.is_none() {
        return Err(RpcError::bad_request("no `symbol` param"));
    };
    let _market = symbol.unwrap().split(':').last().unwrap();
    log::debug!("kline get symbol {:?}", symbol);
    Ok(json!(
        {
            "name": "ETH_BTC",
            "ticker": "ETH_BTC",
            "description": "ETH_BTC",
            "type": "btc",
            "session": "24x7",
            "exchange": "STOCK",
            "listed_exchange": "STOCK",
            "timezone": "Asia/Singapore",
            "has_intraday": true,
            "has_daily": true,
            "has_weekly_and_monthly": true,
            "pricescale": 10000,
            "minmovement": 1,
            "minmov": 1,
            "minmovement2": 0,
            "minmov2": 0
        }
    )
    .to_string())
}

use sqlx::types::chrono::NaiveDateTime;
use rust_decimal::{prelude::*, Decimal};
use futures::TryStreamExt;

#[derive(sqlx::FromRow, Debug, Clone)]
struct KlineItem {
    ts: NaiveDateTime,
    first: Decimal,
    last: Decimal,
    max: Decimal,
    min: Decimal,
    sum: Decimal,
}

pub async fn history(req_origin: HttpRequest) -> Result<Json<KlineResult>, RpcError> {
    let req : web::Query<KlineReq> = web::Query::from_query(req_origin.query_string())?;
    let req = req.into_inner();
    let app_state = req_origin.app_data::<Data<crate::AppState>>().expect("App state not found");
    log::debug!("kline req {:?}", req);

/*
    let no_data = false;
    if no_data {
        return Ok(json!({
            "s": "no_data"
        })
        .to_string());
    } else {
        return serde_json::to_string(&mock::fake_kline_result(&req)).map_err(|_e| RpcError::unknown("failed to serialize"));
    }*/
    if let Some(_) = req.usemock {
        log::debug!("Use mock mode");
        return Ok(Json(mock::fake_kline_result(&req)));
    }

    let core_query = format!("select time_bucket($1, time) as ts, first(price, time), 
    last(price, time), max(price), min(price), sum(amount) from {} 
    where market = $2 and time > $3 and time < $4
    group by ts order by ts desc", 
    TRADERECORD);

    let mut query_rows = sqlx::query_as::<_, KlineItem>(&core_query)
        .bind(std::time::Duration::new(req.resolution as u64, 0))
        .bind(&req.symbol)
        .bind(NaiveDateTime::from_timestamp(req.from as i64, 0))
        .bind(NaiveDateTime::from_timestamp(req.to as i64, 0))
        .fetch(&app_state.db);

    let mut resp_t : Vec<i32> = Vec::new();
    let mut resp_c : Vec<f32> = Vec::new();
    let mut resp_o : Vec<f32> = Vec::new();
    let mut resp_h : Vec<f32> = Vec::new();
    let mut resp_l : Vec<f32> = Vec::new();
    let mut resp_v : Vec<f32> = Vec::new();

    while let Some(item) = query_rows.try_next().await? {
        resp_t.push(item.ts.timestamp() as i32);
        resp_c.push(item.last.to_f32().unwrap_or(0.0));
        resp_o.push(item.first.to_f32().unwrap_or(0.0));
        resp_h.push(item.max.to_f32().unwrap_or(0.0));
        resp_l.push(item.min.to_f32().unwrap_or(0.0));
        resp_v.push(item.sum.to_f32().unwrap_or(0.0));
    }

    log::debug!("Query {} results", resp_t.len());

    if resp_t.is_empty() {
        return Ok(Json(KlineResult{s: String::from("no_data"), 
            t: resp_t, c: resp_c, o: resp_o, h: resp_h, l: resp_l, v: resp_v,}));
    }

    Ok(Json(KlineResult{s: String::from("ok"), 
    t: resp_t, c: resp_c, o: resp_o, h: resp_h, l: resp_l, v: resp_v,}))

    //let next_query = format!("select time from {} where time < $1 order by time desc limit 1", TRADERECORD);

}
