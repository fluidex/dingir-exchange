use anyhow::Result;
use std::sync::Arc;

use rdkafka::config::ClientConfig;
use rdkafka::consumer::Consumer;
use rdkafka::consumer::StreamConsumer;

use crate::config;
use crate::message::DEALS_TOPIC;

pub struct KlineManager {
    msg_fetcher: Arc<StreamConsumer>,
}

impl KlineManager {
    pub fn new(settings: &config::Settings) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &settings.brokers)
            .set("group.id", "kline_data_fetcher")
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .create()?;

        consumer.subscribe(&[DEALS_TOPIC])?;

        Ok(KlineManager {
            msg_fetcher: Arc::new(consumer),
        })
    }

    pub fn run(&self) {}
}
