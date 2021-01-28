use super::types::{UserInfo, TickerResult};
use super::config::Settings;

use sqlx::postgres::Postgres;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;
pub struct AppState {
    pub user_addr_map: Mutex<HashMap<String, UserInfo>>,
    pub db: sqlx::pool::Pool<Postgres>,
    pub config: Settings,
}

#[derive(Debug)]
pub struct TradingData {
    pub ticker_ret_cache: Option<TickerResult>,
}

impl TradingData {
    pub fn new() -> Self {
        TradingData {
            ticker_ret_cache: None,
        }
    }
}

//TLS storage
#[derive(Debug)]
pub struct AppCache {
    pub trading : RefCell<TradingData>,
}

impl AppCache {
    pub fn new() -> Self {
        AppCache {
            trading: TradingData::new().into(),
        }
    }
}