use actix_web::{web, HttpRequest, Responder};
use serde_json::json;
use std::{
    time::{SystemTime, UNIX_EPOCH},
    vec,
};

use super::errors::RpcError;
use super::types::KlineReq;

use super::mock;

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

pub async fn history(req: web::Query<KlineReq>) -> Result<String, RpcError> {
    log::debug!("kline req {:?}", req);
    let no_data = false;
    if no_data {
        return Ok(json!({
            "s": "no_data"
        })
        .to_string());
    } else {
        return serde_json::to_string(&mock::fake_kline_result(&req)).map_err(|_e| RpcError::unknown("failed to serialize"));
    }
}
