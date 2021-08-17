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
    pub asset: String,
    pub business: String,
    pub change: String,
    pub balance: String,
    pub balance_available: String,
    pub balance_frozen: String,
    pub detail: String,
}

impl From<&BalanceHistory> for BalanceMessage {
    fn from(balance: &BalanceHistory) -> Self {
        Self {
            timestamp: balance.time.timestamp() as f64,
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
            timestamp: balance.time.timestamp() as f64,
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
}

impl From<&BalanceHistory> for WithdrawMessage {
    fn from(balance: &BalanceHistory) -> Self {
        Self {
            timestamp: balance.time.timestamp() as f64,
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
pub struct TransferMessage {
    pub time: f64,
    pub user_from: u32,
    pub user_to: u32,
    pub signature: String,
    pub asset: String,
    pub amount: String,
}

impl From<InternalTx> for TransferMessage {
    fn from(tx: InternalTx) -> Self {
        Self {
            time: FTimestamp::from(&tx.time).into(),
            user_from: tx.user_from as u32,
            user_to: tx.user_to as u32,
            signature: tx.signature.to_string(),
            asset: tx.asset.to_string(),
            amount: tx.amount.to_string(),
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

pub struct RdProducerStub<T> {
    pub sender: crossbeam_channel::Sender<(&'static str, String)>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> RdProducerStub<T> {
    fn push_message_and_topic(&self, message: String, topic_name: &'static str) {
        //log::debug!("KAFKA: push {} message: {}", topic_name, message);
        self.sender.try_send((topic_name, message)).unwrap();
    }
}

impl<T: producer::MessageScheme + 'static> RdProducerStub<T> {
    pub fn new_and_run(brokers: &str) -> Result<Self> {
        //now the channel is just need to provide a small buffer which is
        //enough to accommodate a pluse request in some time slice of thread
        let (sender, receiver) = crossbeam_channel::bounded(2048);

        let producer_context: producer::RdProducerContext<T> = Default::default();

        let kafkaproducer = producer_context.new_producer(brokers)?;
        std::thread::spawn(move || {
            producer::RdProducerContext::<T>::run_default(kafkaproducer, receiver);
        });
        Ok(Self {
            sender,
            _phantom: std::marker::PhantomData,
        })
    }
}

impl<T: producer::MessageScheme> MessageManager for RdProducerStub<T> {
    /*
    fn push_message(&mut self, msg: &Message) {
        match msg {
            Message::OrderMessage{value: order} => {
                let message = serde_json::to_string(order).unwrap();
                self.push_message_and_topic(message, ORDERS_TOPIC)
            },
            Message::BalanceMessage{value: balance} => {
                let message = serde_json::to_string(balance).unwrap();
                self.push_message_and_topic(message, BALANCES_TOPIC)
            },
            Message::TradeMessage{value: trade} => {
                let message = serde_json::to_string(trade).unwrap();
                self.push_message_and_topic(message, TRADES_TOPIC)
            }
        }
    }
    */

    fn is_block(&self) -> bool {
        // https://github.com/fluidex/dingir-exchange/issues/119
        //self.sender.is_full()
        //self.sender.len() >= (self.sender.capacity().unwrap() as f64 * 0.9) as usize
        self.sender.len() >= (self.sender.capacity().unwrap() - 1000)
    }
    fn push_order_message(&mut self, order: &OrderMessage) {
        let message = serde_json::to_string(&order).unwrap();
        self.push_message_and_topic(message, ORDERS_TOPIC)
    }
    fn push_trade_message(&mut self, trade: &Trade) {
        let message = serde_json::to_string(&trade).unwrap();
        self.push_message_and_topic(message, TRADES_TOPIC)
    }
    fn push_balance_message(&mut self, balance: &BalanceMessage) {
        let message = serde_json::to_string(&balance).unwrap();
        self.push_message_and_topic(message, BALANCES_TOPIC)
    }
    fn push_deposit_message(&mut self, deposit: &DepositMessage) {
        let message = serde_json::to_string(&deposit).unwrap();
        self.push_message_and_topic(message, DEPOSITS_TOPIC)
    }
    fn push_withdraw_message(&mut self, withdraw: &WithdrawMessage) {
        let message = serde_json::to_string(&withdraw).unwrap();
        self.push_message_and_topic(message, WITHDRAWS_TOPIC)
    }
    fn push_transfer_message(&mut self, tx: &TransferMessage) {
        let message = serde_json::to_string(&tx).unwrap();
        self.push_message_and_topic(message, INTERNALTX_TOPIC)
    }
    fn push_user_message(&mut self, user: &UserMessage) {
        let message = serde_json::to_string(&user).unwrap();
        self.push_message_and_topic(message, USER_TOPIC)
    }
}

pub type SimpleMessageManager = RdProducerStub<producer::SimpleMessageScheme>;

// TODO: since now we enable SimpleMessageManager & FullOrderMessageManager both,
// we only need to process useful (deposit,trade etc, which update the rollup global state) msgs only
// and skip others
pub type FullOrderMessageManager = RdProducerStub<producer::FullOrderMessageScheme>;

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
    SimpleMessageManager::new_and_run(brokers)
}

pub fn new_full_order_message_manager(brokers: &str) -> Result<FullOrderMessageManager> {
    FullOrderMessageManager::new_and_run(brokers)
}
