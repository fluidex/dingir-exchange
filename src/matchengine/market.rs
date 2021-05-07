#![allow(clippy::if_same_then_else)]
use crate::asset::{BalanceManager, BalanceType};
use crate::config;
use crate::history::HistoryWriter;
use crate::message::{MessageManager, OrderMessage};
use crate::sequencer::Sequencer;
use crate::types::{self, MarketRole, OrderEventType};
use crate::utils;

use std::cmp::{min, Ordering};
use std::collections::BTreeMap;
use std::iter::Iterator;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use itertools::Itertools;
use rust_decimal::prelude::Zero;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub use types::{OrderSide, OrderType};

#[derive(PartialEq, PartialOrd, Eq, Ord)]
pub struct MarketKeyAsk {
    pub order_price: Decimal,
    pub order_id: u64,
}
pub type MarketKey = MarketKeyAsk;

#[derive(PartialEq, Eq)]
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

impl PartialOrd for MarketKeyBid {
    fn partial_cmp(&self, other: &MarketKeyBid) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub enum MarketString {
    Left(&'static str),
    Right(String),
}

impl From<&'static str> for MarketString {
    fn from(str: &'static str) -> Self {
        MarketString::Left(str)
    }
}

impl std::ops::Deref for MarketString {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        match self {
            MarketString::Left(str) => *str,
            MarketString::Right(stri) => stri.as_str(),
        }
    }
}

impl serde::ser::Serialize for MarketString {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            MarketString::Left(str) => serializer.serialize_str(*str),
            MarketString::Right(stri) => serializer.serialize_str(stri.as_str()),
        }
    }
}

impl<'de> serde::de::Deserialize<'de> for MarketString {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(MarketString::Right(s))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub market: MarketString,
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
    pub remain: Decimal,
    pub frozen: Decimal,
    pub finished_base: Decimal,
    pub finished_quote: Decimal,
    pub finished_fee: Decimal,
}

/*
fn de_market_string<'de, D: serde::de::Deserializer<'de>>(_deserializer: D) -> Result<&'static str, D::Error> {
    Ok("Test")
}
*/

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VerboseOrderState {
    price: Decimal,
    amount: Decimal,
    finished_base: Decimal,
    finished_quote: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VerboseBalanceState {
    pub bid_user_base: Decimal,
    pub bid_user_quote: Decimal,
    pub ask_user_base: Decimal,
    pub ask_user_quote: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VerboseTradeState {
    // emit all the related state
    pub ask_order_state: VerboseOrderState,
    pub bid_order_state: VerboseOrderState,
    pub balance: VerboseBalanceState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trade {
    pub id: u64,
    pub timestamp: f64, // unix epoch timestamp,
    pub market: String,
    pub base: String,
    pub quote: String,
    pub price: Decimal,
    pub amount: Decimal,
    pub quote_amount: Decimal,

    pub ask_user_id: u32,
    pub ask_order_id: u64,
    pub ask_role: MarketRole, // take/make
    pub ask_fee: Decimal,

    pub bid_user_id: u32,
    pub bid_order_id: u64,
    pub bid_role: MarketRole,
    pub bid_fee: Decimal,

    #[cfg(feature = "emit_state_diff")]
    pub state_before: VerboseTradeState,
    #[cfg(feature = "emit_state_diff")]
    pub state_after: VerboseTradeState,
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

#[derive(Clone, Debug)]
pub struct OrderRc(Arc<RwLock<Order>>);

/*
    simulate behavior like RefCell, the syncing is ensured by locking in higher rank
    here we use RwLock only for avoiding unsafe tag, we can just use raw pointer
    casted from ARc rather than RwLock here if we do not care about unsafe
*/
impl OrderRc {
    fn new(order: Order) -> Self {
        OrderRc(Arc::new(RwLock::new(order)))
    }

    pub(super) fn borrow(&self) -> RwLockReadGuard<'_, Order> {
        self.0.try_read().expect("Lock for parent entry ensure it")
    }

    pub(super) fn borrow_mut(&mut self) -> RwLockWriteGuard<'_, Order> {
        self.0.try_write().expect("Lock for parent entry ensure it")
    }

    fn deep(&self) -> Order {
        self.borrow().clone()
    }
}

pub fn is_order_ask(order: &Order) -> bool {
    order.side == OrderSide::ASK
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

    pub trade_count: u64,

    // other options
    pub disable_self_trade: bool,
}

pub trait PersistExector {
    fn real_persist(&self) -> bool {
        true
    }
    fn put_order(&mut self, order: &Order, at_step: OrderEventType);
    fn put_trade(&mut self, trade: &Trade);
}

impl PersistExector for Box<dyn PersistExector + '_> {
    fn put_order(&mut self, order: &Order, at_step: OrderEventType) {
        self.as_mut().put_order(order, at_step)
    }
    fn put_trade(&mut self, trade: &Trade) {
        self.as_mut().put_trade(trade)
    }
}

pub(super) struct DummyPersistor {
    pub(super) real_persist: bool,
}
impl DummyPersistor {
    pub(super) fn new(real_persist: bool) -> Self {
        Self { real_persist }
    }
}
/*
impl PersistExector for &mut DummyPersistor {
    fn real_persist(&self) -> bool {
        self.real_persist
    }
    fn put_order(&mut self, _order: &Order, _as_step: OrderEventType) {}
    fn put_trade(&mut self, _trade: &Trade) {}
}
*/
impl PersistExector for DummyPersistor {
    fn real_persist(&self) -> bool {
        self.real_persist
    }
    fn put_order(&mut self, _order: &Order, _as_step: OrderEventType) {}
    fn put_trade(&mut self, _trade: &Trade) {}
}

pub(super) struct MessengerAsPersistor<'a, T>(&'a mut T, (String, String));

