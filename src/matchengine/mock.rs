use crate::asset::{self, AssetManager, BalanceManager};
use crate::config;
use crate::matchengine::{controller, market};
use crate::message::{self, Message};
use crate::models::BalanceHistory;
use crate::types::OrderEventType;
use rust_decimal_macros::*;

pub fn get_simple_market_config() -> config::Market {
    config::Market {
        name: String::from("ETH_USDT"),
        base: config::MarketUnit { name: eth(), prec: 4 },   // amount: xx.xxxx
        quote: config::MarketUnit { name: usdt(), prec: 2 }, // price xx.xx
        fee_prec: 2,
        min_amount: dec!(0.01),
        disable_self_trade: false,
    }
}
pub fn get_integer_prec_market_config() -> config::Market {
    config::Market {
        name: String::from("ETH_USDT"),
        base: config::MarketUnit { name: eth(), prec: 0 },
        quote: config::MarketUnit { name: usdt(), prec: 0 },
        fee_prec: 0,
        min_amount: dec!(0),
        disable_self_trade: true,
    }
}
pub fn get_simple_asset_config(prec: u32) -> Vec<config::Asset> {
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
pub fn usdt() -> String {
    String::from("USDT")
}
pub fn eth() -> String {
    String::from("ETH")
}
pub fn get_simple_asset_manager(assets: Vec<config::Asset>) -> AssetManager {
    AssetManager::new(&assets).unwrap()
}
pub fn get_simple_balance_manager(assets: Vec<config::Asset>) -> BalanceManager {
    BalanceManager::new(&assets).unwrap()
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