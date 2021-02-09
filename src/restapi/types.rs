use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct KlineReq {
    pub from: i32,
    pub to: i32,
    pub symbol: String,
    pub resolution: i32,
    pub usemock: Option<String>,
}
#[derive(Serialize, Deserialize, Default)]
pub struct KlineResult {
    pub s: String, // status, 'ok' or 'no_data' etc
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub t: Vec<i32>, // timestamp
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub c: Vec<f32>, // closing price
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub o: Vec<f32>, // opening price
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub h: Vec<f32>, // highest price
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub l: Vec<f32>, // lowest price
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub v: Vec<f32>, // trading volume
    #[serde(rename = "nextTime", skip_serializing_if = "Option::is_none")]
    pub nxt: Option<i32>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct TickerResult {
    pub market: String,
    #[serde(rename = "price_change_percent")]
    pub change: f32,
    pub last: f32,
    pub high: f32,
    pub low: f32,
    pub volume: f32,
    pub quote_volume: f32,
    pub from: u64,
    pub to: u64,
}

#[derive(Serialize, Copy, Clone)]
pub struct UserInfo {
    pub user_id: i64,
}

#[derive(Serialize, Deserialize)]
pub struct MarketTrade {
    pub time: i32,
    pub amount: f32,
    pub quote_amount: f32,
    pub price: f32,
    pub fee: f32,
}

#[derive(Serialize, Deserialize)]
pub struct OrderTradeResult {
    pub trades: Vec<MarketTrade>,
}