impl<T: MessageManager> PersistExector for MessengerAsPersistor<'_, T> {
    fn put_order(&mut self, order: &Order, at_step: OrderEventType) {
        self.0.push_order_message(&OrderMessage {
            event: at_step,
            order: order.clone(),
            base: self.1 .0.clone(),
            quote: self.1 .1.clone(),
        });
    }
    fn put_trade(&mut self, trade: &Trade) {
        self.0.push_trade_message(trade);
    }
}

impl<T1: PersistExector, T2: PersistExector> PersistExector for (T1, T2) {
    fn real_persist(&self) -> bool {
        self.0.real_persist() || self.1.real_persist()
    }
    fn put_order(&mut self, order: &Order, at_step: OrderEventType) {
        self.0.put_order(order, at_step);
        self.1.put_order(order, at_step);
    }
    fn put_trade(&mut self, trade: &Trade) {
        self.0.put_trade(trade);
        self.1.put_trade(trade);
    }
}

pub(super) struct DBAsPersistor<'a, T>(&'a mut T);

impl<T: HistoryWriter> PersistExector for DBAsPersistor<'_, T> {
    fn put_order(&mut self, order: &Order, at_step: OrderEventType) {
        //only persist on finish
        match at_step {
            OrderEventType::FINISH => self.0.append_order_history(order),
            OrderEventType::EXPIRED => self.0.append_expired_order_history(order),
            OrderEventType::PUT => (),
            _ => (),
        }
    }
    fn put_trade(&mut self, trade: &Trade) {
        self.0.append_pair_user_trade(trade);
    }
}

pub(super) fn persistor_for_message<T: MessageManager>(messenger: &mut T, tag: (String, String)) -> MessengerAsPersistor<'_, T> {
    MessengerAsPersistor(messenger, tag)
}

pub(super) fn persistor_for_db<T: HistoryWriter>(history_writer: &mut T) -> DBAsPersistor<'_, T> {
    DBAsPersistor(history_writer)
}

pub struct BalanceManagerWrapper<'a> {
    inner: &'a mut BalanceManager,
}

impl<'a> From<&'a mut BalanceManager> for BalanceManagerWrapper<'a> {
    fn from(origin: &'a mut BalanceManager) -> Self {
        BalanceManagerWrapper { inner: origin }
    }
}

