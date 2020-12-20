use anyhow::Result;
use futures::StreamExt;
use std::sync::Arc;

use rdkafka::config::ClientConfig;
use rdkafka::consumer::Consumer;
use rdkafka::consumer::StreamConsumer;
use rdkafka::Message;

use crate::config;
use crate::message::DEALS_TOPIC;
use crate::types::Trade;

pub struct KlineManager {
    msg_fetcher: Arc<StreamConsumer>,
}

impl KlineManager {
    // TODO: can we return self?
    pub fn new(settings: &config::Settings) -> Result<()> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &settings.brokers)
            .set("group.id", "kline_data_fetcher")
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .create()?;

        consumer.subscribe(&[DEALS_TOPIC])?;

        let mngr = KlineManager {
            msg_fetcher: Arc::new(consumer),
        };
        // TODO: can we tokio::spawn outside?
        tokio::spawn(async move {
            mngr.run().await;
        });
        Ok(())
    }

    async fn run(&self) {
        let mut stream = self.msg_fetcher.start();

        while let Some(message) = stream.next().await {
            match message {
                Err(e) => {
                    println!("Kafka error: {}", e);
                }
                Ok(m) => {
                    if let Some(p) = m.payload() {
                        let payload = String::from_utf8(p.to_vec()).unwrap();
                        let trade: Trade = serde_json::from_str(&payload).unwrap();
                        println!("{:?}", trade);
                        // TODO: insert into db
                    }
                }
            }
        }
    }
}
