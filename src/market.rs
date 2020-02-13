use crate::asset::{BalanceManager, BalanceType};
use crate::history::HistoryWriter;
use crate::message::{MessageSender, OrderMessage};
use crate::sequencer::Sequencer;
use crate::types::{self, MarketRole, OrderEventType, Trade};
use crate::utils;
use crate::{config, message};
use anyhow::Result;
use itertools::Itertools;
use rust_decimal::prelude::Zero;
use rust_decimal::Decimal;
use std::iter::Iterator;

use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::cmp::{min, Ordering};

use std::collections::BTreeMap;

use std::rc::Rc;

#[derive(PartialEq, PartialOrd, Eq, Ord)]
pub struct MarketKeyAsk {
    pub order_price: Decimal,
    pub order_id: u64,
}

pub type MarketKey = MarketKeyAsk;

#[derive(PartialEq, PartialOrd, Eq)]
pub struct MarketKeyBid {
    pub order_price: Decimal,
    pub order_id: u64,
}

impl Ord for MarketKeyBid {
    fn cmp(&self, other: &Self) -> Ordering {
        let price_order = self.order_price.cmp(&other.order_price);
        if price_order == Ordering::Equal {
            self.order_id.cmp(&other.order_id).reverse()
        } else {
            price_order.reverse()
        }
    }
}