impl BalanceManagerWrapper<'_> {
    pub fn balance_add(&mut self, user_id: u32, balance_type: BalanceType, asset: &str, amount: &Decimal) {
        self.inner.add(user_id, balance_type, asset, amount);
    }
    pub fn balance_get(&mut self, user_id: u32, balance_type: BalanceType, asset: &str) -> Decimal {
        self.inner.get(user_id, balance_type, asset)
    }
    pub fn balance_total(&mut self, user_id: u32, asset: &str) -> Decimal {
        self.inner.get(user_id, BalanceType::FREEZE, asset) + self.inner.get(user_id, BalanceType::AVAILABLE, asset)
    }
    pub fn balance_sub(&mut self, user_id: u32, balance_type: BalanceType, asset: &str, amount: &Decimal) {
        self.inner.sub(user_id, balance_type, asset, amount);
    }
    pub fn balance_frozen(&mut self, user_id: u32, asset: &str, amount: &Decimal) {
        self.inner.frozen(user_id, asset, amount)
    }
    pub fn balance_unfrozen(&mut self, user_id: u32, asset: &str, amount: &Decimal) {
        self.inner.unfrozen(user_id, asset, amount)
    }
    pub fn asset_prec(&mut self, asset: &str) -> u32 {
        self.inner.asset_manager.asset_prec(asset)
    }
}

const MAP_INIT_CAPACITY: usize = 1024;

// TODO: is it ok to match with oneself's order?
// TODO: precision
impl Market {
    pub fn new(market_conf: &config::Market, balance_manager: &BalanceManager) -> Result<Market> {
        let asset_exist = |asset: &str| -> bool { balance_manager.asset_manager.asset_exist(asset) };
        let asset_prec = |asset: &str| -> u32 { balance_manager.asset_manager.asset_prec(asset) };
        if !asset_exist(&market_conf.quote.symbol) || !asset_exist(&market_conf.base.symbol) {
            return Err(anyhow!(
                "invalid assert name {} {}",
                market_conf.quote.symbol,
                market_conf.base.symbol
            ));
        }

        if market_conf.base.prec + market_conf.quote.prec > asset_prec(&market_conf.quote.symbol)
            || market_conf.base.prec + market_conf.fee_prec > asset_prec(&market_conf.base.symbol)
            || market_conf.quote.prec + market_conf.fee_prec > asset_prec(&market_conf.quote.symbol)
        {
            return Err(anyhow!("invalid precision"));
        }

        let market = Market {
            name: Box::leak(market_conf.name.clone().into_boxed_str()),
            base: market_conf.base.symbol.clone(),
            quote: market_conf.quote.symbol.clone(),
            base_prec: market_conf.base.prec,
            quote_prec: market_conf.quote.prec,
            fee_prec: market_conf.fee_prec,
            min_amount: market_conf.min_amount,
            orders: BTreeMap::new(),
            users: BTreeMap::new(),
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            trade_count: 0,
            disable_self_trade: market_conf.disable_self_trade,
        };
        Ok(market)
    }

    pub fn tag(&self) -> (String, String) {
        (self.base.clone(), self.quote.clone())
    }

    pub fn reset(&mut self) {
        log::debug!("market {} reset", self.name);
        self.bids.clear();
        self.asks.clear();
        self.users.clear();
        self.orders.clear();
    }
    pub fn frozen_balance(&self, balance_manager: &mut BalanceManagerWrapper<'_>, order: &Order) {
        let asset = if is_order_ask(order) { &self.base } else { &self.quote };

        balance_manager.balance_frozen(order.user, asset, &order.frozen);
    }
    pub fn unfrozen_balance(&self, balance_manager: &mut BalanceManagerWrapper<'_>, order: &Order) {
        debug_assert!(order.remain.is_sign_positive());
        if order.remain.is_zero() {
            return;
        }
        let asset = if is_order_ask(&order) { &self.base } else { &self.quote };
        balance_manager.balance_unfrozen(order.user, asset, &order.frozen);
    }
    pub fn insert_order(&mut self, mut order: Order) -> Order {
        if order.side == OrderSide::ASK {
            order.frozen = order.remain;
        } else {
            order.frozen = order.remain * order.price;
        }
        debug_assert_eq!(order.type_, OrderType::LIMIT);
        debug_assert!(!self.orders.contains_key(&order.id));
        // log::debug!("order insert {}", &order.id);
        let order_rc = OrderRc::new(order);
        let order = order_rc.borrow();
        self.orders.insert(order.id, order_rc.clone());
        let user_map = self.users.entry(order.user).or_insert_with(BTreeMap::new);
        debug_assert!(!user_map.contains_key(&order.id));
        user_map.insert(order.id, order_rc.clone());
        if order.side == OrderSide::ASK {
            let key = order.get_ask_key();
            debug_assert!(!self.asks.contains_key(&key));
            self.asks.insert(key, order_rc.clone());
        } else {
            let key = order.get_bid_key();
            debug_assert!(!self.bids.contains_key(&key));
            self.bids.insert(key, order_rc.clone());
        }
        order_rc.deep()
    }

