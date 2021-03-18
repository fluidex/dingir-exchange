use crate::market::Order;
use crate::types::{OrderEventType, SimpleResult};
use core::cell::RefCell;

use anyhow::{anyhow, Result};
use crossbeam_channel::RecvTimeoutError;
use rdkafka::client::ClientContext;
use rdkafka::config::ClientConfig;
use rdkafka::error::{KafkaError, RDKafkaErrorCode};
use rdkafka::producer::{BaseProducer, BaseRecord, DeliveryResult, Producer, ProducerContext};

use serde::{Deserialize, Serialize};

use std::collections::LinkedList;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub mod consumer;
pub mod persist;

pub struct SimpleProducerContext;
impl ClientContext for SimpleProducerContext {}
impl ProducerContext for SimpleProducerContext {
    type DeliveryOpaque = ();
    fn delivery(&self, result: &DeliveryResult, _: Self::DeliveryOpaque) {
        match result {
            // TODO: how to handle this err
            Err(e) => log::error!("kafka send err: {:?}", e),
            Ok(_r) => {
                // log::info!("kafka send done: {:?}", r)
            }
        }
    }
}
pub const ORDERS_TOPIC: &str = "orders";
pub const TRADES_TOPIC: &str = "trades";
pub const BALANCES_TOPIC: &str = "balances";

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

// https://rust-lang.github.io/rust-clippy/master/index.html#large_enum_variant
// TODO: better naming?
// TODO: change push_order_message etc interface to this enum class?
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "value")]
pub enum Message {
    BalanceMessage(Box<BalanceMessage>),
    OrderMessage(Box<OrderMessage>),
    TradeMessage(Box<Trade>),
}

//re-export from market, act as TradeMessage
pub use crate::market::Trade;

#[derive(Serialize, Deserialize)]
pub struct MessageSenderStatus {
    trades_len: usize,
    orders_len: usize,
    balances_len: usize,
}

pub struct KafkaMessageSender {
    producer: Arc<BaseProducer<SimpleProducerContext>>,
    orders_list: RefCell<LinkedList<String>>,
    trades_list: RefCell<LinkedList<String>>,
    balances_list: RefCell<LinkedList<String>>,
    receiver: crossbeam_channel::Receiver<(&'static str, String)>,
}

impl KafkaMessageSender {
    pub fn new(brokers: &str, receiver: crossbeam_channel::Receiver<(&'static str, String)>) -> Result<KafkaMessageSender> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("queue.buffering.max.ms", "1")
            .create_with_context(SimpleProducerContext)?;
        let arc = Arc::new(producer);

        Ok(KafkaMessageSender {
            producer: arc,
            trades_list: RefCell::new(LinkedList::new()),
            orders_list: RefCell::new(LinkedList::new()),
            balances_list: RefCell::new(LinkedList::new()),
            receiver,
        })
    }
    pub fn on_message(&self, topic_name: &str, message: &str) -> SimpleResult {
        log::debug!("KAFKA: push {} message: {}", topic_name, message);
        let mut list = match topic_name {
            BALANCES_TOPIC => self.balances_list.borrow_mut(),
            TRADES_TOPIC => self.trades_list.borrow_mut(),
            ORDERS_TOPIC => self.orders_list.borrow_mut(),
            _ => unreachable!(),
        };

        // busy, so not push message now
        if !list.is_empty() {
            list.push_back(message.to_string());
            return Ok(());
        }
        let record = BaseRecord::to(topic_name).key("").payload(message);
        let result = self.producer.send(record);
        if result.is_err() {
            log::error!("fail to push message {} to {}", message, topic_name);
            if let Err((KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull), _)) = result {
                list.push_back(message.to_string());
                return Ok(());
            }
            return Err(anyhow!("kafka push err"));
        }
        Ok(())
    }
    pub fn finish(self) -> SimpleResult {
        self.flush();
        self.producer.flush(std::time::Duration::from_millis(1000));
        drop(self);
        Ok(())
    }

    // if kafka is full, queue messages in list, so here flush them.
    fn flush_list(&self, topic_name: &str) {
        let mut list = match topic_name {
            BALANCES_TOPIC => self.balances_list.borrow_mut(),
            TRADES_TOPIC => self.trades_list.borrow_mut(),
            ORDERS_TOPIC => self.orders_list.borrow_mut(),
            _ => unreachable!(),
        };
        for message in list.iter() {
            let result = self.producer.send(BaseRecord::to(topic_name).key("").payload(message.as_str()));

            if result.is_err() {
                // log::error!("fail to push message {} to {}", message_str, topic_name);
                if let Err((KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull), _)) = result {
                    break;
                }
            }
        }
        list.clear();
    }

    fn flush(&self) {
        self.flush_list(BALANCES_TOPIC);
        self.flush_list(ORDERS_TOPIC);
        self.flush_list(TRADES_TOPIC);
        self.producer.poll(Duration::from_millis(0));
    }

    pub fn start(self) {
        let mut last_flush_time = Instant::now();
        let flush_interval = std::time::Duration::from_millis(100);
        let timeout_interval = std::time::Duration::from_millis(100);
        loop {
            if self.is_block() {
                log::warn!("kafka sender buffer is full");
                // skip receiving from channel, so main server can know something goes wrong
                // sleep to avoid cpu 100% usage
                thread::sleep(flush_interval);
            } else {
                match self.receiver.recv_timeout(timeout_interval) {
                    Ok((topic, message)) => {
                        self.on_message(topic, &message).ok();
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => {
                        log::info!("kafka producer disconnected");
                        break;
                    }
                }
            }
            let now = Instant::now();
            if now > last_flush_time + flush_interval {
                self.flush();
                last_flush_time = now;
            }
        }
        self.finish().ok();
        log::info!("kafka sender exit");
    }

    pub fn is_block(&self) -> bool {
        self.trades_list.borrow_mut().len() >= 100
            || self.orders_list.borrow_mut().len() >= 100
            || self.balances_list.borrow_mut().len() >= 100
    }

    pub fn status(&self) -> MessageSenderStatus {
        MessageSenderStatus {
            trades_len: self.trades_list.borrow_mut().len(),
            orders_len: self.orders_list.borrow_mut().len(),
            balances_len: self.balances_list.borrow_mut().len(),
        }
    }
}

pub trait MessageManager {
    //fn push_message(&mut self, msg: &Message);
    fn is_block(&self) -> bool;
    fn push_order_message(&mut self, order: &OrderMessage);
    fn push_trade_message(&mut self, trade: &Trade);
    fn push_balance_message(&mut self, balance: &BalanceMessage);
}

pub struct ChannelMessageManager {
    pub sender: crossbeam_channel::Sender<(&'static str, String)>,
}

impl ChannelMessageManager {
    fn push_message_and_topic(&self, message: String, topic_name: &'static str) {
        //log::debug!("KAFKA: push {} message: {}", topic_name, message);
        self.sender.try_send((topic_name, message)).unwrap();
    }
}

impl MessageManager for ChannelMessageManager {
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
        self.sender.len() >= (self.sender.capacity().unwrap() as f64 * 0.9) as usize
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
}

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

pub fn new_message_manager_with_kafka_backend(brokers: &str) -> Result<ChannelMessageManager> {
    let (sender, receiver) = crossbeam_channel::bounded(100);
    let kafka_sender = KafkaMessageSender::new(brokers, receiver)?;
    // TODO: join handle?
    std::thread::spawn(move || kafka_sender.start());
    Ok(ChannelMessageManager { sender })
}
