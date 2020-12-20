use anyhow::Result;
use std::sync::Arc;

use rdkafka::config::ClientConfig;

use crate::config;

pub struct KlineManager {
    msgFetcher: Arc<KafkaMessageFetcher>,
}

impl KlineManager {
    pub fn new(settings: &config::Settings) -> Result<Self> {
        let consumer = ClientConfig::new()
            .set("bootstrap.servers", &settings.brokers)
            // .set("queue.buffering.max.ms", "1")
            .create()?;
        let arc = Arc::new(consumer);

        Ok(KlineManager { msgFetcher: arc })
    }
}

// pub struct SimpleConsumerContext;
// // TODO: impl ClientContext for SimpleConsumerContext {}
// impl ConsumerContext for SimpleConsumerContext {
//     // TODO:
// }

struct KafkaMessageFetcher {}

// impl KafkaMessageFetcher {
//     pub fn new(brokers: &str) -> Result<KafkaMessageFetcher> {
//         let consumer = ClientConfig::new()
//             .set("bootstrap.servers", brokers)
//             .set("queue.buffering.max.ms", "1")
//             .create_with_context(SimpleConsumerContext)?;
//         let arc = Arc::new(consumer);

//         Ok(KafkaMessageSender { consumer: arc })
//     }
// }
