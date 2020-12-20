use anyhow::Result;
use futures::StreamExt;
use std::time::Duration;
// use std::sync::Arc;

use rdkafka::config::ClientConfig;
use rdkafka::consumer::Consumer;
use rdkafka::consumer::StreamConsumer;
use rdkafka::Message;

// use crate::config;
use crate::message::DEALS_TOPIC;
use crate::types::Trade;

fn init_kafka_fetcher(brokers: &str) -> Result<StreamConsumer> {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("group.id", "kline_data_fetcher")
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .create()?;
    consumer.subscribe(&[DEALS_TOPIC])?;
    // kafka server health check
    consumer.fetch_metadata(Some(DEALS_TOPIC), Duration::from_millis(2000u64))?;
    Ok(consumer)
}

pub struct KlineUpdater {}
impl KlineUpdater {
    pub async fn run(brokers: &str) {
        let consumer = match init_kafka_fetcher(brokers) {
            Err(e) => {
                log::error!("init_kafka_fetcher error: {}", e);
                return;
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
