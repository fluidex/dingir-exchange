#[derive(Debug)]
pub struct KlineManager {}

impl KlineManager {
    pub fn new() -> Self {
        KlineManager {}
    }
}



// pub struct SimpleConsumerContext;
// // TODO: impl ClientContext for SimpleConsumerContext {}
// impl ConsumerContext for SimpleConsumerContext {
//     // TODO:
// }

// // TODO: should we embed it into sender?
// pub struct KafkaMessageFetcher {}

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
