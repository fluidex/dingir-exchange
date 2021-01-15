use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct KlineReq {
    pub from: i32,
    pub to: i32,
    pub symbol: String,
    pub resolution: i32,
}
#[derive(Serialize, Default)]
pub struct KlineResult {
    pub s: String,   // status, 'ok' or 'no_data' etc
    pub t: Vec<i32>, // timestamp
    pub c: Vec<f32>, // closing price
    pub o: Vec<f32>, // opening price
    pub h: Vec<f32>, // highest price
    pub l: Vec<f32>, // lowest price
    pub v: Vec<f32>, // trading volume
}

#[derive(Serialize, Copy, Clone)]
pub struct UserInfo {
    pub user_id: i64,
}
