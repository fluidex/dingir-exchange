use crate::market::Order;
pub use crate::models::{AccountDesc, BalanceHistory, InternalTx};
use crate::types::OrderEventType;

use crate::utils::FTimestamp;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod consumer;
pub mod persist;
pub mod producer;

pub use producer::{
    BALANCES_TOPIC, DEPOSITS_TOPIC, INTERNALTX_TOPIC, ORDERS_TOPIC, TRADES_TOPIC, UNIFY_TOPIC, USER_TOPIC, WITHDRAWS_TOPIC,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserMessage {
    pub user_id: u32,
    pub l1_address: String,
    pub l2_pubkey: String,
}

impl From<AccountDesc> for UserMessage {
    fn from(user: AccountDesc) -> Self {
        Self {
            user_id: user.id as u32,
            l1_address: user.l1_address,
            l2_pubkey: user.l2_pubkey,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BalanceMessage {
    pub timestamp: f64,
    pub user_id: u32,
    pub business_id: u64,
    pub asset: String,
    pub business: String,
    pub market_price: String,
    pub change: String,
    pub balance: String,
    pub balance_available: String,
    pub balance_frozen: String,
    pub detail: String,
    pub signature: String,
}

impl From<&BalanceHistory> for BalanceMessage {
    fn from(balance: &BalanceHistory) -> Self {
        Self {
            timestamp: balance.time.and_utc().timestamp() as f64,
            user_id: balance.user_id as u32,
            business_id: balance.business_id as u64,
            asset: balance.asset.clone(),
            business: balance.business.clone(),
            market_price: balance.market_price.to_string(),
            change: balance.change.to_string(),
            balance: balance.balance.to_string(),
            balance_available: balance.balance_available.to_string(),
            balance_frozen: balance.balance_frozen.to_string(),
            detail: balance.detail.clone(),
            signature: String::from_utf8(balance.signature.clone()).unwrap(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DepositMessage {
    pub timestamp: f64,
    pub user_id: u32,
    pub asset: String,
    pub business: String,
    pub change: String,
    pub balance: String,
    pub balance_available: String,
    pub balance_frozen: String,
    pub detail: String,
}

impl From<&BalanceHistory> for DepositMessage {
    fn from(balance: &BalanceHistory) -> Self {
        Self {
            timestamp: balance.time.and_utc().timestamp() as f64,
            user_id: balance.user_id as u32,
            asset: balance.asset.clone(),
            business: balance.business.clone(),
            change: balance.change.to_string(),
            balance: balance.balance.to_string(),
            balance_available: balance.balance_available.to_string(),
            balance_frozen: balance.balance_frozen.to_string(),
            detail: balance.detail.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WithdrawMessage {
    pub timestamp: f64,
    pub user_id: u32,
    pub asset: String,
    pub business: String,
    pub change: String,
    pub balance: String,
    pub balance_available: String,
    pub balance_frozen: String,
    pub detail: String,
    pub signature: String,
}

impl From<&BalanceHistory> for WithdrawMessage {
    fn from(balance: &BalanceHistory) -> Self {
        Self {
            timestamp: balance.time.and_utc().timestamp() as f64,
            user_id: balance.user_id as u32,
            asset: balance.asset.clone(),
            business: balance.business.clone(),
            change: balance.change.to_string(),
            balance: balance.balance.to_string(),
            balance_available: balance.balance_available.to_string(),
            balance_frozen: balance.balance_frozen.to_string(),
            detail: balance.detail.clone(),
            signature: String::from_utf8(balance.signature.clone()).unwrap(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferMessage {
    pub time: f64,
    pub user_from: u32,
    pub user_to: u32,
    pub asset: String,
    pub amount: String,
    pub signature: String,
}

impl From<InternalTx> for TransferMessage {
    fn from(tx: InternalTx) -> Self {
        Self {
            time: FTimestamp::from(&tx.time).into(),
            user_from: tx.user_from as u32,
            user_to: tx.user_to as u32,
            asset: tx.asset,
            amount: tx.amount.to_string(),
            signature: String::from_utf8(tx.signature).unwrap(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderMessage {
    pub event: OrderEventType,
    pub order: Order,
    pub base: String,
    pub quote: String,
}

impl OrderMessage {
    pub fn from_order(order: &Order, at_step: OrderEventType) -> Self {
        Self {
            event: at_step,
            order: *order,
            base: order.base.to_string(),
            quote: order.quote.to_string(),
        }
    }
}
//re-export from market, act as TradeMessage
pub use crate::market::Trade;

//TODO: senderstatus is not used anymore?
#[derive(Serialize, Deserialize)]
pub struct MessageSenderStatus {
    trades_len: usize,
    orders_len: usize,
    balances_len: usize,
}

pub trait MessageManager: Sync + Send {
    //fn push_message(&mut self, msg: &Message);
    fn is_block(&self) -> bool;
    fn push_order_message(&mut self, order: &OrderMessage);
    fn push_trade_message(&mut self, trade: &Trade);
    fn push_balance_message(&mut self, balance: &BalanceMessage);
    fn push_deposit_message(&mut self, balance: &DepositMessage);
    fn push_withdraw_message(&mut self, balance: &WithdrawMessage);
    fn push_transfer_message(&mut self, tx: &TransferMessage);
    fn push_user_message(&mut self, user: &UserMessage);
}

pub struct KafkaMessageManager {
    producer: producer::KafkaProducer,
    full_order: bool,
}

impl KafkaMessageManager {
    pub fn new(brokers: &str, full_order: bool) -> Result<Self> {
        Ok(Self {
            producer: producer::KafkaProducer::new(brokers, full_order)?,
            full_order,
        })
    }
}

impl MessageManager for KafkaMessageManager {
    fn is_block(&self) -> bool {
        self.producer.is_full()
    }
    fn push_order_message(&mut self, order: &OrderMessage) {
        let message = serde_json::to_string(order).unwrap();
        if self.full_order {
            self.producer.try_send(UNIFY_TOPIC, ORDERS_TOPIC, message);
        } else {
            self.producer.try_send(ORDERS_TOPIC, "", message);
        }
    }
    fn push_trade_message(&mut self, trade: &Trade) {
        let message = serde_json::to_string(trade).unwrap();
        if self.full_order {
            self.producer.try_send(UNIFY_TOPIC, TRADES_TOPIC, message);
        } else {
            self.producer.try_send(TRADES_TOPIC, "", message);
        }
    }
    fn push_balance_message(&mut self, balance: &BalanceMessage) {
        let message = serde_json::to_string(balance).unwrap();
        if self.full_order {
            self.producer.try_send(UNIFY_TOPIC, BALANCES_TOPIC, message);
        } else {
            self.producer.try_send(BALANCES_TOPIC, "", message);
        }
    }
    fn push_deposit_message(&mut self, deposit: &DepositMessage) {
        let message = serde_json::to_string(deposit).unwrap();
        if self.full_order {
            self.producer.try_send(UNIFY_TOPIC, DEPOSITS_TOPIC, message);
        } else {
            self.producer.try_send(DEPOSITS_TOPIC, "", message);
        }
    }
    fn push_withdraw_message(&mut self, withdraw: &WithdrawMessage) {
        let message = serde_json::to_string(withdraw).unwrap();
        if self.full_order {
            self.producer.try_send(UNIFY_TOPIC, WITHDRAWS_TOPIC, message);
        } else {
            self.producer.try_send(WITHDRAWS_TOPIC, "", message);
        }
    }
    fn push_transfer_message(&mut self, tx: &TransferMessage) {
        let message = serde_json::to_string(tx).unwrap();
        if self.full_order {
            self.producer.try_send(UNIFY_TOPIC, INTERNALTX_TOPIC, message);
        } else {
            self.producer.try_send(INTERNALTX_TOPIC, "", message);
        }
    }
    fn push_user_message(&mut self, user: &UserMessage) {
        let message = serde_json::to_string(user).unwrap();
        if self.full_order {
            self.producer.try_send(UNIFY_TOPIC, USER_TOPIC, message);
        } else {
            self.producer.try_send(USER_TOPIC, "", message);
        }
    }
}

pub type SimpleMessageManager = KafkaMessageManager;
pub type FullOrderMessageManager = KafkaMessageManager;

// https://rust-lang.github.io/rust-clippy/master/index.html#large_enum_variant
// TODO: better naming?
// TODO: change push_order_message etc interface to this enum class?
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "value")]
pub enum Message {
    BalanceMessage(Box<BalanceMessage>),
    DepositMessage(Box<BalanceMessage>),
    OrderMessage(Box<OrderMessage>),
    TradeMessage(Box<Trade>),
    TransferMessage(Box<TransferMessage>),
    UserMessage(Box<UserMessage>),
    WithdrawMessage(Box<BalanceMessage>),
}

/*
pub struct DummyMessageManager {
    // debug purpose only
    pub keep_data: bool,
    pub data: Vec<Message>,
}
impl MessageManager for DummyMessageManager {
    //fn push_message(&mut self, msg: &Message) {
    //    if self.keep_data {
    //        self.data.push(msg.clone());
    //    }
    //}

    fn is_block(&self) -> bool {
        false
    }
    fn push_order_message(&mut self, order: &OrderMessage) {
        if self.keep_data {
            self.data.push(Message::OrderMessage(Box::new(order.clone())));
        }
    }
    fn push_trade_message(&mut self, trade: &Trade) {
        if self.keep_data {
            self.data.push(Message::TradeMessage(Box::new(trade.clone())));
        }
    }
    fn push_balance_message(&mut self, balance: &BalanceMessage) {
        if self.keep_data {
            self.data.push(Message::BalanceMessage(Box::new(balance.clone())));
        }
    }
}
*/

pub fn new_simple_message_manager(brokers: &str) -> Result<SimpleMessageManager> {
    KafkaMessageManager::new(brokers, false)
}

pub fn new_full_order_message_manager(brokers: &str) -> Result<FullOrderMessageManager> {
    KafkaMessageManager::new(brokers, true)
}

#[cfg(test)]
mod tests;