    fn order_finish(&mut self, balance_manager: &mut BalanceManagerWrapper<'_>, persistor: &mut impl PersistExector, order: &Order) {
        if order.side == OrderSide::ASK {
            let key = &order.get_ask_key();
            debug_assert!(self.asks.contains_key(key));
            self.asks.remove(key);
        } else {
            let key = &order.get_bid_key();
            debug_assert!(self.bids.contains_key(key));
            self.bids.remove(key);
        }
        self.unfrozen_balance(balance_manager, order);
        debug_assert!(self.orders.contains_key(&order.id));
        // log::debug!("order finish {}", &order.id);
        self.orders.remove(&order.id);
        let user_map = self.users.get_mut(&order.user).unwrap();
        debug_assert!(user_map.contains_key(&order.id));
        user_map.remove(&order.id);

        persistor.put_order(order, OrderEventType::FINISH);
    }

    // TODO: better naming
    fn get_trade_state(
        ask: &Order,
        bid: &Order,
        balance_manager: &mut BalanceManagerWrapper<'_>,
        base: &str,
        quote: &str,
    ) -> VerboseTradeState {
        let ask_order_state = VerboseOrderState {
            price: ask.price,
            amount: ask.amount,
            finished_base: ask.finished_base,
            finished_quote: ask.finished_quote,
        };
        let bid_order_state = VerboseOrderState {
            price: bid.price,
            amount: bid.amount,
            finished_base: bid.finished_base,
            finished_quote: bid.finished_quote,
        };
        let ask_user_base = balance_manager.balance_total(ask.user, base);
        let ask_user_quote = balance_manager.balance_total(ask.user, quote);
        let bid_user_base = balance_manager.balance_total(bid.user, base);
        let bid_user_quote = balance_manager.balance_total(bid.user, quote);
        VerboseTradeState {
            ask_order_state,
            bid_order_state,
            balance: VerboseBalanceState {
                bid_user_base,
                bid_user_quote,
                ask_user_base,
                ask_user_quote,
            },
        }
    }

