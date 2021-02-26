use super::config::Settings;
use super::types::{TickerResult, UserInfo};

use sqlx::postgres::Postgres;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;
pub struct AppState {
    pub user_addr_map: Mutex<HashMap<String, UserInfo>>,
    pub db: sqlx::pool::Pool<Postgres>,
    pub manage_channel: Option<tonic::transport::channel::Channel>,
    pub config: Settings,
}

#[derive(Debug)]
pub struct TradingData {
    pub ticker_ret_cache: HashMap<String, TickerResult>,
}

impl TradingData {
    pub fn new() -> Self {
        TradingData {
            ticker_ret_cache: HashMap::new(),
        }
    }
}

impl Default for TradingData {
    fn default() -> Self {
        Self::new()
    }
}

//TLS storage
#[derive(Debug)]
pub struct AppCache {
    pub trading: RefCell<TradingData>,
}

impl AppCache {
    pub fn new() -> Self {
        AppCache {
            trading: TradingData::new().into(),
        }
    }
}

impl Default for AppCache {
    fn default() -> Self {
        Self::new()
    }
}
