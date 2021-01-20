use anyhow::{format_err, Result};
use futures::StreamExt;
use std::time::Duration;
// use std::sync::Arc;

use rdkafka::consumer::*;
use rdkafka::error::KafkaError;
use rdkafka::message::BorrowedMessage;
use rdkafka::Message;

// use crate::config;
use std::collections::HashMap;
use std::pin::Pin;
use tonic::async_trait;

pub trait RdConsumerExt {
    type CTXType: ConsumerContext;
    //So we can elimate the generic dep in trait bound ....
    type SelfType: Consumer<Self::CTXType> + Sync;
    fn to_self(&self) -> &Self::SelfType;
}

#[async_trait]
pub trait MessageHandler<'c, C: RdConsumerExt>: Send + Sync {
    async fn on_message(&self, msg: BorrowedMessage<'c>, cr: &'c C::SelfType);
    async fn on_no_msg(&self, cr: &'c C::SelfType);
}

type PinBox<T> = Pin<Box<T>>;

/*A consumer which can handle mutiple topics*/
pub struct SimpleConsumer<'c, C: RdConsumerExt> {
    consumer: &'c C::SelfType,
    handlers: HashMap<String, PinBox<dyn MessageHandler<'c, C> + 'c>>,
}
/*
impl<C: RdConsumerExt> SimpleConsumer<'_, C> {
    pub fn new(cr :&C) -> SimpleConsumer<C> {
        SimpleConsumer{
            consumer: cr.to_self(),
            handlers: HashMap::new(),
        }
    }
}*/

impl<'c, C: RdConsumerExt> SimpleConsumer<'c, C> {
    pub fn new(cr: &'c C) -> SimpleConsumer<'c, C> {
        SimpleConsumer {
            consumer: cr.to_self(),
            handlers: HashMap::new(),
        }
    }

    pub fn add_topic<'a: 'c>(mut self, topic: &str, h: impl MessageHandler<'c, C> + 'a) -> Result<SimpleConsumer<'c, C>> {
        // kafka server health and topic check, fetch metadata
        self.consumer
            .fetch_metadata(Some(topic), Duration::from_millis(2000u64))
            .map_err(|e| format_err!("kafka topic & health check: {}", e))?;

        self.handlers.insert(topic.to_string(), Box::pin(h));
        Ok(self)
    }

    pub async fn run_stream<CT, RT>(&self, f: impl Fn(&'c C::SelfType) -> MessageStream<'c, CT, RT>) -> KafkaError
    where
        CT: ConsumerContext + 'static,
        RT: rdkafka::util::AsyncRuntime,
    {
        let topic_list: Vec<&str> = self.handlers.iter().map(|(k, _)| k.as_str()).collect();

        if let Err(e) = self.consumer.subscribe(topic_list.as_slice()) {
            return e;
        }
        log::info!("start consuming topic {:?}", topic_list);
        let mut stream = f(self.consumer);

        loop {
            match stream.next().await.expect("Kafka's stream has no EOF") {
                Err(KafkaError::NoMessageReceived) => {
                    let fs: Vec<PinBox<dyn futures::Future<Output = ()> + Send>> =
                        self.handlers.iter().map(|(_, h)| h.on_no_msg(self.consumer)).collect();
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
                        .on_message(m, self.consumer)
                        .await;
                }
            }
        }
    }
}

use serde::Deserialize;

#[async_trait]
pub trait TypedMessageHandler<'c, C: RdConsumerExt>: Send + Sync {
    type DataType: for<'de> Deserialize<'de> + 'static + std::fmt::Debug + Send;
    async fn on_message(&self, msg: Self::DataType, cr: &'c C::SelfType);
    async fn on_no_msg(&self, cr: &'c C::SelfType);
}

#[async_trait]
impl<'c, C, U> MessageHandler<'c, C> for U
where
    U: TypedMessageHandler<'c, C>,
    C: RdConsumerExt + 'static,
{
    async fn on_message(&self, msg: BorrowedMessage<'c>, cr: &'c C::SelfType) {
        if let Some(pl) = msg.payload() {
            match String::from_utf8(pl.to_vec())
                .map_err(|e| format_err!("Decode kafka message fail: {}", e))
                .and_then(|json_str| serde_json::from_str::<U::DataType>(&json_str).map_err(|e| format_err!("Decode json fail: {}", e)))
            {
                Ok(t) => {
                    log::debug!("{:?}", t);
                    <Self as TypedMessageHandler<'c, C>>::on_message(&self, t, cr).await;
                }
                Err(e) => {
                    log::error!("{}", e);
                }
            }
        }
    }
    async fn on_no_msg(&self, cr: &'c C::SelfType) {
        <Self as TypedMessageHandler<'c, C>>::on_no_msg(&self, cr).await
    }
}
