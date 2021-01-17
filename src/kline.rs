use anyhow::{format_err, Result};
use futures::StreamExt;
use std::time::Duration;
// use std::sync::Arc;

use rdkafka::config::ClientConfig;
use rdkafka::consumer::Consumer;
use rdkafka::consumer::StreamConsumer;
use rdkafka::Message;

// use crate::config;
use crate::message::TRADES_TOPIC;
use crate::types::Trade;

fn init_kafka_fetcher(brokers: &str) -> Result<StreamConsumer> {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("group.id", "kline_data_fetcher")
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .create()?;
    consumer.subscribe(&[TRADES_TOPIC])?;

    // kafka server health check
    consumer
        .fetch_metadata(Some(TRADES_TOPIC), Duration::from_millis(2000u64))
        .map_err(|e| format_err!("kafka server health check: {}", e))?;

    Ok(consumer)
}

// TODO:
// use lifetime so that we can have
// KlineUpdater::new() -> Self
// and make stream as a member.
// Thus we can wrap KlineUpdater into controller::Controller and tokio::spawn in server.rs
pub struct KlineUpdater {}
impl KlineUpdater {
    pub async fn run(brokers: &str) {
        let consumer = match init_kafka_fetcher(brokers) {
            Err(e) => {
                log::error!("init_kafka_fetcher error: {}", e);
                std::process::exit(1);
            }
            consumer => consumer.unwrap(),
        };
        let mut stream = consumer.start();
        while let Some(message) = stream.next().await {
            match message {
                Err(e) => {
                    log::error!("Kafka error: {}", e);
                }
                Ok(m) => {
                    if let Some(p) = m.payload() {
                        let payload = String::from_utf8(p.to_vec()).unwrap();
                        let trade: Trade = serde_json::from_str(&payload).unwrap();
                        log::debug!("{:?}", trade);
                        // TODO: insert into db
                    }
                }
            }
        }
    }
}