    // the last parameter `quote_limit`, is only used for market bid order,
    // it indicates the `quote` balance of the user,
    // so the sum of all the trades' quote amount cannot exceed this value
    fn execute_order(
        &mut self,
        sequencer: &mut Sequencer,
        balance_manager: &mut BalanceManagerWrapper<'_>,
        persistor: &mut impl PersistExector,
        mut taker: Order,
        quote_limit: &Decimal,
    ) -> Order {
        log::debug!("execute_order {:?}", taker);
        let taker_is_ask = taker.side == OrderSide::ASK;
        let taker_is_bid = !taker_is_ask;
        let maker_is_bid = taker_is_ask;
        let maker_is_ask = !maker_is_bid;
        let is_limit_order = taker.type_ == OrderType::LIMIT;
        let is_market_order = !is_limit_order;
        //let mut quote_available = *quote_limit;
        let mut quote_sum = Decimal::zero();

        let mut finished_orders = Vec::new();

        let counter_orders: Box<dyn Iterator<Item = &mut OrderRc>> = if maker_is_bid {
            Box::new(self.bids.values_mut())
        } else {
            Box::new(self.asks.values_mut())
        };

        // TODO: find a more elegant way to handle this
        let mut need_cancel = false;
        for maker_ref in counter_orders {
            let mut maker = maker_ref.borrow_mut();
            if taker.user == maker.user && self.disable_self_trade {
                need_cancel = true;
                break;
            }
            if taker.remain.is_zero() {
                break;
            }
            let (ask_fee_rate, bid_fee_rate) = if taker_is_ask {
                (taker.taker_fee, maker.maker_fee)
            } else {
                (maker.maker_fee, taker.taker_fee)
            };
            let price = maker.price;
            let (ask_order, bid_order) = if taker_is_ask {
                (&mut taker, &mut *maker)
            } else {
                (&mut *maker, &mut taker)
            };
            //let ask_order_id: u64 = ask_order.id;
            //let bid_order_id: u64 = bid_order.id;
            if is_limit_order && ask_order.price.gt(&bid_order.price) {
                break;
            }
            let traded_base_amount = min(ask_order.remain, bid_order.remain);
            let traded_quote_amount = price * traded_base_amount;

            quote_sum += traded_quote_amount;
            if taker_is_bid && is_market_order {
                if quote_sum.gt(quote_limit) {
                    // Now user has not enough balance, stop here.
                    // Notice: another approach here is to divide remain quote by price to get a base amount
                    // to be traded, then all `quote_limit` will be consumed.
                    // But division is prone to bugs in financial decimal calculation,
                    // so we will not adapt tis method.
                    // TODO: maybe another method is to make:
                    // trade_base_amount = round_down(quote_limit - old_quote_sum / price)
                    // so quote_limit will be `almost` fulfilled
                    break;
                }
            }

            let ask_fee = traded_quote_amount * ask_fee_rate;
            let bid_fee = traded_base_amount * bid_fee_rate;

            let timestamp = utils::current_timestamp();
            ask_order.update_time = timestamp;
            bid_order.update_time = timestamp;

            // emit the trade
            let trade_id = sequencer.next_trade_id();
            let trade = Trade {
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
                #[cfg(feature = "emit_state_diff")]
                state_before: Default::default(),
                #[cfg(feature = "emit_state_diff")]
                state_after: Default::default(),
            };
            #[cfg(feature = "emit_state_diff")]
            let state_before = Self::get_trade_state(ask_order, bid_order, balance_manager, &self.base, &self.quote);
            self.trade_count += 1;
            if self.disable_self_trade {
                debug_assert_ne!(trade.ask_user_id, trade.bid_user_id);
            }
            ask_order.remain -= traded_base_amount;
            bid_order.remain -= traded_base_amount;
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
            balance_manager.balance_add(bid_order.user, BalanceType::AVAILABLE, &self.base, &traded_base_amount);
            balance_manager.balance_sub(
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
            balance_manager.balance_add(ask_order.user, BalanceType::AVAILABLE, &self.quote, &traded_quote_amount);
            balance_manager.balance_sub(
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
                balance_manager.balance_sub(ask_order.user, BalanceType::AVAILABLE, &self.quote, &ask_fee);
            }
            if bid_fee.is_sign_positive() {
                balance_manager.balance_sub(bid_order.user, BalanceType::AVAILABLE, &self.base, &bid_fee);
            }
            #[cfg(feature = "emit_state_diff")]
            let state_after = Self::get_trade_state(ask_order, bid_order, balance_manager, &self.base, &self.quote);

            if persistor.real_persist() {
                let trade = Trade {
                    #[cfg(feature = "emit_state_diff")]
                    state_after,
                    #[cfg(feature = "emit_state_diff")]
                    state_before,
                    ..trade
                };
                persistor.put_trade(&trade)
            }
            maker.frozen -= if maker_is_bid { traded_quote_amount } else { traded_base_amount };

            let maker_finished = maker.remain.is_zero();
            if maker_finished {
                finished_orders.push(maker.clone());
            } else {
                // When maker_finished, `order_finish` will send message.
                // So we don't need to send the finish message here.
                persistor.put_order(&maker, OrderEventType::UPDATE);
            }
        }

        for item in finished_orders.iter() {
            self.order_finish(&mut *balance_manager, &mut *persistor, item);
        }

        if need_cancel {
            // now only self trade orders will be canceled here
            persistor.put_order(&taker, OrderEventType::FINISH);
        } else if taker.type_ == OrderType::MARKET {
            // market order can either filled or not
            // if it is filled, `FINISH` is ok
            // if it is not filled, `CANCELED` may be a better choice?
            persistor.put_order(&taker, OrderEventType::FINISH);
        } else {
            // now the order type is limit
            if taker.remain.is_zero() {
                persistor.put_order(&taker, OrderEventType::FINISH);
            } else {
                persistor.put_order(&taker, OrderEventType::PUT);
                // `insert_order` will update the order info
                taker = self.insert_order(taker);
                self.frozen_balance(balance_manager, &taker);
            }
        }

        taker
    }

    pub fn put_order(
        &mut self,
        sequencer: &mut Sequencer,
        mut balance_manager: BalanceManagerWrapper<'_>,
        mut persistor: impl PersistExector,
        order_input: OrderInput,
    ) -> Result<Order> {
        if order_input.amount.lt(&self.min_amount) {
            return Err(anyhow!("invalid amount"));
        }
        // TODO: refactor this
        let base_prec = self.base_prec;
        let quote_prec = self.quote_prec;
        let amount = order_input.amount.round_dp(base_prec);
        let price = order_input.price.round_dp(quote_prec);
        // log::debug!("decimal {} {} {} {} ", self.base, base_prec, self.quote, quote_prec);
        let order_input = OrderInput {
            amount,
            price,
            ..order_input
        };
        if order_input.type_ == OrderType::MARKET {
            if !order_input.price.is_zero() {
                return Err(anyhow!("market order should not have a price"));
            }
            if order_input.side == OrderSide::ASK && self.bids.is_empty() || order_input.side == OrderSide::BID && self.asks.is_empty() {
                return Err(anyhow!("no counter orders"));
            }
        } else if order_input.price.is_zero() {
            return Err(anyhow!("invalid price for limit order"));
        }

        if order_input.side == OrderSide::ASK {
            if balance_manager
                .balance_get(order_input.user_id, BalanceType::AVAILABLE, &self.base)
                .lt(&order_input.amount)
            {
                return Err(anyhow!("balance not enough"));
            }
        } else {
            let balance = balance_manager.balance_get(order_input.user_id, BalanceType::AVAILABLE, &self.quote);

            if order_input.type_ == OrderType::LIMIT {
                if balance.lt(&(order_input.amount * order_input.price)) {
                    return Err(anyhow!(
                        "balance not enough: balance({}) < amount({}) * price({})",
                        &balance,
                        &order_input.amount,
                        &order_input.price
                    ));
                }
            } else {
                // We have already checked that counter order book is not empty,
                // so `unwrap` here is safe.
                // Here we only make a minimum balance check against the top of the counter order book.
                // After the check, balance may still be not enough, then the remain part of the order
                // will be marked as `canceled(finished)`.
                let top_counter_order_price = self.asks.values().next().unwrap().borrow().price;
                if balance.lt(&(order_input.amount * top_counter_order_price)) {
                    return Err(anyhow!("balance not enough"));
                }
            }
        }
        let quote_limit = if order_input.type_ == OrderType::MARKET && order_input.side == OrderSide::BID {
            balance_manager.balance_get(order_input.user_id, BalanceType::AVAILABLE, &self.quote)
        } else {
            // not used
            Decimal::zero()
        };

        let t = utils::current_timestamp();
        let order = Order {
            id: sequencer.next_order_id(),
            type_: order_input.type_,
            side: order_input.side,
            create_time: t,
            update_time: t,
            market: self.name.into(),
            user: order_input.user_id,
            price: order_input.price,
            amount: order_input.amount,
            taker_fee: order_input.taker_fee,
            maker_fee: order_input.maker_fee,
            remain: order_input.amount,
            frozen: Decimal::zero(),
            finished_base: Decimal::zero(),
            finished_quote: Decimal::zero(),
            finished_fee: Decimal::zero(),
        };
        let order = self.execute_order(sequencer, &mut balance_manager, &mut persistor, order, &quote_limit);
        Ok(order)
    }
    pub fn cancel(&mut self, mut balance_manager: BalanceManagerWrapper<'_>, mut persistor: impl PersistExector, order_id: u64) -> Order {
        let order = self.orders.get(&order_id).unwrap();
        let order_struct = order.deep();
        self.order_finish(&mut balance_manager, &mut persistor, &order_struct);
        order_struct
    }
    pub fn cancel_all_for_user(
        &mut self,
        mut balance_manager: BalanceManagerWrapper<'_>,
        mut persistor: impl PersistExector,
        user_id: u32,
    ) -> usize {
        // TODO: can we mutate while iterate?
        let order_ids: Vec<u64> = self.users.get(&user_id).unwrap_or(&BTreeMap::new()).keys().copied().collect();
        let total = order_ids.len();
        for order_id in order_ids {
            let order = self.orders.get(&order_id).unwrap();
            let order_struct = order.deep();
            self.order_finish(&mut balance_manager, &mut persistor, &order_struct);
        }
        total
    }
    pub fn get(&self, order_id: u64) -> Option<Order> {
        self.orders.get(&order_id).map(OrderRc::deep)
    }
    pub fn get_order_of_user(&self, user_id: u32) -> Vec<Order> {
        self.users
            .get(&user_id)
            .unwrap_or(&BTreeMap::new())
            .values()
            .map(OrderRc::deep)
            .collect()
    }
    pub fn print(&self) {
        log::info!("orders:");
        for (k, v) in self.orders.iter() {
            log::info!("{}, {:?}", k, v.borrow())
        }
    }
    pub fn status(&self) -> MarketStatus {
        MarketStatus {
            name: self.name.to_string(),
            ask_count: self.asks.len(),
            ask_amount: self.asks.values().map(|item| item.borrow().remain).sum(),
            bid_count: self.bids.len(),
            bid_amount: self.bids.values().map(|item| item.borrow().remain).sum(),
            trade_count: self.trade_count,
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
            .group_by(|order_rc| -> Decimal { f(&order_rc.borrow()) })
            .into_iter()
            .take(limit)
            .map(|(price, group)| PriceInfo {
                price,
                amount: group.map(|order_rc| order_rc.borrow().remain).sum(),
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
    pub trade_count: u64,
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
    use crate::matchengine::mock;
    use mock::*;
    use rust_decimal_macros::*;

    #[cfg(feature = "emit_state_diff")]
    #[test]
    fn test_multi_orders() {
        use crate::asset::BalanceUpdateController;
        use crate::matchengine::market::{Market, OrderInput};
        use crate::types::{OrderSide, OrderType};
        use rand::Rng;
        use rust_decimal::prelude::FromPrimitive;

        let only_int = true;
        let mut persistor = get_mocking_persistor();
        let mut update_controller = BalanceUpdateController::new();
        let balance_manager = &mut get_simple_balance_manager(get_simple_asset_config(if only_int { 0 } else { 6 }));
        let uid0 = 0;
        let uid1 = 1;
        let mut update_balance_fn = |seq_id, user_id, asset: &str, amount| {
            update_controller
                .update_user_balance(
                    balance_manager,
                    persistor.persistor_for_balance(true),
                    user_id,
                    asset,
                    "deposit".to_owned(),
                    seq_id,
                    amount,
                    serde_json::Value::default(),
                )
                .unwrap();
        };
        update_balance_fn(0, uid0, &usdt(), dec!(1_000_000));
        update_balance_fn(1, uid0, &eth(), dec!(1_000_000));
        update_balance_fn(2, uid1, &usdt(), dec!(1_000_000));
        update_balance_fn(3, uid1, &eth(), dec!(1_000_000));

        let sequencer = &mut Sequencer::default();
        let mut market_conf = if only_int {
            mock::get_integer_prec_market_config()
        } else {
            mock::get_simple_market_config()
        };
        market_conf.disable_self_trade = true;
        let mut market = Market::new(&market_conf, balance_manager).unwrap();
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let user_id = if rng.gen::<bool>() { uid0 } else { uid1 };
            let side = if rng.gen::<bool>() { OrderSide::BID } else { OrderSide::ASK };
            let amount = if only_int {
                Decimal::from_i32(rng.gen_range(1..10)).unwrap()
            } else {
                Decimal::from_f64(rng.gen_range(1.0..10.0)).unwrap()
            };
            let price = if only_int {
                Decimal::from_i32(rng.gen_range(120..140)).unwrap()
            } else {
                Decimal::from_f64(rng.gen_range(120.0..140.0)).unwrap()
            };
            let order = OrderInput {
                user_id,
                side,
                type_: OrderType::LIMIT,
                // the matchengine will truncate precision
                // but later we'd better truncate precision outside
                amount,
                price,
                taker_fee: dec!(0),
                maker_fee: dec!(0),
                market: market.name.to_string(),
            };
            market
                .put_order(
                    sequencer,
                    balance_manager.into(),
                    persistor.persistor_for_market(true, (market.base.clone(), market.quote.clone())),
                    order,
                )
                .unwrap();
        }
    }

    #[test]
    fn test_market_taker_is_bid() {
        let balance_manager = &mut get_simple_balance_manager(get_simple_asset_config(8));

        balance_manager.add(101, BalanceType::AVAILABLE, &MockAsset::USDT.symbol(), &dec!(300));
        balance_manager.add(102, BalanceType::AVAILABLE, &MockAsset::USDT.symbol(), &dec!(300));
        balance_manager.add(101, BalanceType::AVAILABLE, &MockAsset::ETH.symbol(), &dec!(1000));
        balance_manager.add(102, BalanceType::AVAILABLE, &MockAsset::ETH.symbol(), &dec!(1000));

        let sequencer = &mut Sequencer::default();
        let mut persistor = MockPersistor::new();
        let ask_user_id = 101;
        let mut market = Market::new(&get_simple_market_config(), balance_manager).unwrap();
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
        let ask_order = market
            .put_order(sequencer, balance_manager.into(), &mut persistor, ask_order_input)
            .unwrap();
        assert_eq!(ask_order.id, 1);
        assert_eq!(ask_order.remain, dec!(20.0));

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
        let bid_order = market
            .put_order(sequencer, balance_manager.into(), &mut persistor, bid_order_input)
            .unwrap();
        // trade: price: 0.10 amount: 10
        assert_eq!(bid_order.id, 2);
        assert_eq!(bid_order.remain, dec!(0));
        assert_eq!(bid_order.finished_quote, dec!(1));
        assert_eq!(bid_order.finished_base, dec!(10));
        assert_eq!(bid_order.finished_fee, dec!(0.01));

        //market.print();

        let ask_order = market.get(ask_order.id).unwrap();
        assert_eq!(ask_order.remain, dec!(10));
        assert_eq!(ask_order.finished_quote, dec!(1));
        assert_eq!(ask_order.finished_base, dec!(10));
        assert_eq!(ask_order.finished_fee, dec!(0.001));

        // original balance: btc 300, eth 1000
        assert_eq!(
            balance_manager.get(ask_user_id, BalanceType::AVAILABLE, &MockAsset::ETH.symbol()),
            dec!(980)
        );
        assert_eq!(
            balance_manager.get(ask_user_id, BalanceType::FREEZE, &MockAsset::ETH.symbol()),
            dec!(10)
        );

        assert_eq!(
            balance_manager.get(ask_user_id, BalanceType::AVAILABLE, &MockAsset::USDT.symbol()),
            dec!(300.999)
        );
        assert_eq!(
            balance_manager.get(ask_user_id, BalanceType::FREEZE, &MockAsset::USDT.symbol()),
            dec!(0)
        );

        assert_eq!(
            balance_manager.get(bid_user_id, BalanceType::AVAILABLE, &MockAsset::ETH.symbol()),
            dec!(1009.99)
        );
        assert_eq!(
            balance_manager.get(bid_user_id, BalanceType::FREEZE, &MockAsset::ETH.symbol()),
            dec!(0)
        );

        assert_eq!(
            balance_manager.get(bid_user_id, BalanceType::AVAILABLE, &MockAsset::USDT.symbol()),
            dec!(299)
        );
        assert_eq!(
            balance_manager.get(bid_user_id, BalanceType::FREEZE, &MockAsset::USDT.symbol()),
            dec!(0)
        );

        //assert_eq!(persistor.orders.len(), 3);
        //assert_eq!(persistor.trades.len(), 1);
    }
}
