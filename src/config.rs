use config_rs::{Config, File};
use fluidex_common::rust_decimal::Decimal;
use serde::de;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Asset {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub chain_id: i16,
    pub token_address: String,
    pub rollup_token_id: i32,
    pub prec_save: u32,
    pub prec_show: u32,
    pub logo_uri: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MarketUnit {
    pub asset_id: String,
    pub prec: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Market {
    pub name: String,
    pub base: String,
    pub quote: String,
    pub amount_prec: u32,
    pub price_prec: u32,
    pub fee_prec: u32,
    pub min_amount: Decimal,
}

impl Default for MarketUnit {
    fn default() -> Self {
        MarketUnit {
            asset_id: "".to_string(),
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
            amount_prec: 0,
            price_prec: 0,
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

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum OrderSignatrueCheck {
    None,
    // auto means check sig only if sig != ""
    Auto,
    Needed,
}

impl<'de> de::Deserialize<'de> for OrderSignatrueCheck {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;

        match s.as_ref() {
            "true" => Ok(OrderSignatrueCheck::Needed),
            "false" => Ok(OrderSignatrueCheck::None),
            "auto" => Ok(OrderSignatrueCheck::Auto),
            _ => Err(serde::de::Error::custom("unexpected specification for order sig check policy")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
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
    pub disable_self_trade: bool,
    pub disable_market_order: bool,
    pub check_eddsa_signatue: OrderSignatrueCheck,
    pub user_order_num_limit: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            debug: false,
            db_log: Default::default(),
            db_history: Default::default(),
            history_persist_policy: PersistPolicy::ToMessage,
            market_from_db: true,
            assets: Vec::new(),
            markets: Vec::new(),
            consumer_group: "kline_data_fetcher".to_string(),
            brokers: "127.0.0.1:9092".to_string(),
            persist_interval: 3600,
            slice_interval: 86400,
            slice_keeptime: 86400 * 3,
            history_thread: 10,
            cache_timeout: 0.45,
            disable_self_trade: true,
            disable_market_order: false,
            check_eddsa_signatue: OrderSignatrueCheck::None,
            user_order_num_limit: 1000,
        }
    }
}

impl Settings {
    pub fn new() -> Self {
        // Initializes with `config/default.yaml`.
        let mut conf = Config::default();
        conf.merge(File::with_name("config/default")).unwrap();

        // Merges with `config/RUN_MODE.yaml` (development as default).
        let run_mode = dotenv::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        conf.merge(File::with_name(&format!("config/{}", run_mode)).required(false))
            .unwrap();

        conf.try_into().unwrap()
    }
}
