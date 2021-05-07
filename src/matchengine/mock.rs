use crate::asset::{self, AssetManager, BalanceManager};
use crate::config;
use crate::matchengine::{controller, market};
use crate::message::{self, Message, UnifyMessageManager};
use crate::models::BalanceHistory;
use crate::types::OrderEventType;
use rust_decimal_macros::*;

use std::fs::File;
use std::io::Write;

// TODO: fix
pub fn get_simple_market_config() -> config::Market {
    config::Market {
        name: String::from("ETH_USDT"),
        base: config::MarketUnit { name: MockAsset::ETH.name(), prec: 4 },   // amount: xx.xxxx
        quote: config::MarketUnit { name: MockAsset::USDT.name(), prec: 2 }, // price xx.xx
        fee_prec: 2,
        min_amount: dec!(0.01),
        disable_self_trade: false,
    }
}

// TODO: fix
pub fn get_integer_prec_market_config() -> config::Market {
    config::Market {
        name: String::from("ETH_USDT"),
        base: config::MarketUnit { name: MockAsset::ETH.name(), prec: 0 },
        quote: config::MarketUnit { name: MockAsset::USDT.name(), prec: 0 },
        fee_prec: 0,
        min_amount: dec!(0),
        disable_self_trade: true,
    }
}

pub fn get_simple_asset_config(prec: u32) -> Vec<config::Asset> {
    vec![
        config::Asset {
            symbol: MockAsset::USDT.symbol(),
            name: MockAsset::USDT.name(),
            chain_id: 1,
            token_address: MockAsset::USDT.token_address(),
            is_commonly_quoted: true,
            prec_save: prec,
            prec_show: prec,
            logo_uri: String::default(),
        },
        config::Asset {
            symbol: MockAsset::ETH.symbol(),
            name: MockAsset::ETH.name(),
            chain_id: 1,
            token_address: MockAsset::ETH.token_address(),
            is_commonly_quoted: true,
            prec_save: prec,
            prec_show: prec,
            logo_uri: String::default(),
        },
    ]
}

#[derive(Debug)]
enum MockAsset {
    ETH,
    USDT,
}
impl MockAsset {
    fn symbol(self) -> String {
        match self {
            MockAsset::ETH => String::from("ETH"),
            MockAsset::USDT => String::from("USDT"),
        }
    }
    fn name(self) -> String {
        match self {
            MockAsset::ETH => String::from("Ether"),
            MockAsset::USDT => String::from("Tether USD"),
        }
    }
    fn token_address(self) -> String {
        match self {
            MockAsset::ETH => String::from("0x0000000000000000000000000000000000000000"),
            MockAsset::USDT => String::from("0xdAC17F958D2ee523a2206206994597C13D831ec7"),
        }
    }
}

pub fn get_simple_asset_manager(assets: Vec<config::Asset>) -> AssetManager {
    AssetManager::new(&assets).unwrap()
}
pub fn get_simple_balance_manager(assets: Vec<config::Asset>) -> BalanceManager {
    BalanceManager::new(&assets).unwrap()
}

pub fn get_mocking_persistor() -> Box<dyn controller::IntoPersistor> {
    match std::env::var("KAFKA_BROKER") {
        Ok(val) => Box::new(UnifyMessageManager::new_and_run(&val).unwrap()),
        Err(_) => Box::new(MockPersistor::new()),
    }
}

pub(super) struct MockPersistor {
    //orders: Vec<market::Order>,
    //trades: Vec<market::Trade>,
    pub messages: Vec<crate::message::Message>,
}
impl MockPersistor {
    pub(super) fn new() -> Self {
        Self {
            //orders: Vec::new(),
            //trades: Vec::new(),
            messages: Vec::new(),
        }
    }
}

fn get_market_base_and_quote(market: &str) -> (String, String) {
    let splits: Vec<&str> = market.split("_").collect();
    (splits[0].to_owned(), splits[1].to_owned())
}

impl Drop for MockPersistor {
    fn drop(&mut self) {
        let output_file_name = "output.txt";
        let mut file = File::create(output_file_name).unwrap();
        for item in self.messages.iter() {
            let s = serde_json::to_string(item).unwrap();
            file.write_fmt(format_args!("{}\n", s)).unwrap();
        }
        log::info!("output done")
        //rust file need not to be closed manually
    }
}

impl market::PersistExector for &mut MockPersistor {
    fn put_order(&mut self, order: &market::Order, at_step: OrderEventType) {
        //self.orders.push(order.clone());
        self.messages.push(Message::OrderMessage(Box::new(message::OrderMessage {
            event: at_step,
            order: order.clone(),
            base: get_market_base_and_quote(&*order.market).0,
            quote: get_market_base_and_quote(&*order.market).1,
        })));
    }
    fn put_trade(&mut self, trade: &market::Trade) {
        //self.trades.push(trade.clone());
        self.messages.push(Message::TradeMessage(Box::new(trade.clone())));
    }
}

impl asset::PersistExector for &mut MockPersistor {
    fn put_balance(&mut self, balance: BalanceHistory) {
        self.messages.push(Message::BalanceMessage(Box::new(message::BalanceMessage {
            timestamp: balance.time.timestamp() as f64,
            user_id: balance.user_id as u32,
            asset: balance.asset.clone(),
            business: balance.business.clone(),
            change: balance.change.to_string(),
            balance: balance.balance.to_string(),
            detail: balance.detail,
        })))
    }
}

impl controller::IntoPersistor for MockPersistor {
    fn persistor_for_market<'c>(&'c mut self, _real: bool, _market_tag: (String, String)) -> Box<dyn market::PersistExector + 'c> {
        Box::new(self)
    }
    fn persistor_for_balance<'c>(&'c mut self, _real: bool) -> Box<dyn asset::PersistExector + 'c> {
        Box::new(self)
    }
}
