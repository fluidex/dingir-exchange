use crate::asset::{BalanceManager, BalanceType};
use crate::history::HistoryWriter;
use crate::message::{MessageSender, OrderMessage};
use crate::types::{self, Deal, MarketRole, OrderEventType};
use crate::utils;
use crate::{config, message};
use anyhow::Result;
use itertools::Itertools;
use rust_decimal::prelude::Zero;
use rust_decimal::Decimal;


use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::cmp::{min, Ordering};
use std::collections::{BTreeMap, HashMap};
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum OrderType {
    LIMIT = 1,
    MARKET = 2,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum OrderSide {
    ASK = 1,
    BID = 2,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Order {
    pub id: u64,
    pub market: &'static str,
    pub source: &'static str,
    #[serde(rename = "type")]
    pub type_0: OrderType, // enum
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
    pub deal_stock: Decimal,
    pub deal_money: Decimal,
    pub deal_fee: Decimal,
}

impl Order {
    pub fn to_ask_key(&self) -> MarketKeyAsk {
        MarketKeyAsk {
            order_price: self.price,
            order_id: self.id,
        }
    }
    pub fn to_bid_key(&self) -> MarketKeyBid {
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

pub struct Sequencer {
    pub order_id_start: u64,
    pub deals_id_start: u64,
    pub operlog_id_start: u64,
}

impl Sequencer {
    pub fn next_order_id(&mut self) -> u64 {
        self.order_id_start += 1;
        self.order_id_start
    }
    pub fn next_deal_id(&mut self) -> u64 {
        self.deals_id_start += 1;
        self.deals_id_start
    }
    pub fn next_operlog_id(&mut self) -> u64 {
        self.operlog_id_start += 1;
        self.operlog_id_start
    }
    pub fn set_operlog_id(&mut self, id: u64) {
        self.operlog_id_start = id;
    }
}

pub struct Market {
    pub name: &'static str,
    pub stock: String,
    pub money: String,
    pub stock_prec: u32,
    pub money_prec: u32,
    pub fee_prec: u32,
    pub min_amount: Decimal,

    pub orders: HashMap<u64, OrderRc>,
    pub users: HashMap<u32, BTreeMap<u64, OrderRc>>,

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
    pub fn push_deal_message(&self, message: &Deal) {
        self.inner.borrow_mut().push_deal_message(message).unwrap()
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

impl Market {
    pub fn new(
        market_conf: &config::Market,
        balance_manager: Rc<RefCell<BalanceManager>>,
        sequencer: Rc<RefCell<Sequencer>>,
        history_writer: Rc<RefCell<dyn HistoryWriter>>,
        message_sender: Rc<RefCell<dyn MessageSender>>,
    ) -> Result<Market> {
        if !asset_exist(&market_conf.money.name) || !asset_exist(&market_conf.stock.name) {
            return simple_err!("invalid assert name {} {}", market_conf.money.name, market_conf.stock.name);
        }
        if market_conf.stock.prec + market_conf.money.prec > asset_prec(&market_conf.money.name)
            || market_conf.stock.prec + market_conf.fee_prec > asset_prec(&market_conf.stock.name)
            || market_conf.money.prec + market_conf.fee_prec > asset_prec(&market_conf.money.name)
        {
            return simple_err!("invalid precision");
        }

        let market = Market {
            name: Box::leak(market_conf.name.clone().into_boxed_str()),
            stock: market_conf.stock.name.clone(),
            money: market_conf.money.name.clone(),
            stock_prec: market_conf.stock.prec,
            money_prec: market_conf.money.prec,
            fee_prec: market_conf.fee_prec,
            min_amount: market_conf.min_amount,
            sequencer,
            orders: HashMap::with_capacity(MAP_INIT_CAPACITY),
            users: HashMap::with_capacity(MAP_INIT_CAPACITY),
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            balance_manager: BalanceManagerWrapper { inner: balance_manager },
            history_writer,
            message_sender: MessageSenderWrapper { inner: message_sender },
        };
        Ok(market)
    }
    pub fn freeze_balance(&self, order: &Order) {
        let asset = if is_order_ask(order) { &self.stock } else { &self.money };
        self.balance_manager.balance_freeze(order.user, asset, &order.freeze);
    }
    pub fn unfreeze_balance(&self, order: &Order) {
        debug_assert!(order.left.is_sign_positive());
        if order.left.is_zero() {
            return;
        }
        let asset = if is_order_ask(&order) { &self.stock } else { &self.money };
        self.balance_manager.balance_unfreeze(order.user, asset, &order.freeze);
    }
    pub fn order_put(&mut self, order_rc: OrderRc) {
        let mut order = order_rc.borrow_mut();
        if order.side == OrderSide::ASK {
            order.freeze = order.left;
        } else {
            order.freeze = order.left * order.price;
        }
        debug_assert_eq!(order.type_0, OrderType::LIMIT);
        debug_assert!(!self.orders.contains_key(&order.id));
        //println!("order insert {}", &order.id);
        self.orders.insert(order.id.clone(), order_rc.clone());
        let user_map = self.users.entry(order.user).or_insert_with(BTreeMap::new);
        debug_assert!(!user_map.contains_key(&order.id));
        user_map.insert(order.id.clone(), order_rc.clone());
        if order.side == OrderSide::ASK {
            let key = order.to_ask_key();
            debug_assert!(!self.asks.contains_key(&key));
            self.asks.insert(key, order_rc.clone());
        } else {
            let key = order.to_bid_key();
            debug_assert!(!self.bids.contains_key(&key));
            self.bids.insert(key, order_rc.clone());
        }
        self.freeze_balance(&order);
    }

    fn order_finish(&mut self, real: bool, order: &Order) {
        if order.side == OrderSide::ASK {
            let key = &order.to_ask_key();
            debug_assert!(self.asks.contains_key(key));
            self.asks.remove(key);
        } else {
            let key = &order.to_bid_key();
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
            if order.deal_stock.is_sign_positive() {
                self.history_writer.borrow_mut().append_order_history(&order);
            }
            let order_message = OrderMessage {
                event: OrderEventType::FINISH,
                order: *order,
                stock: self.stock.clone(),
                money: self.money.clone(),
            };
            self.message_sender.push_order_message(&order_message);
        }
    }

    pub fn execute_limit_order(&mut self, real: bool, taker: OrderRc) {
        let taker_is_ask = taker.borrow_mut().side == OrderSide::ASK;
        let _taker_is_bid = !taker_is_ask;
        let maker_is_bid = taker_is_ask;
        let maker_is_ask = !maker_is_bid;

        let mut finished_orders = Vec::new();

        let iter: Box<dyn Iterator<Item = &mut OrderRc>> = if maker_is_bid {
            Box::new(self.bids.values_mut())
        } else {
            Box::new(self.asks.values_mut())
        };
        for maker in iter {
            if taker.borrow_mut().left.is_zero() {
                break;
            }
            let (ask_fee_rate, bid_fee_rate) = if taker_is_ask {
                (taker.borrow_mut().taker_fee, maker.borrow_mut().maker_fee)
            } else {
                (maker.borrow_mut().maker_fee, taker.borrow_mut().taker_fee)
            };
            let taker_mut = taker.borrow_mut();
            let maker_mut = maker.borrow_mut();
            let price = maker_mut.price;
            let (mut ask_order, mut bid_order) = if taker_is_ask {
                (taker_mut, maker_mut)
            } else {
                (maker_mut, taker_mut)
            };
            if ask_order.price.gt(&bid_order.price) {
                break;
            }
            let amount = min(ask_order.left, bid_order.left);
            let deal = price * amount;
            let ask_fee = deal * ask_fee_rate;
            let bid_fee = amount * bid_fee_rate;
            let timestamp = utils::current_timestamp();
            ask_order.update_time = timestamp;
            bid_order.update_time = timestamp;

            let deals_id = self.sequencer.borrow_mut().next_deal_id();
            if real {
                let deal = types::Deal {
                    id: deals_id,
                    timestamp: utils::current_timestamp(),
                    market: self.name.to_string(),
                    stock: self.stock.clone(),
                    money: self.money.clone(),
                    price,
                    amount,
                    deal,
                    ask_user_id: ask_order.user,
                    ask_order_id: ask_order.id,
                    ask_role: if taker_is_ask { MarketRole::TAKER } else { MarketRole::MAKER },
                    ask_fee,
                    bid_user_id: bid_order.user,
                    bid_order_id: bid_order.id,
                    bid_role: if taker_is_ask { MarketRole::MAKER } else { MarketRole::TAKER },
                    bid_fee,
                };
                self.history_writer.borrow_mut().append_deal_history(&deal);
                self.message_sender.push_deal_message(&deal);
            }
            ask_order.left -= amount;
            bid_order.left -= amount;
            ask_order.deal_stock += amount;
            bid_order.deal_stock += amount;
            ask_order.deal_money += deal;
            bid_order.deal_money += deal;
            ask_order.deal_fee += ask_fee;
            bid_order.deal_fee += bid_fee;

            // handle maker balance
            let _balance_type = if maker_is_bid {
                BalanceType::FREEZE
            } else {
                BalanceType::AVAILABLE
            };
            // handle stock
            self.balance_manager
                .balance_add(bid_order.user, BalanceType::AVAILABLE, &self.stock, &amount);
            self.balance_manager.balance_sub(
                ask_order.user,
                if maker_is_ask {
                    BalanceType::FREEZE
                } else {
                    BalanceType::AVAILABLE
                },
                &self.stock,
                &amount,
            );
            // handle money
            self.balance_manager
                .balance_add(ask_order.user, BalanceType::AVAILABLE, &self.money, &deal);
            self.balance_manager.balance_sub(
                bid_order.user,
                if maker_is_bid {
                    BalanceType::FREEZE
                } else {
                    BalanceType::AVAILABLE
                },
                &self.money,
                &deal,
            );

            if ask_fee.is_sign_positive() {
                self.balance_manager
                    .balance_sub(ask_order.user, BalanceType::AVAILABLE, &self.money, &ask_fee);
            }
            if bid_fee.is_sign_positive() {
                self.balance_manager
                    .balance_sub(bid_order.user, BalanceType::AVAILABLE, &self.stock, &bid_fee);
            }

            let (mut _taker_mut, mut maker_mut) = if taker_is_ask {
                (ask_order, bid_order)
            } else {
                (bid_order, ask_order)
            };
            maker_mut.freeze -= if maker_is_bid { deal } else { amount };

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
                    stock: self.stock.clone(),
                    money: self.money.clone(),
                };
                self.message_sender.push_order_message(&order_message);
            }
        }

        for item in finished_orders.iter() {
            self.order_finish(real, item);
        }
    }

    pub fn market_put_limit_order(&mut self, real: bool, order_input: &LimitOrderInput) -> Result<Order> {
        if order_input.side == OrderSide::ASK {
            if self
                .balance_manager
                .balance_get(order_input.user_id, BalanceType::AVAILABLE, &self.stock)
                .lt(&order_input.amount)
            {
                return simple_err!("balance not enough");
            }
        } else {
            let balance = self
                .balance_manager
                .balance_get(order_input.user_id, BalanceType::AVAILABLE, &self.money);
            if balance.lt(&(order_input.amount * order_input.price)) {
                return simple_err!("balance not enough");
            }
        }
        if order_input.amount.lt(&self.min_amount) {
            return simple_err!("invalid amount");
        }

        let t = utils::current_timestamp();
        let order_rc = Rc::new(RefCell::new(Order {
            id: self.sequencer.borrow_mut().next_order_id(),
            type_0: OrderType::LIMIT,
            side: order_input.side,
            create_time: t,
            update_time: t,
            market: &self.name,
            source: "",
            user: order_input.user_id,
            price: order_input.price,
            amount: order_input.amount,
            taker_fee: order_input.taker_fee,
            maker_fee: order_input.maker_fee,
            left: order_input.amount,
            freeze: Decimal::zero(),
            deal_stock: Decimal::zero(),
            deal_money: Decimal::zero(),
            deal_fee: Decimal::zero(),
        }));
        self.execute_limit_order(real, order_rc.clone());
        let order = *order_rc.borrow_mut();
        if order.left.is_zero() {
            if real {
                self.history_writer.borrow_mut().append_order_history(&order);
                let order_message = OrderMessage {
                    event: OrderEventType::FINISH,
                    order,
                    stock: self.stock.clone(),
                    money: self.money.clone(),
                };
                self.message_sender.push_order_message(&order_message);
            }
        } else {
            if real {
                let order_message = OrderMessage {
                    event: OrderEventType::PUT,
                    order,
                    stock: self.stock.clone(),
                    money: self.money.clone(),
                };
                self.message_sender.push_order_message(&order_message);
            }
            self.order_put(order_rc);
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
    pub fn depth(&self, limit: usize) -> MarketDepth {
        MarketDepth {
            asks: Self::group_ordebook(&self.asks, limit),
            bids: Self::group_ordebook(&self.bids, limit),
        }
    }
    fn group_ordebook<K>(orderbook: &BTreeMap<K, OrderRc>, limit: usize) -> Vec<PriceInfo> {
        // TODO rust language server cannot handle this ...
        orderbook
            .values()
            .map(|o| o.borrow_mut())
            .map(|o| (o.price, o.left))
            .group_by(|(price, _)| *price)
            .into_iter()
            .take(limit)
            .map(|(price, group)| PriceInfo {
                price,
                amount: group.map(|(_, left)| left).sum(),
            })
            .collect()
    }
    /*
    fn group_ordebook_by_key<K>(orderbook: &BTreeMap<K, OrderRc>, limit: usize, key: Box<dyn Fn(&Decimal) -> Decimal>) -> Vec<PriceInfo> {
        orderbook.values().group_by(
            |&o| key(o.borrow_mut().price)).into_iter().take(limit)
            .map(|k, g| PriceInfo { price: k, amount: g.sum::<Decimal>() }).collect()
    }
    */
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

pub struct LimitOrderInput {
    pub user_id: u32,
    pub side: OrderSide,
    pub amount: Decimal,
    pub price: Decimal,
    pub taker_fee: Decimal, // FIXME fee should be determined inside engine rather than take from input
    pub maker_fee: Decimal,
    pub source: String,
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

    fn get_simple_market_config() -> config::Market {
        config::Market {
            name: String::from("eth/btc"),
            stock: config::MarketUnit { name: eth(), prec: 6 },
            money: config::MarketUnit { name: btc(), prec: 4 },
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
        let sequencer = Rc::new(RefCell::new(Sequencer {
            order_id_start: 0,
            deals_id_start: 0,
            operlog_id_start: 0,
        }));
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
        let ask_order_input = LimitOrderInput {
            user_id: ask_user_id,
            side: OrderSide::ASK,
            amount: dec!(20.0),
            price: dec!(0.1),
            taker_fee: dec!(0.001),
            maker_fee: dec!(0.001),
            source: String::from(""),
            market: market.name.to_string(),
        };
        let ask_order = market.market_put_limit_order(false, &ask_order_input).unwrap();
        assert_eq!(ask_order.id, 1);
        assert_eq!(ask_order.left, dec!(20.0));

        let bid_user_id = 102;
        let bid_order_input = LimitOrderInput {
            user_id: bid_user_id,
            side: OrderSide::BID,
            amount: dec!(10.0),
            price: dec!(0.11),
            taker_fee: dec!(0.001),
            maker_fee: dec!(0.001),
            source: String::from(""),
            market: market.name.to_string(),
        };
        let bid_order = market.market_put_limit_order(false, &bid_order_input).unwrap();
        // deal: price: 0.10 amount: 10
        assert_eq!(bid_order.id, 2);
        assert_eq!(bid_order.left, dec!(0));
        assert_eq!(bid_order.deal_money, dec!(1));
        assert_eq!(bid_order.deal_stock, dec!(10));
        assert_eq!(bid_order.deal_fee, dec!(0.01));

        //market.print();

        let ask_order = market.get(ask_order.id).unwrap();
        assert_eq!(ask_order.left, dec!(10));
        assert_eq!(ask_order.deal_money, dec!(1));
        assert_eq!(ask_order.deal_stock, dec!(10));
        assert_eq!(ask_order.deal_fee, dec!(0.001));

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
