use crate::market::Order;
use crate::types::{Deal, OrderEventType, SimpleResult};

use anyhow::Result;
use rdkafka::client::ClientContext;
use rdkafka::config::ClientConfig;
use rdkafka::error::{KafkaError, RDKafkaError};
use rdkafka::producer::{BaseProducer, BaseRecord, DeliveryResult, ProducerContext};

use serde::{Deserialize, Serialize};

use std::collections::LinkedList;
use std::sync::Arc;
use std::time::Duration;

pub struct SimpleProducerContext;
impl ClientContext for SimpleProducerContext {}
impl ProducerContext for SimpleProducerContext {
    type DeliveryOpaque = ();
    fn delivery(&self, result: &DeliveryResult, _: Self::DeliveryOpaque) {
        match result {
            Err(e) => println!("kafka send err: {:?}", e),
            Ok(r) => println!("kafka send done: {:?}", r),
        }
    }
}
pub(crate) const ORDERS_TOPIC: &str = "orders";
pub(crate) const DEALS_TOPIC: &str = "deals";
pub(crate) const BALANCES_TOPIC: &str = "balances";

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceMessage {
    pub timestamp: f64,
    pub user_id: u32,
    pub asset: String,
    pub business: String,
    pub change: String,
}

#[derive(Debug, Serialize)] //, Deserialize)]
pub struct OrderMessage {
    pub event: OrderEventType,
    pub order: Order,
    pub stock: String,
    pub money: String,
}

#[derive(Serialize, Deserialize)]
pub struct MessageSenderStatus {
    deals_len: usize,
    orders_len: usize,
    balances_len: usize,
}

pub trait MessageSender {
    fn push_order_message(&mut self, order: &OrderMessage) -> SimpleResult;
    fn push_deal_message(&mut self, deal: &Deal) -> SimpleResult;
    fn push_balance_message(&mut self, balance: &BalanceMessage) -> SimpleResult;
}

pub struct DummyMessageSender;
impl MessageSender for DummyMessageSender {
    fn push_order_message(&mut self, _order: &OrderMessage) -> SimpleResult {
        Ok(())
    }
    fn push_deal_message(&mut self, _deal: &Deal) -> SimpleResult {
        Ok(())
    }
    fn push_balance_message(&mut self, _balance: &BalanceMessage) -> SimpleResult {
        Ok(())
    }
}

pub struct KafkaMessageSender {
    producer: Arc<BaseProducer<SimpleProducerContext>>,
    orders_list: LinkedList<String>,
    deals_list: LinkedList<String>,
    balances_list: LinkedList<String>,
}
impl KafkaMessageSender {
    pub fn new(brokers: &str) -> Result<KafkaMessageSender> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("queue.buffering.max.ms", "1")
            .create_with_context(SimpleProducerContext)?;
        let arc = Arc::new(producer);

        Ok(KafkaMessageSender {
            producer: arc,
            deals_list: LinkedList::new(),
            orders_list: LinkedList::new(),
            balances_list: LinkedList::new(),
        })
    }
    pub fn push_message(&mut self, message: &str, topic_name: &str) -> SimpleResult {
        println!("KAFA: push {} message: {}", topic_name, message);
        let list: &mut LinkedList<String> = match topic_name {
            BALANCES_TOPIC => &mut self.balances_list,
            DEALS_TOPIC => &mut self.deals_list,
            ORDERS_TOPIC => &mut self.orders_list,
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
            println!("fail to push message {} to {}", message, topic_name);
            if let Err((KafkaError::MessageProduction(RDKafkaError::QueueFull), _)) = result {
                list.push_back(message.to_string());
                return Ok(());
            }
            return simple_err!("kafka push err");
        }
        Ok(())
    }
    pub fn finish(mut self) -> SimpleResult {
        self.flush();
        self.producer.flush(std::time::Duration::from_millis(1000));
        drop(self);
        Ok(())
    }

    // if kafka is full, queue messages in list, so here flush them.
    fn flush_list(&mut self, topic_name: &str) {
        let list: &mut LinkedList<String> = match topic_name {
            BALANCES_TOPIC => &mut self.balances_list,
            DEALS_TOPIC => &mut self.deals_list,
            ORDERS_TOPIC => &mut self.orders_list,
            _ => unreachable!(),
        };
        for message in list.iter() {
            let result = self.producer.send(BaseRecord::to(topic_name).key("").payload(message.as_str()));

            if result.is_err() {
                //println!("fail to push message {} to {}", message_str, topic_name);
                if let Err((KafkaError::MessageProduction(RDKafkaError::QueueFull), _)) = result {
                    break;
                }
            }
        }
        list.clear();
    }

    fn flush(&mut self) {
        self.flush_list(BALANCES_TOPIC);
        self.flush_list(ORDERS_TOPIC);
        self.flush_list(DEALS_TOPIC);
        self.producer.poll(Duration::from_millis(0));
    }

    pub fn start_timer(&self) {
        let mut ticker = tokio::time::interval(std::time::Duration::from_millis(100));
        let self_ptr: *mut Self = self as *const Self as *mut Self;
        tokio::task::spawn_local(async move {
            loop {
                ticker.tick().await;
                unsafe {
                    (*self_ptr).flush();
                }
            }
        });
    }

    pub fn is_block(&self) -> bool {
        self.deals_list.len() >= 100 || self.orders_list.len() >= 100 || self.balances_list.len() >= 100
    }

    pub fn status(&self) -> MessageSenderStatus {
        MessageSenderStatus {
            deals_len: self.deals_list.len(),
            orders_len: self.orders_list.len(),
            balances_len: self.balances_list.len(),
        }
    }
}

impl MessageSender for KafkaMessageSender {
    fn push_order_message(&mut self, order: &OrderMessage) -> SimpleResult {
        let message = serde_json::to_string(&order)?;
        self.push_message(&message, ORDERS_TOPIC)
    }
    fn push_deal_message(&mut self, deal: &Deal) -> SimpleResult {
        let message = serde_json::to_string(&deal)?;
        self.push_message(&message, DEALS_TOPIC)
    }
    fn push_balance_message(&mut self, balance: &BalanceMessage) -> SimpleResult {
        let message = serde_json::to_string(&balance)?;
        self.push_message(&message, BALANCES_TOPIC)
    }
}
