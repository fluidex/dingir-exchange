use crate::models::tablenames::{MARKET, MARKETTRADE};
use crate::models::MarketDesc;
use crate::restapi::errors::RpcError;
use crate::restapi::types::{KlineReq, KlineResult, TickerResult};
use crate::restapi::{mock, state};
use actix_web::Responder;
use paperclip::actix::web::{self, HttpRequest, Json};
use paperclip::actix::{api_v2_operation, Apiv2Schema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// All APIs here follow https://zlq4863947.gitbook.io/tradingview/3-shu-ju-bang-ding/udf

pub async fn unix_timestamp(_req: HttpRequest) -> impl Responder {
    format!("{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs())
}

static DEFAULT_EXCHANGE: &str = "test";
static DEFAULT_SYMBOL: &str = "tradepair";
static DEFAULT_SESSION: &str = "24x7";

pub async fn chart_config(_req: HttpRequest) -> impl Responder {
    log::debug!("request config");
    let value = json!({
        "supports_search": true,
        "supports_group_request": false,
        "supports_marks": false,
        "supports_timescale_marks": false,
        "supports_time": true,
        "exchanges": [
            {

                "value": "test",
                "name": "Test Zone",
                "desc": "Current default exchange"
            }
        ],
        "symbols_types": [],
        "supported_resolutions": [1, 5, 15, 30, 60, 120, 240, 360, 720, 1440, 4320, 10080] // minutes
    });
    value.to_string()
}

#[derive(Deserialize)]
pub struct SymbolQueryReq {
    symbol: String,
}

#[derive(Serialize)]
pub struct Symbol {
    name: String,
    ticker: String,
    #[serde(rename = "type")]
    sym_type: String,
    session: String,
    exchange: String,
    listed_exchange: String,
    //TODO: we can use a enum
    timezone: String,
    minmov: u32,
    pricescale: u32,
    //TODO: this two field may has been deprecated
    minmovement2: u32,
    minmov2: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    minmove2: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fractional: Option<bool>,
    has_intraday: bool,
    has_daily: bool,
    has_weekly_and_monthly: bool,
}

impl Default for Symbol {
    fn default() -> Self {
        Symbol {
            name: String::default(),
            ticker: String::default(),
            sym_type: DEFAULT_SYMBOL.to_string(),
            session: DEFAULT_SESSION.to_string(),
            exchange: DEFAULT_EXCHANGE.to_string(),
            listed_exchange: DEFAULT_EXCHANGE.to_string(),
            timezone: "Etc/UTC".to_string(),
            minmov: 1,
            pricescale: 100,
            minmovement2: 0,
            minmov2: 0,
            minmove2: None,
            fractional: None,
            has_intraday: true,
            has_daily: true,
            has_weekly_and_monthly: true,
        }
    }
}

type SymNameAndTicker = String;
type FullName = String;

/*
   According to the symbology of chart [https://github.com/serdimoa/charting/blob/master/Symbology.md],
   it may refer a symbol by EXCHANGE:SYMBOL format, to made things easy, our symbology always specify
   a ticker which is identify with symbol name itself, which is the market_name used in matchingengine
   and db record ({base asset}_{quote asset} or an specified name), to keep all API running correct,
   and the user of chart libaray can use symbol() method to acquire current symbol name for their API
*/
fn symbology(origin: &MarketDesc) -> (SymNameAndTicker, FullName) {
    let s_name = format!("{}_{}", origin.base_asset, origin.quote_asset);
    (
        origin.market_name.clone().unwrap_or_else(|| s_name.clone()),
        origin
            .market_name
            .clone()
            .map_or_else(|| s_name.clone(), |n| format!("{}({})", n, s_name)),
    )
}

impl From<MarketDesc> for Symbol {
    fn from(origin: MarketDesc) -> Self {
        let (name, _) = symbology(&origin);
        let pricescale = 10u32.pow(origin.min_amount.scale());
        //simply pick the lo part
        let minmov = origin.min_amount.unpack().lo;

        Symbol {
            name: name.clone(),
            ticker: name,
            sym_type: DEFAULT_SYMBOL.to_string(),
            session: DEFAULT_SESSION.to_string(),
            exchange: DEFAULT_EXCHANGE.to_string(),
            listed_exchange: DEFAULT_EXCHANGE.to_string(),
            timezone: "Etc/UTC".to_string(),
            minmov,
            pricescale,
            minmovement2: 0,
            minmov2: 0,
            minmove2: None,
            fractional: None,
            has_intraday: true,
            has_daily: true,
            has_weekly_and_monthly: true,
        }
    }
}

#[cfg(sqlxverf)]
fn sqlverf_symbol_resolve() -> impl std::any::Any {
    (
        sqlx::query_as!(
            MarketDesc,
            "select * from market where 
        (base_asset = $1 AND quote_asset = $2) OR
        (base_asset = $2 AND quote_asset = $1)",
            "UNI",
            "ETH"
        ),
        sqlx::query_as!(MarketDesc, "select * from market where market_name = $1", "Any spec name"),
    )
}

//notice we use the standard symbology (EXCHANGE:SYMBOL) format while consider the exchange part may
//missed, so we return optional exchange part and the symbol part
fn resolve_canionical_symbol(symbol: &str) -> (Option<&str>, &str) {
    match symbol.find(':') {
        Some(pos) => (Some(symbol.get(..pos).unwrap()), symbol.get((pos + 1)..).unwrap()),
        None => (None, symbol),
    }
}

#[cfg(test)]
#[test]
fn test_symbol_resolution() {
    let (ex1, sym1) = resolve_canionical_symbol("test:USDT_ETH");
    assert_eq!(ex1.unwrap(), "test");
    assert_eq!(sym1, "USDT_ETH");

    let (ex2, sym2) = resolve_canionical_symbol("BTC_ETH");
    assert_eq!(ex2, None);
    assert_eq!(sym2, "BTC_ETH");
}

pub async fn symbols(symbol_req: web::Query<SymbolQueryReq>, app_state: web::Data<state::AppState>) -> Result<web::Json<Symbol>, RpcError> {
    let symbol = symbol_req.into_inner().symbol;
    log::debug!("resolve symbol {:?}", symbol);

    //now we simply drop the exg part
    let (_, rsymbol) = resolve_canionical_symbol(&symbol);

    let as_asset: Vec<&str> = rsymbol.split(&['-', '_'][..]).collect();
    let mut queried_market: Option<MarketDesc> = None;
    //try asset first
    if as_asset.len() == 2 {
        log::debug!("query market from asset {}:{}", as_asset[0], as_asset[1]);
        let symbol_query_1 = format!(
            "select * from {} where 
            (base_asset = $1 AND quote_asset = $2) OR
            (base_asset = $2 AND quote_asset = $1)",
            MARKET
        );
        queried_market = sqlx::query_as(&symbol_query_1)
            .bind(as_asset[0])
            .bind(as_asset[1])
            .fetch_optional(&app_state.db)
            .await?;
    }

    let queried_market = if queried_market.is_none() {
        log::debug!("query market from name {}", rsymbol);
        let symbol_query_2 = format!("select * from {} where market_name = $1", MARKET);
        //TODO: would this returning correct? should we just
        //response 404?
        sqlx::query_as(&symbol_query_2).bind(&symbol).fetch_one(&app_state.db).await?
    } else {
        queried_market.unwrap()
    };

    Ok(Json(Symbol::from(queried_market)))
    /*    Ok(json!(
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
    .to_string())*/
}

#[derive(Deserialize, Debug)]
pub struct SymbolSearchQueryReq {
    query: String,
    #[serde(default, rename = "type")]
    sym_type: Option<String>,
    #[serde(default)]
    exchange: Option<String>,
    #[serde(default)]
    limit: u32,
}

#[derive(Serialize)]
pub struct SymbolDesc {
    symbol: String,
    full_name: String, // e.g. BTCE:BTCUSD
    description: String,
    exchange: String,
    ticker: String,
    #[serde(rename = "type")]
    sym_type: String,
}

impl From<MarketDesc> for SymbolDesc {
    fn from(origin: MarketDesc) -> Self {
        let (name, full_name) = symbology(&origin);

        SymbolDesc {
            symbol: name.clone(),
            full_name: format!("{}:{}", DEFAULT_EXCHANGE, full_name),
            description: String::default(),
            sym_type: DEFAULT_SYMBOL.to_string(),
            ticker: name,
            exchange: DEFAULT_EXCHANGE.to_string(),
        }
    }
}

#[cfg(sqlxverf)]
fn sqlverf_symbol_search() -> impl std::any::Any {
    sqlx::query_as!(
        MarketDesc,
        "select * from market where base_asset = $1 OR quote_asset = $1 OR market_name = $1",
        "UNI"
    )
}

pub async fn search_symbols(
    symbol_search_req: web::Query<SymbolSearchQueryReq>,
    app_state: web::Data<state::AppState>,
) -> Result<web::Json<Vec<SymbolDesc>>, RpcError> {
    let symbol_query = symbol_search_req.into_inner();
    log::debug!("search symbol {:?}", symbol_query);

    //query should not contain exchange part?
    let (_, rsymbol) = resolve_canionical_symbol(&symbol_query.query);

    let as_asset: Vec<&str> = rsymbol.split(&['-', '_'][..]).collect();
    let limit_query = if symbol_query.limit == 0 {
        "".to_string()
    } else {
        format!(" limit {}", symbol_query.limit)
    };
    //use different query type
    //try asset first

    let ret: Vec<MarketDesc> = if as_asset.len() == 2 {
        log::debug!("query symbol as trade pair {}:{}", as_asset[0], as_asset[1]);
        let symbol_query_1 = format!(
            "select * from {} where (base_asset = $1 AND quote_asset = $2) OR
            (base_asset = $2 AND quote_asset = $1){}",
            MARKET, limit_query
        );
        sqlx::query_as(&symbol_query_1)
            .bind(as_asset[0])
            .bind(as_asset[1])
            .fetch_all(&app_state.db)
            .await?
    } else {
        log::debug!("query symbol as name {}", rsymbol);
        let symbol_query_2 = format!(
            "select * from {} where base_asset = $1 OR quote_asset = $1 OR market_name = $1{}",
            MARKET, limit_query
        );
        sqlx::query_as(&symbol_query_2).bind(rsymbol).fetch_all(&app_state.db).await?
    };

    Ok(Json(ret.into_iter().map(From::from).collect()))
}

use chrono::{self, DurationRound};
use fluidex_common::rust_decimal::{prelude::*, Decimal};
use futures::TryStreamExt;
use sqlx::types::chrono::{DateTime, NaiveDateTime, Utc};

#[derive(sqlx::FromRow, Debug, Clone)]
struct TickerItem {
    first: Option<Decimal>,
    last: Option<Decimal>,
    max: Option<Decimal>,
    min: Option<Decimal>,
    sum: Option<Decimal>,
    quote_sum: Option<Decimal>,
}

// gupeng

#[derive(Serialize, Deserialize, Apiv2Schema)]
pub struct TickerInv(#[serde(with = "humantime_serde")] Duration);

#[cfg(sqlxverf)]
fn sqlverf_ticker() -> impl std::any::Any {
    sqlx::query_as!(
        TickerItem,
        "select first(price, time), last(price, time), max(price), min(price), 
        sum(amount), sum(quote_amount) as quote_sum from market_trade where market = $1 and time > $2",
        "USDT_ETH",
        NaiveDateTime::from_timestamp(100_000_000, 0)
    )
}

#[api_v2_operation]
pub async fn ticker(
    req: HttpRequest,
    path: web::Path<(TickerInv, String)>,
    app_state: web::Data<state::AppState>,
) -> Result<Json<TickerResult>, actix_web::Error> {
    let (TickerInv(ticker_inv), market_name) = path.into_inner();
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
        .checked_sub_signed(ticker_inv)
        .ok_or_else(|| RpcError::unknown("Internal clock error"))?;
    log::debug!("query ticker from {} to {}", from_ts, now_ts);

    let ticker_ret: TickerItem = sqlx::query_as(&core_query)
        .bind(&market_name)
        .bind(from_ts.naive_utc())
        .fetch_one(&app_state.db)
        .await
        .map_err(|err| actix_web::Error::from(RpcError::from(err)))?;

    let ret = TickerResult {
        market: market_name.clone(),
        change: match ticker_ret.last {
            Some(lst) => ticker_ret
                .first
                .and_then(|fst| lst.checked_sub(fst))
                .and_then(|r1| r1.checked_div(lst))
                .as_ref()
                .and_then(Decimal::to_f32)
                .unwrap_or(9999.9),
            None => 0.0,
        },
        last: ticker_ret.last.as_ref().and_then(Decimal::to_f32).unwrap_or(0.0),
        high: ticker_ret.max.as_ref().and_then(Decimal::to_f32).unwrap_or(0.0),
        low: ticker_ret.min.as_ref().and_then(Decimal::to_f32).unwrap_or(0.0),
        volume: ticker_ret.sum.as_ref().and_then(Decimal::to_f32).unwrap_or(0.0),
        quote_volume: ticker_ret.quote_sum.as_ref().and_then(Decimal::to_f32).unwrap_or(0.0),
        from: from_ts.timestamp() as u64,
        to: now_ts.timestamp() as u64,
    };

    //update cache
    ticker_ret_cache.insert(market_name, ret.clone());
    Ok(Json(ret))
}

#[derive(sqlx::FromRow, Debug, Clone)]
struct KlineItem {
    ts: Option<NaiveDateTime>,
    first: Option<Decimal>,
    last: Option<Decimal>,
    max: Option<Decimal>,
    min: Option<Decimal>,
    sum: Option<Decimal>,
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

#[cfg(sqlxverf)]
use std::convert::TryFrom;

#[cfg(sqlxverf)]
fn sqlverf_history() -> impl std::any::Any {
    sqlx::query_as!(
        KlineItem,
        "select time_bucket($1, time) as ts, first(price, time), 
    last(price, time), max(price), min(price), sum(amount) from market_trade
    where market = $2 and time > $3 and time < $4
    group by ts order by ts asc",
        sqlx::postgres::types::PgInterval::try_from(std::time::Duration::new(3600, 0)).unwrap(),
        "ETH_USDT",
        NaiveDateTime::from_timestamp(100_000_000, 0),
        NaiveDateTime::from_timestamp(100_000_000, 0),
    )
}

pub async fn history(req_origin: HttpRequest, app_state: web::Data<state::AppState>) -> Result<Json<KlineResult>, TradeViewError> {
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
        out_t.push(item.ts.as_ref().map(NaiveDateTime::timestamp).unwrap_or(0) as i32);
        out_c.push(item.last.as_ref().and_then(Decimal::to_f32).unwrap_or(0.0));
        out_o.push(item.first.as_ref().and_then(Decimal::to_f32).unwrap_or(0.0));
        out_h.push(item.max.as_ref().and_then(Decimal::to_f32).unwrap_or(0.0));
        out_l.push(item.min.as_ref().and_then(Decimal::to_f32).unwrap_or(0.0));
        out_v.push(item.sum.as_ref().and_then(Decimal::to_f32).unwrap_or(0.0));
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
