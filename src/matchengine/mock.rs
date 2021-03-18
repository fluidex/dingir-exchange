use crate::asset::{AssetManager, BalanceManager};
use crate::config;
use rust_decimal_macros::*;

fn get_simple_market_config() -> config::Market {
    config::Market {
        name: String::from("ETH_USDT"),
        base: config::MarketUnit { name: eth(), prec: 4 },   // amount: xx.xxxx
        quote: config::MarketUnit { name: usdt(), prec: 2 }, // price xx.xx
        fee_prec: 2,
        min_amount: dec!(0.01),
        disable_self_trade: false,
    }
}
fn get_integer_prec_market_config() -> config::Market {
    config::Market {
        name: String::from("ETH_USDT"),
        base: config::MarketUnit { name: eth(), prec: 0 },
        quote: config::MarketUnit { name: usdt(), prec: 0 },
        fee_prec: 0,
        min_amount: dec!(0),
        disable_self_trade: true,
    }
}
fn get_simple_asset_config(prec: u32) -> Vec<config::Asset> {
    vec![
        config::Asset {
            name: usdt(),
            prec_save: prec,
            prec_show: prec,
        },
        config::Asset {
            name: eth(),
            prec_show: prec,
            prec_save: prec,
        },
    ]
}
fn usdt() -> String {
    String::from("USDT")
}
fn eth() -> String {
    String::from("ETH")
}
fn get_simple_asset_manager(assets: Vec<config::Asset>) -> AssetManager {
    AssetManager::new(&assets).unwrap()
}
fn get_simple_balance_manager(assets: Vec<config::Asset>) -> BalanceManager {
    BalanceManager::new(&assets).unwrap()
}