// TODO: store as string or int?
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum OrderType {
    LIMIT,
    MARKET,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum OrderSide {
    ASK,
    BID,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Order {
    pub id: u64,
    pub market: &'static str,
    #[serde(rename = "type")]
    pub type_: OrderType, // enum
    pub side: OrderSide,
    pub user: u32,
    pub create_time: f64,
    pub update_time: f64,
    pub price: Decimal,
    pub amount: Decimal,
    pub taker_fee: Decimal,
    pub maker_fee: Decimal,
    pub left: Decimal,
    pub freeze: Decimal,
    pub finished_base: Decimal,
    pub finished_quote: Decimal,
    pub finished_fee: Decimal,
}

impl Order {
    pub fn get_ask_key(&self) -> MarketKeyAsk {
        MarketKeyAsk {
            order_price: self.price,
            order_id: self.id,
        }
    }
    pub fn get_bid_key(&self) -> MarketKeyBid {
        MarketKeyBid {
            order_price: self.price,
            order_id: self.id,
        }
    }
}

pub type OrderRc = Rc<RefCell<Order>>;

pub fn is_order_ask(order: &Order) -> bool {
    let side: OrderSide = order.side;
    side == OrderSide::ASK
}

pub struct Market {
    pub name: &'static str,
    pub base: String,
    pub quote: String,
    pub base_prec: u32,
    pub quote_prec: u32,
    pub fee_prec: u32,
    pub min_amount: Decimal,

    pub orders: BTreeMap<u64, OrderRc>,
    pub users: BTreeMap<u32, BTreeMap<u64, OrderRc>>,

    pub asks: BTreeMap<MarketKeyAsk, OrderRc>,
    pub bids: BTreeMap<MarketKeyBid, OrderRc>,

    pub sequencer: Rc<RefCell<Sequencer>>,
    balance_manager: BalanceManagerWrapper,
    pub history_writer: Rc<RefCell<dyn HistoryWriter>>,
    message_sender: MessageSenderWrapper,
}

// FIXME
pub fn asset_exist(_asset: &str) -> bool {
    true
}
// FIXME
pub fn asset_prec(_asset: &str) -> u32 {
    100
}

const MAP_INIT_CAPACITY: usize = 1024;

struct MessageSenderWrapper {
    inner: Rc<RefCell<dyn MessageSender>>,
}
impl MessageSenderWrapper {
    pub fn push_order_message(&self, message: &OrderMessage) {
        self.inner.borrow_mut().push_order_message(message).unwrap()
    }
    pub fn push_trade_message(&self, message: &Trade) {
        self.inner.borrow_mut().push_trade_message(message).unwrap()
    }
}

struct BalanceManagerWrapper {
    inner: Rc<RefCell<BalanceManager>>,
}

impl BalanceManagerWrapper {
    pub fn balance_add(&self, user_id: u32, balance_type: BalanceType, asset: &str, amount: &Decimal) {
        self.inner.borrow_mut().add(user_id, balance_type, asset, amount);
    }
    pub fn balance_get(&self, user_id: u32, balance_type: BalanceType, asset: &str) -> Decimal {
        self.inner.borrow_mut().get(user_id, balance_type, asset)
    }
    pub fn balance_sub(&self, user_id: u32, balance_type: BalanceType, asset: &str, amount: &Decimal) {
        self.inner.borrow_mut().sub(user_id, balance_type, asset, amount);
    }
    pub fn balance_freeze(&self, user_id: u32, asset: &str, amount: &Decimal) {
        self.inner.borrow_mut().freeze(user_id, asset, amount)
    }
    pub fn balance_unfreeze(&self, user_id: u32, asset: &str, amount: &Decimal) {
        self.inner.borrow_mut().unfreeze(user_id, asset, amount)
    }
}
// TODO: is it ok to match with oneself's order?
// TODO: precision
impl Market {
    pub fn new(
        market_conf: &config::Market,
        balance_manager: Rc<RefCell<BalanceManager>>,
        sequencer: Rc<RefCell<Sequencer>>,
        history_writer: Rc<RefCell<dyn HistoryWriter>>,
        message_sender: Rc<RefCell<dyn MessageSender>>,
    ) -> Result<Market> {
        if !asset_exist(&market_conf.quote.name) || !asset_exist(&market_conf.base.name) {
            return simple_err!("invalid assert name {} {}", market_conf.quote.name, market_conf.base.name);
        }
        if market_conf.base.prec + market_conf.quote.prec > asset_prec(&market_conf.quote.name)
            || market_conf.base.prec + market_conf.fee_prec > asset_prec(&market_conf.base.name)
            || market_conf.quote.prec + market_conf.fee_prec > asset_prec(&market_conf.quote.name)
        {
            return simple_err!("invalid precision");
        }

        let market = Market {
            name: Box::leak(market_conf.name.clone().into_boxed_str()),
            base: market_conf.base.name.clone(),
            quote: market_conf.quote.name.clone(),
            base_prec: market_conf.base.prec,
            quote_prec: market_conf.quote.prec,
            fee_prec: market_conf.fee_prec,
            min_amount: market_conf.min_amount,
            sequencer,
            orders: BTreeMap::new(),
            users: BTreeMap::new(),
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            balance_manager: BalanceManagerWrapper { inner: balance_manager },
            history_writer,
            message_sender: MessageSenderWrapper { inner: message_sender },
        };
        Ok(market)
    }
    pub fn reset(&mut self) {
        self.bids.clear();
        self.asks.clear();
        self.users.clear();
        self.orders.clear();
    }
    pub fn freeze_balance(&self, order: &Order) {
        let asset = if is_order_ask(order) { &self.base } else { &self.quote };

        self.balance_manager.balance_freeze(order.user, asset, &order.freeze);
    }
    pub fn unfreeze_balance(&self, order: &Order) {
        debug_assert!(order.left.is_sign_positive());
        if order.left.is_zero() {
            return;
        }
        let asset = if is_order_ask(&order) { &self.base } else { &self.quote };
        self.balance_manager.balance_unfreeze(order.user, asset, &order.freeze);
    }
    pub fn insert_order(&mut self, order_rc: OrderRc) -> Order {
        let mut order = order_rc.borrow_mut();
        if order.side == OrderSide::ASK {
            order.freeze = order.left;
        } else {
            order.freeze = order.left * order.price;
        }
        debug_assert_eq!(order.type_, OrderType::LIMIT);
        debug_assert!(!self.orders.contains_key(&order.id));
        //println!("order insert {}", &order.id);
        self.orders.insert(order.id.clone(), order_rc.clone());
        let user_map = self.users.entry(order.user).or_insert_with(BTreeMap::new);
        debug_assert!(!user_map.contains_key(&order.id));
        user_map.insert(order.id.clone(), order_rc.clone());
        if order.side == OrderSide::ASK {
            let key = order.get_ask_key();
            debug_assert!(!self.asks.contains_key(&key));
            self.asks.insert(key, order_rc.clone());
        } else {
            let key = order.get_bid_key();
            debug_assert!(!self.bids.contains_key(&key));
            self.bids.insert(key, order_rc.clone());
        }
        *order
    }

    fn order_finish(&mut self, real: bool, order: &Order) {
        if order.side == OrderSide::ASK {
            let key = &order.get_ask_key();
            debug_assert!(self.asks.contains_key(key));
            self.asks.remove(key);
        } else {
            let key = &order.get_bid_key();
            debug_assert!(self.bids.contains_key(key));
            self.bids.remove(key);
        }
        self.unfreeze_balance(&order);
        debug_assert!(self.orders.contains_key(&order.id));
        //println!("order finish {}", &order.id);
        self.orders.remove(&order.id);
        let user_map = self.users.get_mut(&order.user).unwrap();
        debug_assert!(user_map.contains_key(&order.id));
        user_map.remove(&order.id);

        if real {
            // TODO need this if??
            if order.finished_base.is_sign_positive() {
                self.history_writer.borrow_mut().append_order_history(&order);
            }
            let order_message = OrderMessage {
                event: OrderEventType::FINISH,
                order: *order,
                base: self.base.clone(),
                quote: self.quote.clone(),
            };
            self.message_sender.push_order_message(&order_message);
        }
    }

    // the last parameter `quote_limit`, is only used for market bid order,
    // it indicates the `quote` balance of the user,
    // so the sum of all the trades' quote amount cannot exceed this value
    pub fn execute_order(&mut self, real: bool, taker: OrderRc, quote_limit: &Decimal) {
        let taker_is_ask = taker.borrow_mut().side == OrderSide::ASK;
        let taker_is_bid = !taker_is_ask;
        let maker_is_bid = taker_is_ask;
        let maker_is_ask = !maker_is_bid;
        let is_limit_order = taker.borrow_mut().type_ == OrderType::LIMIT;
        let is_market_order = !is_limit_order;
        //let mut quote_available = *quote_limit;
        let mut quote_sum = Decimal::zero();

        let mut finished_orders = Vec::new();

        let counter_orders: Box<dyn Iterator<Item = &mut OrderRc>> = if maker_is_bid {
            Box::new(self.bids.values_mut())
        } else {
            Box::new(self.asks.values_mut())
        };
        for maker in counter_orders {
            let taker_mut = taker.borrow_mut();
            let maker_mut = maker.borrow_mut();
            if taker_mut.left.is_zero() {
                break;
            }
            let (ask_fee_rate, bid_fee_rate) = if taker_is_ask {
                (taker_mut.taker_fee, maker_mut.maker_fee)
            } else {
                (maker_mut.maker_fee, taker_mut.taker_fee)
            };
            let price = maker_mut.price;
            let (mut ask_order, mut bid_order) = if taker_is_ask {
                (taker_mut, maker_mut)
            } else {
                (maker_mut, taker_mut)
            };
            if is_limit_order && ask_order.price.gt(&bid_order.price) {
                break;
            }
            let traded_base_amount = min(ask_order.left, bid_order.left);
            let traded_quote_amount = price * traded_base_amount;

            quote_sum += traded_quote_amount;
            if taker_is_bid && is_market_order {
                if quote_sum.gt(quote_limit) {
                    // Now user has not enough balance, stop here.
                    // Notice: another approach here is to divide left quote by price to get a base amount
                    // to be traded, then all `quote_limit` will be consumed.
                    // But division is prone to bugs in financial decimal calculation,
                    // so we will not adapt tis method.
                    break;
                }
            }

            let ask_fee = traded_quote_amount * ask_fee_rate;
            let bid_fee = traded_base_amount * bid_fee_rate;

            let timestamp = utils::current_timestamp();
            ask_order.update_time = timestamp;
            bid_order.update_time = timestamp;

            if real {
                // emit the trade
                let trade_id = self.sequencer.borrow_mut().next_trade_id();
                let trade = types::Trade {
                    id: trade_id,
                    timestamp: utils::current_timestamp(),
                    market: self.name.to_string(),
                    base: self.base.clone(),
                    quote: self.quote.clone(),
                    price,
                    amount: traded_base_amount,
                    quote_amount: traded_quote_amount,
                    ask_user_id: ask_order.user,
                    ask_order_id: ask_order.id,
                    ask_role: if taker_is_ask { MarketRole::TAKER } else { MarketRole::MAKER },
                    ask_fee,
                    bid_user_id: bid_order.user,
                    bid_order_id: bid_order.id,
                    bid_role: if taker_is_ask { MarketRole::MAKER } else { MarketRole::TAKER },
                    bid_fee,
                };
                self.history_writer.borrow_mut().append_trade_history(&trade);
                self.message_sender.push_trade_message(&trade);
            }
            ask_order.left -= traded_base_amount;
            bid_order.left -= traded_base_amount;
            ask_order.finished_base += traded_base_amount;
            bid_order.finished_base += traded_base_amount;
            ask_order.finished_quote += traded_quote_amount;
            bid_order.finished_quote += traded_quote_amount;
            ask_order.finished_fee += ask_fee;
            bid_order.finished_fee += bid_fee;

            // TODO: change balance should emit a balance update history/event
            // handle maker balance
            let _balance_type = if maker_is_bid {
                BalanceType::FREEZE
            } else {
                BalanceType::AVAILABLE
            };
            // handle base
            self.balance_manager
                .balance_add(bid_order.user, BalanceType::AVAILABLE, &self.base, &traded_base_amount);
            self.balance_manager.balance_sub(
                ask_order.user,
                if maker_is_ask {
                    BalanceType::FREEZE
                } else {
                    BalanceType::AVAILABLE
                },
                &self.base,
                &traded_base_amount,
            );
            // handle quote
            self.balance_manager
                .balance_add(ask_order.user, BalanceType::AVAILABLE, &self.quote, &traded_quote_amount);
            self.balance_manager.balance_sub(
                bid_order.user,
                if maker_is_bid {
                    BalanceType::FREEZE
                } else {
                    BalanceType::AVAILABLE
                },
                &self.quote,
                &traded_quote_amount,
            );

            if ask_fee.is_sign_positive() {
                self.balance_manager
                    .balance_sub(ask_order.user, BalanceType::AVAILABLE, &self.quote, &ask_fee);
            }
            if bid_fee.is_sign_positive() {
                self.balance_manager
                    .balance_sub(bid_order.user, BalanceType::AVAILABLE, &self.base, &bid_fee);
            }

            let (mut _taker_mut, mut maker_mut) = if taker_is_ask {
                (ask_order, bid_order)
            } else {
                (bid_order, ask_order)
            };
            maker_mut.freeze -= if maker_is_bid { traded_quote_amount } else { traded_base_amount };

            let maker_finished = maker_mut.left.is_zero();
            if maker_finished {
                finished_orders.push(maker_mut.clone());
            }
            // When maker_finished, `order_finish` will send message.
            // So we don't need to send the finish message here.
            if real && !maker_finished {
                let order_message = message::OrderMessage {
                    event: OrderEventType::UPDATE,
                    order: *maker_mut,
                    base: self.base.clone(),
                    quote: self.quote.clone(),
                };
                self.message_sender.push_order_message(&order_message);
            }
        }

        for item in finished_orders.iter() {
            self.order_finish(real, item);
        }
    }

    pub fn put_order(&mut self, real: bool, order_input: &OrderInput) -> Result<Order> {
        if order_input.amount.lt(&self.min_amount) {
            return simple_err!("invalid amount");
        }
        if order_input.type_ == OrderType::MARKET {
            if !order_input.price.is_zero() {
                return simple_err!("market order should not have a price");
            }
            if order_input.side == OrderSide::ASK && self.bids.is_empty() || order_input.side == OrderSide::BID && self.asks.is_empty() {
                return simple_err!("no counter orders");
            }
        } else {
            if order_input.price.is_zero() {
                return simple_err!("invalid price for limit order");
            }
        }
        if order_input.side == OrderSide::ASK {
            if self
                .balance_manager
                .balance_get(order_input.user_id, BalanceType::AVAILABLE, &self.base)
                .lt(&order_input.amount)
            {
                return simple_err!("balance not enough");
            }
        } else {
            let balance = self
                .balance_manager
                .balance_get(order_input.user_id, BalanceType::AVAILABLE, &self.quote);

            if order_input.type_ == OrderType::LIMIT {
                if balance.lt(&(order_input.amount * order_input.price)) {
                    return simple_err!(
                        "balance not enough: balance({}) < amount({}) * price({})",
                        &balance,
                        &order_input.amount,
                        &order_input.price
                    );
                }
            } else {
                // We have already checked that counter order book is not empty,
                // so `unwrap` here is safe.
                // Here we only make a minimum balance check against the top of the counter order book.
                // After the check, balance may still be not enough, then the left part of the order
                // will be marked as `canceled(finished)`.
                let top_counter_order_price = self.asks.values().next().unwrap().borrow_mut().price;
                if balance.lt(&(order_input.amount * top_counter_order_price)) {
                    return simple_err!("balance not enough");
                }
            }
        }
        let quote_limit = if order_input.type_ == OrderType::MARKET && order_input.side == OrderSide::BID {
            self.balance_manager
                .balance_get(order_input.user_id, BalanceType::AVAILABLE, &self.quote)
        } else {
            // not used
            Decimal::zero()
        };

        let t = utils::current_timestamp();
        let order_rc = Rc::new(RefCell::new(Order {
            id: self.sequencer.borrow_mut().next_order_id(),
            type_: order_input.type_,
            side: order_input.side,
            create_time: t,
            update_time: t,
            market: &self.name,
            user: order_input.user_id,
            price: order_input.price,
            amount: order_input.amount,
            taker_fee: order_input.taker_fee,
            maker_fee: order_input.maker_fee,
            left: order_input.amount,
            freeze: Decimal::zero(),
            finished_base: Decimal::zero(),
            finished_quote: Decimal::zero(),
            finished_fee: Decimal::zero(),
        }));
        self.execute_order(real, order_rc.clone(), &quote_limit);
        let mut order = *order_rc.borrow_mut();
        if order.type_ == OrderType::LIMIT && !order.left.is_zero() {
            if real {
                let order_message = OrderMessage {
                    event: OrderEventType::PUT,
                    order,
                    base: self.base.clone(),
                    quote: self.quote.clone(),
                };
                self.message_sender.push_order_message(&order_message);
            }
            order = self.insert_order(order_rc);
            self.freeze_balance(&order);
        } else {
            if real {
                self.history_writer.borrow_mut().append_order_history(&order);
                let order_message = OrderMessage {
                    event: OrderEventType::FINISH,
                    order,
                    base: self.base.clone(),
                    quote: self.quote.clone(),
                };
                self.message_sender.push_order_message(&order_message);
            }
        }
        Ok(order)
    }
    pub fn cancel(&mut self, real: bool, order_id: u64) -> Order {
        let order = self.orders.get(&order_id).unwrap();
        let order_struct = *order.borrow_mut();
        self.order_finish(real, &order_struct);
        order_struct
    }
    pub fn get(&self, order_id: u64) -> Option<Order> {
        self.orders.get(&order_id).map(|o| *o.borrow_mut())
    }
    pub fn get_order_of_user(&self, user_id: u32) -> Vec<Order> {
        self.users
            .get(&user_id)
            .unwrap_or(&BTreeMap::new())
            .values()
            .map(|v| *v.borrow_mut())
            .collect()
    }
    pub fn print(&self) {
        println!("orders:");
        for (k, v) in self.orders.iter() {
            println!("{}, {:?}", k, v.borrow())
        }
    }
    pub fn status(&self) -> MarketStatus {
        MarketStatus {
            name: self.name.to_string(),
            ask_count: self.asks.len(),
            ask_amount: self.asks.values().map(|item| item.borrow_mut().left).sum(),
            bid_count: self.bids.len(),
            bid_amount: self.bids.values().map(|item| item.borrow_mut().left).sum(),
        }
    }
    pub fn depth(&self, limit: usize, interval: &Decimal) -> MarketDepth {
        if interval.is_zero() {
            let id_fn = |order: &Order| -> Decimal { order.price };
            MarketDepth {
                asks: Self::group_ordebook_by_fn(&self.asks, limit, id_fn),
                bids: Self::group_ordebook_by_fn(&self.bids, limit, id_fn),
            }
        } else {
            let ask_group_fn = |order: &Order| -> Decimal { (order.price / interval).ceil() * interval };
            let bid_group_fn = |order: &Order| -> Decimal { (order.price / interval).floor() * interval };
            MarketDepth {
                asks: Self::group_ordebook_by_fn(&self.asks, limit, ask_group_fn),
                bids: Self::group_ordebook_by_fn(&self.bids, limit, bid_group_fn),
            }
        }
    }

    fn group_ordebook_by_fn<K, F>(orderbook: &BTreeMap<K, OrderRc>, limit: usize, f: F) -> Vec<PriceInfo>
    where
        F: Fn(&Order) -> Decimal,
    {
        orderbook
            .values()
            .group_by(|order_rc| -> Decimal { f(&order_rc.borrow_mut()) })
            .into_iter()
            .take(limit)
            .map(|(price, group)| PriceInfo {
                price,
                amount: group.map(|order_rc| order_rc.borrow_mut().left).sum(),
            })
            .collect::<Vec<PriceInfo>>()
    }
}

pub struct MarketStatus {
    pub name: String,
    pub ask_count: usize,
    pub ask_amount: Decimal,
    pub bid_count: usize,
    pub bid_amount: Decimal,
}

pub struct PriceInfo {
    pub price: Decimal,
    pub amount: Decimal,
}
pub struct MarketDepth {
    pub asks: Vec<PriceInfo>,
    pub bids: Vec<PriceInfo>,
}

pub struct OrderInput {
    pub user_id: u32,
    pub side: OrderSide,
    pub type_: OrderType,
    pub amount: Decimal,
    pub price: Decimal,
    pub taker_fee: Decimal, // FIXME fee should be determined inside engine rather than take from input
    pub maker_fee: Decimal,
    pub market: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct BalanceHistoryFromTrade {
    pub market: String,
    pub order_id: u64,
    pub price: Decimal,
    pub amount: Decimal,
}

#[derive(Serialize, Deserialize, Debug)]
struct BalanceHistoryFromFee {
    pub market: String,
    pub order_id: u64,
    pub price: Decimal,
    pub amount: Decimal,
    pub fee_rate: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::AssetManager;
    use crate::history::DummyHistoryWriter;
    use crate::message::DummyMessageSender;
    use rust_decimal_macros::*;

    fn get_simple_market_config() -> config::Market {
        config::Market {
            name: String::from("eth/btc"),
            base: config::MarketUnit { name: eth(), prec: 6 },
            quote: config::MarketUnit { name: btc(), prec: 4 },
            fee_prec: 3,
            min_amount: dec!(0.01),
        }
    }
    fn get_simple_asset_config() -> Vec<config::Asset> {
        vec![
            config::Asset {
                name: btc(),
                prec_save: 8,
                prec_show: 8,
            },
            config::Asset {
                name: eth(),
                prec_show: 8,
                prec_save: 8,
            },
        ]
    }
    fn btc() -> String {
        String::from("BTC")
    }
    fn eth() -> String {
        String::from("ETH")
    }
    fn get_simple_asset_manager() -> AssetManager {
        AssetManager::new(&get_simple_asset_config()).unwrap()
    }
    fn get_simple_balance_manager() -> BalanceManager {
        BalanceManager::new(&get_simple_asset_config()).unwrap()
    }
    fn init_balance(balance_manager: &mut BalanceManager) {
        balance_manager.add(101, BalanceType::AVAILABLE, &btc(), &dec!(300));
        balance_manager.add(102, BalanceType::AVAILABLE, &btc(), &dec!(300));
        balance_manager.add(101, BalanceType::AVAILABLE, &eth(), &dec!(1000));
        balance_manager.add(102, BalanceType::AVAILABLE, &eth(), &dec!(1000));
    }

    #[test]
    fn test_market_taker_is_bid() {
        //let mut market = get_simple_market_with_data();
        let mut balance_manager = get_simple_balance_manager();
        init_balance(&mut balance_manager);
        let sequencer = Rc::new(RefCell::new(Sequencer::default()));
        let balance_manager_rc = Rc::new(RefCell::new(balance_manager));
        let ask_user_id = 101;
        let mut market = Market::new(
            &get_simple_market_config(),
            balance_manager_rc.clone(),
            sequencer,
            Rc::new(RefCell::new(DummyHistoryWriter)),
            Rc::new(RefCell::new(DummyMessageSender)),
        )
        .unwrap();
        let ask_order_input = OrderInput {
            user_id: ask_user_id,
            side: OrderSide::ASK,
            type_: OrderType::LIMIT,
            amount: dec!(20.0),
            price: dec!(0.1),
            taker_fee: dec!(0.001),
            maker_fee: dec!(0.001),
            market: market.name.to_string(),
        };
        let ask_order = market.put_order(false, &ask_order_input).unwrap();
        assert_eq!(ask_order.id, 1);
        assert_eq!(ask_order.left, dec!(20.0));

        let bid_user_id = 102;
        let bid_order_input = OrderInput {
            user_id: bid_user_id,
            side: OrderSide::BID,
            type_: OrderType::MARKET,
            amount: dec!(10.0),
            price: dec!(0),
            taker_fee: dec!(0.001),
            maker_fee: dec!(0.001),
            market: market.name.to_string(),
        };
        let bid_order = market.put_order(false, &bid_order_input).unwrap();
        // trade: price: 0.10 amount: 10
        assert_eq!(bid_order.id, 2);
        assert_eq!(bid_order.left, dec!(0));
        assert_eq!(bid_order.finished_quote, dec!(1));
        assert_eq!(bid_order.finished_base, dec!(10));
        assert_eq!(bid_order.finished_fee, dec!(0.01));

        //market.print();

        let ask_order = market.get(ask_order.id).unwrap();
        assert_eq!(ask_order.left, dec!(10));
        assert_eq!(ask_order.finished_quote, dec!(1));
        assert_eq!(ask_order.finished_base, dec!(10));
        assert_eq!(ask_order.finished_fee, dec!(0.001));

        // original balance: btc 300, eth 1000
        let balance_manager = balance_manager_rc.borrow_mut();
        assert_eq!(balance_manager.get(ask_user_id, BalanceType::AVAILABLE, &eth()), dec!(980));
        assert_eq!(balance_manager.get(ask_user_id, BalanceType::FREEZE, &eth()), dec!(10));

        assert_eq!(balance_manager.get(ask_user_id, BalanceType::AVAILABLE, &btc()), dec!(300.999));
        assert_eq!(balance_manager.get(ask_user_id, BalanceType::FREEZE, &btc()), dec!(0));

        assert_eq!(balance_manager.get(bid_user_id, BalanceType::AVAILABLE, &eth()), dec!(1009.99));
        assert_eq!(balance_manager.get(bid_user_id, BalanceType::FREEZE, &eth()), dec!(0));

        assert_eq!(balance_manager.get(bid_user_id, BalanceType::AVAILABLE, &btc()), dec!(299));
        assert_eq!(balance_manager.get(bid_user_id, BalanceType::FREEZE, &btc()), dec!(0));
    }
}
