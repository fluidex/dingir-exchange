use anyhow::{Result, format_err};
use futures::StreamExt;
use rdkafka::Message;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::error::KafkaError;
use rdkafka::message::BorrowedMessage;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub type PinBoxFuture<T> = Pin<Box<dyn std::future::Future<Output = T> + Send>>;

pub trait MessageHandler: Send + Sync {
    fn on_message<'a>(&self, msg: &'a BorrowedMessage<'a>, consumer: &'a StreamConsumer) -> PinBoxFuture<()>;
    fn on_no_msg<'a>(&self, consumer: &'a StreamConsumer) -> PinBoxFuture<()>;
}

/*A consumer which can handle mutiple topics*/
pub struct SimpleConsumer<'c> {
    consumer: &'c StreamConsumer,
    handlers: HashMap<String, Arc<dyn MessageHandler + 'c>>,
}

impl<'c> SimpleConsumer<'c> {
    pub fn new(consumer: &'c StreamConsumer) -> SimpleConsumer<'c> {
        SimpleConsumer {
            consumer,
            handlers: HashMap::new(),
        }
    }

    pub fn add_topic(self, topic: &str, handler: impl MessageHandler + 'c) -> Result<SimpleConsumer<'c>> {
        self.add_topic_boxed(topic, Arc::new(handler))
    }

    pub fn add_topic_boxed(mut self, topic: &str, handler: Arc<dyn MessageHandler + 'c>) -> Result<SimpleConsumer<'c>> {
        self.consumer
            .fetch_metadata(Some(topic), Duration::from_millis(2000u64))
            .map_err(|e| format_err!("kafka topic & health check: {}", e))?;

        self.handlers.insert(topic.to_string(), handler);
        Ok(self)
    }

    pub fn add_topic_config(self, cfg: &super::persist::TopicConfig) -> Result<SimpleConsumer<'c>> {
        self.add_topic_boxed(&cfg.topic, cfg.handler.clone())
    }

    pub async fn run_stream(self) -> KafkaError {
        let topic_list: Vec<&str> = self.handlers.iter().map(|(k, _)| k.as_str()).collect();

        if let Err(e) = self.consumer.subscribe(topic_list.as_slice()) {
            return e;
        }
        log::info!("start consuming topic {:?}", topic_list);
        let mut stream = self.consumer.stream();

        loop {
            match stream.next().await.expect("Kafka's stream has no EOF") {
                Err(KafkaError::NoMessageReceived) => {
                    let fs: Vec<PinBoxFuture<()>> = self.handlers.iter().map(|(_, h)| h.on_no_msg(self.consumer)).collect();
                    futures::future::join_all(fs).await;
                }
                Err(KafkaError::PartitionEOF(_)) => {} //simply omit this type of error
                Err(e) => {
                    return e;
                }
                Ok(m) => {
                    self.handlers
                        .get(m.topic())
                        .expect("kafka should not consumer message do not subscribed")
                        .on_message(&m, self.consumer)
                        .await;
                }
            }
        }
    }
}

use serde::Deserialize;

/// A handler that deserializes JSON into T and calls the inner handler function.
pub struct TypedHandler<T, H> {
    handler: H,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, H> TypedHandler<T, H> {
    pub fn new(handler: H) -> Self {
        Self {
            handler,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, H> MessageHandler for TypedHandler<T, H>
where
    T: for<'de> Deserialize<'de> + Send + Sync + std::fmt::Debug + 'static,
    H: Fn(&T, &BorrowedMessage<'_>) + Send + Sync,
{
    fn on_message<'a>(&self, msg: &'a BorrowedMessage<'a>, _consumer: &'a StreamConsumer) -> PinBoxFuture<()> {
        let handler = &self.handler;
        if let Some(payload) = msg.payload() {
            match String::from_utf8(payload.to_vec())
                .map_err(|e| format_err!("Decode kafka message fail: {}", e))
                .and_then(|json_str| {
                    serde_json::from_str::<T>(&json_str).map_err(|e| format_err!("Decode json fail: {}, payload: {}", e, json_str))
                }) {
                Ok(t) => {
                    log::debug!("{:?}", t);
                    handler(&t, msg);
                }
                Err(e) => {
                    log::error!("{}", e);
                }
            }
        } else {
            log::error!("Receive empty message");
        }
        Box::pin(async move {})
    }
    fn on_no_msg<'a>(&self, _consumer: &'a StreamConsumer) -> PinBoxFuture<()> {
        Box::pin(async move {})
    }
}

/// A simple handler that operates directly on the raw BorrowedMessage.
pub struct SimpleHandler<H> {
    handler: H,
}

impl<H> SimpleHandler<H> {
    pub fn new(handler: H) -> Self {
        Self { handler }
    }
}

impl<H> MessageHandler for SimpleHandler<H>
where
    H: Fn(&BorrowedMessage<'_>) + Send + Sync,
{
    fn on_message<'a>(&self, msg: &'a BorrowedMessage<'a>, _consumer: &'a StreamConsumer) -> PinBoxFuture<()> {
        (self.handler)(msg);
        Box::pin(async move {})
    }
    fn on_no_msg<'a>(&self, _consumer: &'a StreamConsumer) -> PinBoxFuture<()> {
        Box::pin(async move {})
    }
}
