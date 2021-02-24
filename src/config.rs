use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Asset {
    pub name: String,
    pub prec_save: u32,
    pub prec_show: u32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MarketUnit {
    pub name: String,
    pub prec: u32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Market {
    pub name: String,
    pub base: MarketUnit,
    pub quote: MarketUnit,
    pub fee_prec: u32,
    pub min_amount: Decimal,
}

impl Default for MarketUnit {
    fn default() -> Self {
        MarketUnit {
            name: "".to_string(),
            prec: 0,
        }
    }
}

impl Default for Market {
    fn default() -> Self {
        Market {
            name: "".to_string(),
            fee_prec: 4,
            min_amount: Decimal::from_str("0.01").unwrap(),
            base: Default::default(),
            quote: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum PersistPolicy {
    Dummy,
    Both,
    ToDB,
    ToMessage,
}

use serde::de;

impl<'de> de::Deserialize<'de> for PersistPolicy {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;

        match s.as_ref() {
            "Both" | "both" => Ok(PersistPolicy::Both),
            "Db" | "db" | "DB" => Ok(PersistPolicy::ToDB),
            "Message" | "message" => Ok(PersistPolicy::ToMessage),
            "Dummy" | "dummy" => Ok(PersistPolicy::Dummy),
            _ => Err(serde::de::Error::custom("unexpected specification for persist policy")),
        }
    }
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub debug: bool,
    pub db_log: String,
    pub db_history: String,
    pub history_persist_policy: PersistPolicy,
    pub market_from_db: bool,
    pub assets: Vec<Asset>,
    pub markets: Vec<Market>,
    pub brokers: String,
    pub consumer_group: String,
    pub persist_interval: i32,
    pub slice_interval: i32,
    pub slice_keeptime: i32,
    pub history_thread: i32,
    pub cache_timeout: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            debug: false,
            db_log: Default::default(),
            db_history: Default::default(),
            history_persist_policy: PersistPolicy::ToMessage,
            market_from_db: false,
            assets: Vec::new(),
            markets: Vec::new(),
            consumer_group: "kline_data_fetcher".to_string(),
            brokers: "127.0.0.1:9092".to_string(),
            persist_interval: 3600,
            slice_interval: 86400,
            slice_keeptime: 86400 * 3,
            history_thread: 10,
            cache_timeout: 0.45,
        }
    }
}
