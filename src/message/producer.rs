use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;
use tokio::sync::mpsc;

pub const BALANCES_TOPIC: &str = "balances";
pub const DEPOSITS_TOPIC: &str = "deposits";
pub const INTERNALTX_TOPIC: &str = "internaltransfer";
pub const ORDERS_TOPIC: &str = "orders";
pub const TRADES_TOPIC: &str = "trades";
pub const UNIFY_TOPIC: &str = "unifyevents";
pub const USER_TOPIC: &str = "registeruser";
pub const WITHDRAWS_TOPIC: &str = "withdraws";

pub struct KafkaProducer {
    sender: mpsc::Sender<(&'static str, &'static str, String)>,
}

impl KafkaProducer {
    pub fn new(brokers: &str, full_order: bool) -> Result<Self> {
        let (sender, mut receiver) = mpsc::channel(2048);

        let mut config = ClientConfig::new();
        config.set("bootstrap.servers", brokers);
        if full_order {
            config.set("enable.idempotence", "true");
            config.set("max.in.flight.requests.per.connection", "1");
        }

        let producer: FutureProducer = config.create()?;

        tokio::spawn(async move {
            while let Some((topic, key, payload)) = receiver.recv().await {
                let record = FutureRecord::to(topic).key(key).payload(&payload);
                match producer.send(record, Duration::from_secs(0)).await {
                    Ok((partition, offset)) => {
                        log::debug!("delivered {}@{} to {}", offset, partition, topic);
                    }
                    Err((e, _)) => {
                        log::error!("kafka send error to {}: {}", topic, e);
                    }
                }
            }
            log::info!("kafka producer task terminated");
        });

        Ok(Self { sender })
    }

    pub fn is_full(&self) -> bool {
        self.sender.capacity() < 1000
    }

    pub fn try_send(&self, topic: &'static str, key: &'static str, message: String) {
        if let Err(e) = self.sender.try_send((topic, key, message)) {
            log::error!("kafka channel full, message dropped: {}", e);
        }
    }
}
