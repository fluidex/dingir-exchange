use anyhow::{format_err, Result};
use futures::StreamExt;
use std::time::Duration;
// use std::sync::Arc;

use fluidex_common::rdkafka;
use fluidex_common::rdkafka::consumer::*;
use fluidex_common::rdkafka::error::KafkaError;
use fluidex_common::rdkafka::message::BorrowedMessage;
use fluidex_common::rdkafka::Message;

// use crate::config;
use std::collections::HashMap;
use std::pin::Pin;

pub(crate) type PinBox<T> = Pin<Box<T>>;

pub trait RdConsumerExt {
    type CTXType: ConsumerContext;
    //So we can elimate the generic dep in trait bound ....
    type SelfType: Consumer<Self::CTXType> + Send + Sync;
    fn to_self(&self) -> &Self::SelfType;
}

//implied for consumer types current we have known in rdkafka, for more types, implied
//then with the new type idiom in your code
impl<C: ConsumerContext + 'static> RdConsumerExt for stream_consumer::StreamConsumer<C> {
    type CTXType = stream_consumer::StreamConsumerContext<C>;
    type SelfType = stream_consumer::StreamConsumer<C>;
    fn to_self(&self) -> &Self::SelfType {
        self
    }
}

impl<C: ConsumerContext> RdConsumerExt for base_consumer::BaseConsumer<C> {
    type CTXType = C;
    type SelfType = base_consumer::BaseConsumer<C>;
    fn to_self(&self) -> &Self::SelfType {
        self
    }
}

/*
    Notice this trait is not easy to be implied (self cannot be involved
    into the return futures, that is why I abondoned the async_trait macro)
    We should provide some trait which is better understood for users
*/
pub trait MessageHandlerAsync<'c, C: RdConsumerExt>: Send {
    fn on_message(&self, msg: &BorrowedMessage<'c>, cr: &'c C::SelfType) -> PinBox<dyn futures::Future<Output = ()> + Send>;
    fn on_no_msg(&self, cr: &'c C::SelfType) -> PinBox<dyn futures::Future<Output = ()> + Send>;
}

/*A consumer which can handle mutiple topics*/
pub struct SimpleConsumer<'c, C: RdConsumerExt> {
    consumer: &'c C::SelfType,
    handlers: HashMap<String, PinBox<dyn MessageHandlerAsync<'c, C> + 'c>>,
}

impl<C: RdConsumerExt> SimpleConsumer<'_, C> {
    pub fn new(cr: &C) -> SimpleConsumer<C> {
        SimpleConsumer {
            consumer: cr.to_self(),
            handlers: HashMap::new(),
        }
    }
}

pub trait TopicBuilder<C>
where
    C: RdConsumerExt,
{
    type HandlerType: for<'r> MessageHandlerAsync<'r, C> + 'static;
    fn topic_name(&self) -> &str;
    fn topic_handler(&self) -> Self::HandlerType;
}

impl<'c, C: RdConsumerExt> SimpleConsumer<'c, C> {
    pub fn add_topic<'a: 'c>(mut self, topic: &str, h: impl MessageHandlerAsync<'c, C> + 'a) -> Result<SimpleConsumer<'c, C>> {
        // kafka server health and topic check, fetch metadata
        self.consumer
            .fetch_metadata(Some(topic), Duration::from_millis(2000u64))
            .map_err(|e| format_err!("kafka topic & health check: {}", e))?;

        self.handlers.insert(topic.to_string(), Box::pin(h));
        Ok(self)
    }

    pub fn add_topic_config<'a, CF>(self, builder: &'a CF) -> Result<SimpleConsumer<'c, C>>
    where
        CF: TopicBuilder<C>,
    {
        self.add_topic(builder.topic_name(), builder.topic_handler())
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
                        .on_message(&m, self.consumer)
                        .await;
                }
            }
        }
    }
}

use serde::Deserialize;

