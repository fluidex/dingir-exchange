use actix_web::web::{self, Data, Json};
use actix_web::{HttpRequest, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    time::{Duration, SystemTime, UNIX_EPOCH},
    vec,
};

use super::errors::RpcError;
use super::types::{KlineReq, KlineResult, TickerResult};
use crate::restapi::state;

use super::mock;

use crate::models::tablenames::MARKETTRADE;

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
        "symbols_types": [{"name": "ETH_USDT", "value": "ETH_USDT"}],
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
            "name": "ETH_USDT",
            "ticker": "ETH_USDT",
            "description": "ETH_USDT",
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

use chrono::{self, DurationRound};
use futures::TryStreamExt;
use rust_decimal::{prelude::*, Decimal};
use sqlx::types::chrono::{DateTime, NaiveDateTime, Utc};

#[derive(sqlx::FromRow, Debug, Clone)]
struct TickerItem {
    first: Decimal,
    last: Decimal,
    max: Decimal,
    min: Decimal,
    sum: Decimal,
    quote_sum: Decimal,
}

#[derive(Deserialize)]
pub struct TickerInv(#[serde(with = "humantime_serde")] Duration);

#[cfg(sqlxverf)]
fn sqlverf_ticker() -> impl std::any::Any{
    sqlx::query!(
        "select first(price, time), last(price, time), max(price), min(price), 
        sum(amount), sum(quote_amount) as quote_sum from market_trade where market = $1 and time > $2",
        "USDT_ETH",
        NaiveDateTime::from_timestamp(100_000_000, 0)
    )
}

pub async fn ticker(
    req: HttpRequest,
    web::Path((TickerInv(ticker_inv), market_name)): web::Path<(TickerInv, String)>,
    app_state: Data<state::AppState>,
) -> Result<Json<TickerResult>, RpcError> {
    let cache = req.app_data::<state::AppCache>().expect("App cache not found");
    let now_ts: DateTime<Utc> = SystemTime::now().into();
    let update_inv = app_state.config.trading.ticker_update_interval;
    let ticker_ret_cache = &mut cache.trading.borrow_mut().ticker_ret_cache;

    if let Some(cached_resp) = ticker_ret_cache.get(&market_name) {
        //consider systemtime may wraparound, we set the valid
        //range of cache is [-inv, +inv] on now
        let now_ts_dur = Duration::from_secs(now_ts.timestamp() as u64);
        let cached_now = Duration::from_secs(cached_resp.to);
        log::debug!(
            "cache judge {}, {}, {}",
            cached_now.as_secs(),
            update_inv.as_secs(),
            now_ts_dur.as_secs()
        );
        if cached_now + update_inv > now_ts_dur && now_ts_dur > cached_now - update_inv {
            log::debug!("use cached response");
            return Ok(Json(cached_resp.clone()));
        }
    }

    let ticker_inv = if ticker_inv > app_state.config.trading.ticker_interval {
        app_state.config.trading.ticker_interval
    } else {
        ticker_inv
    };

    let update_inv = chrono::Duration::from_std(update_inv).map_err(|e| RpcError::unknown(&e.to_string()))?;
    let ticker_inv = chrono::Duration::from_std(ticker_inv).map_err(|e| RpcError::unknown(&e.to_string()))?;
    let now_ts = now_ts.duration_trunc(update_inv).map_err(|e| RpcError::unknown(&e.to_string()))?;

    let core_query = format!(
        "select first(price, time), last(price, time), max(price), min(price), 
        sum(amount), sum(quote_amount) as quote_sum from {} where market = $1 and time > $2",
        MARKETTRADE
    );

    let from_ts = now_ts
        .clone()
        .checked_sub_signed(ticker_inv)
        .ok_or_else(|| RpcError::unknown("Internal clock error"))?;
    log::debug!("query ticker from {} to {}", from_ts, now_ts);

    let ticker_ret: TickerItem = sqlx::query_as(&core_query)
        .bind(&market_name)
        .bind(from_ts.naive_utc())
        .fetch_one(&app_state.db)
        .await?;

    let ret = TickerResult {
        market: market_name.clone(),
        change: (ticker_ret.last - ticker_ret.first)
            .checked_div(ticker_ret.last)
            .and_then(|x| x.to_f32())
            .unwrap_or(9999.9),
        last: ticker_ret.last.to_f32().unwrap_or(0.0),
        high: ticker_ret.max.to_f32().unwrap_or(0.0),
        low: ticker_ret.min.to_f32().unwrap_or(0.0),
        volume: ticker_ret.sum.to_f32().unwrap_or(0.0),
        quote_volume: ticker_ret.quote_sum.to_f32().unwrap_or(0.0),
        from: from_ts.timestamp() as u64,
        to: now_ts.timestamp() as u64,
    };

    //update cache
    ticker_ret_cache.insert(market_name, ret.clone());
    Ok(Json(ret))
}

#[derive(sqlx::FromRow, Debug, Clone)]
struct KlineItem {
    ts: NaiveDateTime,
    first: Decimal,
    last: Decimal,
    max: Decimal,
    min: Decimal,
    sum: Decimal,
}

#[derive(Serialize, Clone, Debug)]
pub struct TradeViewError(RpcError);

impl<T> From<T> for TradeViewError
where
    T: Into<RpcError>,
{
    fn from(original: T) -> TradeViewError {
        TradeViewError(Into::into(original))
    }
}

use actix_web::{http::StatusCode, HttpResponse};

impl std::fmt::Display for TradeViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl actix_web::error::ResponseError for TradeViewError {
    fn status_code(&self) -> StatusCode {
        StatusCode::OK
    }

    fn error_response(&self) -> HttpResponse {
        // all http response are 200. we handle the error inside json
        HttpResponse::build(StatusCode::OK).json(json!(
        {
            "s": "error",
            "errmsg": &self.0.message,
        }))
    }
}

pub async fn history(req_origin: HttpRequest, app_state: Data<state::AppState>) -> Result<Json<KlineResult>, TradeViewError> {
    let req: web::Query<KlineReq> = web::Query::from_query(req_origin.query_string())?;
    let req = req.into_inner();
    log::debug!("kline req {:?}", req);

    if req.usemock.is_some() {
        log::debug!("Use mock mode");
        return Ok(Json(mock::fake_kline_result(&req)));
    }

    let core_query = format!(
        "select time_bucket($1, time) as ts, first(price, time), 
    last(price, time), max(price), min(price), sum(amount) from {} 
    where market = $2 and time > $3 and time < $4
    group by ts order by ts asc",
        MARKETTRADE
    );

    let mut query_rows = sqlx::query_as::<_, KlineItem>(&core_query)
        .bind(std::time::Duration::new(req.resolution as u64 * 60, 0)) // TODO: remove this magic number
        .bind(&req.symbol)
        .bind(NaiveDateTime::from_timestamp(req.from as i64, 0))
        .bind(NaiveDateTime::from_timestamp(req.to as i64, 0))
        .fetch(&app_state.db);

    let mut out_t: Vec<i32> = Vec::new();
    let mut out_c: Vec<f32> = Vec::new();
    let mut out_o: Vec<f32> = Vec::new();
    let mut out_h: Vec<f32> = Vec::new();
    let mut out_l: Vec<f32> = Vec::new();
    let mut out_v: Vec<f32> = Vec::new();

    while let Some(item) = query_rows.try_next().await? {
        out_t.push(item.ts.timestamp() as i32);
        out_c.push(item.last.to_f32().unwrap_or(0.0));
        out_o.push(item.first.to_f32().unwrap_or(0.0));
        out_h.push(item.max.to_f32().unwrap_or(0.0));
        out_l.push(item.min.to_f32().unwrap_or(0.0));
        out_v.push(item.sum.to_f32().unwrap_or(0.0));
    }

    log::debug!("Query {} results", out_t.len());

    if out_t.is_empty() {
        let next_query = format!("select time from {} where time < $1 order by time desc limit 1", MARKETTRADE);
        let nxt = sqlx::query_scalar(&next_query)
            .bind(NaiveDateTime::from_timestamp(req.from as i64, 0))
            .fetch_optional(&app_state.db)
            .await?
            .map(|x: NaiveDateTime| x.timestamp() as i32);

        return Ok(Json(KlineResult {
            s: String::from("no_data"),
            t: out_t,
            c: out_c,
            o: out_o,
            h: out_h,
            l: out_l,
            v: out_v,
            nxt,
        }));
    }

    Ok(Json(KlineResult {
        s: String::from("ok"),
        t: out_t,
        c: out_c,
        o: out_o,
        h: out_h,
        l: out_l,
        v: out_v,
        nxt: None,
    }))
}
