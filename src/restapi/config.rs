use config_rs::{Config, File};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Trading {
    #[serde(with = "humantime_serde")]
    pub ticker_update_interval: std::time::Duration,
    #[serde(with = "humantime_serde")]
    pub ticker_interval: std::time::Duration,
}

impl Default for Trading {
    fn default() -> Self {
        Trading {
            ticker_update_interval: std::time::Duration::from_secs(5),
            ticker_interval: std::time::Duration::from_secs(86_400),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub workers: Option<usize>,
    pub manage_endpoint: Option<String>,
    pub trading: Trading,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            workers: None,
            manage_endpoint: None,
            trading: Default::default(),
        }
    }
}

impl Settings {
    pub fn new() -> Self {
        Config::builder()
            .add_source(File::with_name("config/restapi/default.yaml"))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap()
    }
}
