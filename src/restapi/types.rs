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

#[derive(Serialize, Copy, Clone)]
pub struct UserInfo {
    pub user_id: i64,
}
