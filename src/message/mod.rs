use crate::market::Order;
use crate::types::OrderEventType;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod consumer;
pub mod persist;
pub mod producer;

pub use producer::{BALANCES_TOPIC, ORDERS_TOPIC, TRADES_TOPIC, UNIFY_TOPIC, USER_TOPIC};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserMessage {
    pub user_id: u32,
    pub l1_address: String,
    pub l2_pubkey: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BalanceMessage {
    pub timestamp: f64,
    pub user_id: u32,
    pub asset: String,
    pub business: String,
    pub change: String,
    pub balance: String,
    pub detail: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderMessage {
    pub event: OrderEventType,
    pub order: Order,
    pub base: String,
    pub quote: String,
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

pub trait MessageManager {
    //fn push_message(&mut self, msg: &Message);
    fn is_block(&self) -> bool;
    fn push_order_message(&mut self, order: &OrderMessage);
    fn push_trade_message(&mut self, trade: &Trade);
    fn push_balance_message(&mut self, balance: &BalanceMessage);
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
        // https://github.com/Fluidex/dingir-exchange/issues/119
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
    fn push_user_message(&mut self, user: &UserMessage) {
        let message = serde_json::to_string(&user).unwrap();
        self.push_message_and_topic(message, USER_TOPIC)
    }
}

pub type ChannelMessageManager = RdProducerStub<producer::SimpleMessageScheme>;
//pub type ChannelMessageManager = RdProducerStub<producer::FullOrderMessageScheme>;
pub type UnifyMessageManager = RdProducerStub<producer::FullOrderMessageScheme>;

// https://rust-lang.github.io/rust-clippy/master/index.html#large_enum_variant
// TODO: better naming?
// TODO: change push_order_message etc interface to this enum class?
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "value")]
pub enum Message {
    BalanceMessage(Box<BalanceMessage>),
    UserMessage(Box<UserMessage>),
    OrderMessage(Box<OrderMessage>),
    TradeMessage(Box<Trade>),
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

pub fn new_message_manager_with_kafka_backend(brokers: &str) -> Result<ChannelMessageManager> {
    ChannelMessageManager::new_and_run(brokers)
}