pub trait TypedMessageHandlerAsync<'c, C: RdConsumerExt>: Send {
    type DataType: for<'de> Deserialize<'de> + 'static + std::fmt::Debug + Send;
    fn on_message(
        &self,
        msg: &Self::DataType,
        origin_msg: &BorrowedMessage<'c>,
        cr: &'c C::SelfType,
    ) -> PinBox<dyn futures::Future<Output = ()> + Send>;
    fn on_no_msg(&self, cr: &'c C::SelfType) -> PinBox<dyn futures::Future<Output = ()> + Send>;
}

pub struct Typed<U>(U);

impl<'c, C, U> MessageHandlerAsync<'c, C> for Typed<U>
where
    U: TypedMessageHandlerAsync<'c, C>,
    C: RdConsumerExt + 'static,
{
    fn on_message(&self, msg: &BorrowedMessage<'c>, cr: &'c C::SelfType) -> PinBox<dyn futures::Future<Output = ()> + Send> {
        if let Some(pl) = msg.payload() {
            match String::from_utf8(pl.to_vec())
                .map_err(|e| format_err!("Decode kafka message fail: {}", e))
                .and_then(|json_str| {
                    serde_json::from_str::<U::DataType>(&json_str)
                        .map_err(|e| format_err!("Decode json fail: {}, payload: {}", e, json_str))
                }) {
                Ok(t) => {
                    log::debug!("{:?}", t);
                    U::on_message(&self.0, &t, msg, cr)
                }
                Err(e) => {
                    log::error!("{}", e);
                    Box::pin(async {})
                }
            }
        } else {
            log::error!("Receive empty message");
            Box::pin(async {})
        }
    }
    fn on_no_msg(&self, cr: &'c C::SelfType) -> PinBox<dyn futures::Future<Output = ()> + Send> {
        U::on_no_msg(&self.0, cr)
    }
}

pub trait TypedMessageHandler<'c, C: RdConsumerExt>: Send {
    type DataType: for<'de> Deserialize<'de> + 'static + std::fmt::Debug + Send;
    fn on_message(&self, msg: &Self::DataType, origin_msg: &BorrowedMessage<'c>, cr: &'c C::SelfType);
    fn on_no_msg(&self, cr: &'c C::SelfType);
}

pub struct Synced<U>(U);

impl<'c, C: RdConsumerExt, U: TypedMessageHandler<'c, C>> TypedMessageHandlerAsync<'c, C> for Synced<U> {
    type DataType = U::DataType;

    fn on_message(
        &self,
        msg: &U::DataType,
        origin_msg: &BorrowedMessage<'c>,
        cr: &'c C::SelfType,
    ) -> PinBox<dyn futures::Future<Output = ()> + Send> {
        U::on_message(&self.0, msg, origin_msg, cr);
        Box::pin(async {})
    }
    fn on_no_msg(&self, cr: &'c C::SelfType) -> PinBox<dyn futures::Future<Output = ()> + Send> {
        U::on_no_msg(&self.0, cr);
        Box::pin(async {})
    }
}

impl<U> From<U> for Typed<U> {
    fn from(t: U) -> Self {
        Typed(t)
    }
}
impl<U> From<U> for Synced<U> {
    fn from(t: U) -> Self {
        Synced(t)
    }
}

pub type SyncTyped<U> = Typed<Synced<U>>;
impl<U> From<U> for SyncTyped<U> {
    fn from(t: U) -> Self {
        Typed::from(Synced::from(t))
    }
}

pub trait SimpleMessageHandler: Send {
    fn on_message(&self, msg: &BorrowedMessage<'_>);
    fn on_no_msg(&self) {}
}

pub struct Simple<U>(U);

impl<U> From<U> for Simple<U> {
    fn from(t: U) -> Self {
        Simple(t)
    }
}

impl<'c, C: RdConsumerExt, U: SimpleMessageHandler> MessageHandlerAsync<'c, C> for Simple<U> {
    fn on_message(&self, msg: &BorrowedMessage<'c>, _cr: &'c C::SelfType) -> PinBox<dyn futures::Future<Output = ()> + Send> {
        self.0.on_message(msg);
        Box::pin(async {})
    }
    fn on_no_msg(&self, _cr: &'c C::SelfType) -> PinBox<dyn futures::Future<Output = ()> + Send> {
        self.0.on_no_msg();
        Box::pin(async {})
    }
}
