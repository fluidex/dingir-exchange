use crate::market::Order;
use crate::types::{OrderEventType, SimpleResult};
use core::cell::RefCell;

use anyhow::{anyhow, Result};
use crossbeam_channel::RecvTimeoutError;
use rdkafka::client::ClientContext;
use rdkafka::config::ClientConfig;
use rdkafka::error::{KafkaError, RDKafkaErrorCode};
use rdkafka::producer::{BaseProducer, BaseRecord, DeliveryResult, Producer, ProducerContext};
use rdkafka::message::{ToBytes}

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

pub trait MessageSenderScheme : ProducerContext {
    type KeyType: ToBytes + ?Sized
    fn on_message(&self, title_tip: &str, message: &str) -> 
        BaseRecord<'_, Self::KeyType, String, Self::DeliveryOpaque>
    fn on_send_queue_full(&self, BaseRecord<'_, Self::KeyType, String, Self::DeliveryOpaque>)
}

pub trait ClientContextWithSettings : ClientContext {
    fn settings<K: Into<String>, V: Into<String>>() -> [(K, V)]
}

//provide a running kafka producer instance which keep sending message under the full-ordering scheme
//it simply block the Sender side of crossbeam_channel when the deliver queue is full, and quit
//only when the sender side is closed
pub struct RdProducerRunner<T: MessageSenderScheme> {
    producer: BaseProducer<T>,
    receiver: crossbeam_channel::Receiver<(&'static str, String)>,
}

impl<T: MessageSenderScheme> RdProducerRunner<T> {
    pub fn new(brokers: &str, receiver: crossbeam_channel::Receiver<(&'static str, String)>) -> Result<KafkaMessageSender> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("queue.buffering.max.ms", "1")
            .set("enable.idempotence", "yes")
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

    pub fn run(self) {
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
}
