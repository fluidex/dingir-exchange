use anyhow::{format_err, Result};
use futures::StreamExt;
use std::time::Duration;
// use std::sync::Arc;

use rdkafka::config::ClientConfig;
use rdkafka::consumer::*;
use rdkafka::message::BorrowedMessage;
use rdkafka::Message;
use rdkafka::error::KafkaError;

// use crate::config;
use crate::message::TRADES_TOPIC;
use crate::types::Trade;
use tonic::async_trait;
use std::collections::HashMap;
use std::pin::Pin;

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

#[async_trait]
pub trait MessageHandler<'c, C: Consumer + Sync> : Send + Sync
{
    async fn on_message(&self, msg : BorrowedMessage<'c>, cr :&'c C);
    async fn on_no_msg(&self, cr: &'c C);
}

type PinBox<T> = Pin<Box<T>>;

/*A consumer which can handle mutiple topics*/
pub struct SimpleConsumer<'c, C: Consumer> {
    consumer: &'c C,
    handlers : HashMap<String, PinBox<dyn MessageHandler<'c, C>>>,
}

impl<C: Consumer> SimpleConsumer<'_, C> {
    pub fn new(cr :&C) -> SimpleConsumer<C> {
        SimpleConsumer{
            consumer: cr,
            handlers: HashMap::new(),
        }
    }
}

impl<'c, C: Consumer + Sync> SimpleConsumer<'c, C> {

    pub fn add_topic(mut self, topic: &str, h : impl MessageHandler<'c, C> + 'static) -> Result<SimpleConsumer<'c, C>>{

        // kafka server health and topic check, fetch metadata 
        self.consumer.fetch_metadata(Some(topic), Duration::from_millis(2000u64))
        .map_err(|e| format_err!("kafka topic & health check: {}", e))?;

        self.handlers.insert(topic.to_string(), Box::pin(h));
        Ok(self)
    }

    pub async fn run_stream<CT, RT>(&self, f: impl Fn (&'c C)-> MessageStream<'c, CT, RT>) -> KafkaError
    where 
        CT: ConsumerContext + 'static,
        RT: rdkafka::util::AsyncRuntime,
    {
        let topic_list : Vec<&str> = self.handlers.iter().map(|(k, _)| k.as_str()).collect();

        if let Err(e) = self.consumer.subscribe(topic_list.as_slice()) {
            return e;
        }
        log::info!("start consuming topic {:?}", topic_list);
        let mut stream = f(self.consumer);

        loop {
            match stream.next().await.expect("Kafka's stream has no EOF") {
                Err(KafkaError::NoMessageReceived) => {
                    let fs : Vec<PinBox<dyn futures::Future<Output = ()> + Send>> 
                    = self.handlers.iter()
                        .map(|(_, h)| h.on_no_msg(self.consumer))
                        .collect();
                    futures::future::join_all(fs).await;
                },
                Err(KafkaError::PartitionEOF(_)) => {},//simply omit this type of error
                Err(e) => {
                    log::error!("Kafka error: {}", e);
                    return e;
                }
                Ok(m) => {
                    self.handlers.get(m.topic())
                        .expect("kafka should not consumer message do not subscribed")
                        .on_message(m, self.consumer).await;
                }                 
            }            
        }
    }      
}

use serde::Deserialize;

#[async_trait]
pub trait TypedMessageHandler<'c, C: Consumer + Sync> : Send + Sync
{
    type DataType : for <'de> Deserialize<'de> + 'static + std::fmt::Debug + Send;
    async fn on_message(&self, msg : Self::DataType, cr :&'c C);
    async fn on_no_msg(&self, cr: &'c C);
}

#[async_trait]
impl<'c, C, U> MessageHandler<'c, C> for U 
where 
    U : TypedMessageHandler<'c, C>,
    C: Consumer + Sync,
{
    async fn on_message(&self, msg : BorrowedMessage<'c>, cr :&'c C)
    {
        if let Some(pl) = msg.payload() {
            match String::from_utf8(pl.to_vec())
                .map_err(|e| format_err!("Decode kafka message fail: {}", e))
                .and_then(|json_str| serde_json::from_str::<U::DataType>(&json_str)
                    .map_err(|e| format_err!("Decode json fail: {}", e))
                )
            {
                Ok(t) => {
                    log::debug!("{:?}", t);
                    <Self as TypedMessageHandler<'c, C>>::on_message(&self, t, cr).await;
                },
                Err(e) => {
                    log::error!("{}", e);
                },
            }
        }

    }
    async fn on_no_msg(&self, cr: &'c C){
        <Self as TypedMessageHandler<'c, C>>::on_no_msg(&self, cr).await
    }
}