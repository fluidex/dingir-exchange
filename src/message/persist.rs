use super::consumer::{self, RdConsumerExt, TypedMessageHandlerAsync, TypedMessageHandler, SyncTyped}; //crate::message::consumer
use crate::{database, models, types, utils};
use serde::Deserialize;
use std::cell::RefCell;
use std::marker::PhantomData;
use types::OrderSide;

use sqlx::migrate::Migrator;
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations/ts");

pub struct MsgDataPersistor<U: Clone + Send, UM> {
    pub writer: RefCell<database::DatabaseWriterEntry<U>>,
    pub _phantom: PhantomData<UM>,
}

impl<U: Clone + Send, UM> MsgDataPersistor<U, UM> {
    pub fn new(src: &database::DatabaseWriter<U>) -> SyncTyped<Self> {
        SyncTyped::from(Self::new_raw(src))
    }

    fn new_raw(src: &database::DatabaseWriter<U>) -> Self {
        MsgDataPersistor::<U, UM> {
            writer: RefCell::new(src.get_entry().unwrap()),
            _phantom: PhantomData,
        }
    }    
}

//An simple handler, just persist it by DatabaseWriter
impl<'c, C, U, UM> TypedMessageHandler<'c, C> for MsgDataPersistor<U, UM>
where
    UM: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send + Sync,
    for<'r> &'r UM: Into<U>,
    U: Clone + Send + Sync,
    C: RdConsumerExt + 'static,
{
    type DataType = UM;
    fn on_message(&self, msg: UM, _cr: &'c C::SelfType) {
        self.writer.borrow_mut().gen().append(Into::into(&msg)).ok();
    }
    fn on_no_msg(&self, _cr: &'c C::SelfType) {} //do nothing
}

//Try chainning handlers ...
pub struct EmptyHandler<U> {
    _phantom: PhantomData<U>,
}

impl<'c, C, UM> TypedMessageHandler<'c, C> for EmptyHandler<UM>
where
    C: RdConsumerExt + 'static,
    UM: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send + Sync,
{
    type DataType = UM;
    fn on_message(&self, _msg: UM, _cr: &'c C::SelfType) {}
    fn on_no_msg(&self, _cr: &'c C::SelfType) {}
}

pub struct ChainedHandler<T1, T2> (T1, T2);

impl<'c, C, U, UM, T> TypedMessageHandlerAsync<'c, C> for ChainedHandler<MsgDataPersistor<U, UM>, T> 
where
    C: RdConsumerExt + 'static,
    UM: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send + Sync,
    for<'r> &'r UM: Into<U>,
    U: Clone + Send + Sync,
    T: TypedMessageHandlerAsync<'c, C, DataType = UM> + 'static,
{
    type DataType = UM;
    fn on_message(&self, msg: UM, cr: &'c C::SelfType) 
        -> consumer::PinBox<dyn futures::Future<Output = ()> + Send>{
        self.0.writer.borrow_mut().gen().append(Into::into(&msg)).ok();
        self.1.on_message(msg, cr)
    }
    fn on_no_msg(&self, cr: &'c C::SelfType) -> consumer::PinBox<dyn futures::Future<Output = ()> + Send>
    {self.1.on_no_msg(cr)}    
}

//Config builder ...
pub trait TypedTopicConfig
{
//    type TypedHandlerType: for <'r> TypedMessageHandlerAsync<'r, C, DataType = Self::DataType> + for <'r> From<&'r Self> + 'static;
    fn topic_name(&self) -> &str;
}

pub trait FromTopicConfig<T>
{
    fn from_config<'r, C: RdConsumerExt>(cfg : &'r T) -> Self;
}

pub trait TypedTopicHandlerData<C: RdConsumerExt> : Sized
{
    type DataType : 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send + Sync;
    type HandlerType : for <'r> TypedMessageHandlerAsync<'r, C, DataType = Self::DataType> + FromTopicConfig<Self> + 'static;
}

pub struct TopicConfig<U> {
    topic: String,
    _phantom: PhantomData<U>,
}

impl<U> TypedTopicConfig for TopicConfig<U>
{
    fn topic_name(&self) -> &str {&self.topic}
}

impl<U> FromTopicConfig<TopicConfig<U>> for consumer::Synced<EmptyHandler<U>>
where 
    U: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send + Sync,
{
    fn from_config<'r, C: RdConsumerExt>(_origin : &'r TopicConfig<U>) -> Self
    {
        consumer::Synced::from(EmptyHandler{_phantom: PhantomData})
    }
}

impl<C, U> TypedTopicHandlerData<C>  for TopicConfig<U>
where 
    C: RdConsumerExt + 'static,
    U: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send + Sync,
{
    type DataType = U;
    type HandlerType = consumer::Synced<EmptyHandler<U>>;
}

pub struct ChainedTopicBuilder<'a, T, NXC> 
where 
    T: Clone + Send + Sync,
{
    next_config: NXC,
    dbwriter: &'a database::DatabaseWriter<T>,
}

impl<'a, T, NXC> TypedTopicConfig for ChainedTopicBuilder<'a, T, NXC>
where 
    T: Clone + Send + Sync,
    NXC: TypedTopicConfig + 'a,
{
    fn topic_name(&self) -> &str {&self.next_config.topic_name()}
}

impl<'a, T, U, NXC, T1> FromTopicConfig<ChainedTopicBuilder<'a, T, NXC>> for ChainedHandler<MsgDataPersistor<T, U>, T1>
where 
    U: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send + Sync,
    for<'x> &'x U: Into<T>,
    T: Clone + Send + Sync,
    T1: FromTopicConfig<NXC>,
{
    fn from_config<'r, C: RdConsumerExt>(origin : &'r ChainedTopicBuilder<'a, T, NXC>) -> Self
    {
        ChainedHandler(
            MsgDataPersistor::new_raw(origin.dbwriter),
            T1::from_config::<C>(&origin.next_config),
        )
    }
}

impl<'a, C, T, U, NXC> TypedTopicHandlerData<C> for ChainedTopicBuilder<'a, T, NXC>
where 
    C: RdConsumerExt + 'static,
    MsgDataPersistor<T, U>: 'static,
    U: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send + Sync,
    for<'x> &'x U: Into<T>,
    T: Clone + Send + Sync,
    NXC: TypedTopicHandlerData<C, DataType = U>,
{
    type DataType = U;
    type HandlerType = ChainedHandler<MsgDataPersistor<T, U>, NXC::HandlerType>;
}

impl<'a, C, T, U, NXC> consumer::TopicBuilder<C> for ChainedTopicBuilder<'a, T, NXC>
where 
    C: RdConsumerExt + 'static,
    MsgDataPersistor<T, U>: 'static,
    U: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send + Sync,
    for<'x> &'x U: Into<T>,
    T: Clone + Send + Sync,
    NXC: TypedTopicConfig + TypedTopicHandlerData<C, DataType = U> + 'a,
{
    type HandlerType = consumer::Typed<<Self as TypedTopicHandlerData<C>>::HandlerType>;
    fn topic_name(&self) -> &str {<Self as TypedTopicConfig>::topic_name(&self)}
    fn topic_handler(&self) -> Self::HandlerType{
        consumer::Typed::from(<<Self as TypedTopicHandlerData<C>>::HandlerType>::from_config::<C>(&self))
    }
}

impl<U> TopicConfig<U> 
{
    pub fn new(tpn :&str) -> Self
    {
        TopicConfig{
            topic: tpn.to_string(),
            _phantom: PhantomData,
        }
    }

    pub fn persist_to<'a, T: Clone + Send + Sync>(self, db : &'a database::DatabaseWriter<T>) -> ChainedTopicBuilder<'a, T, Self>
    {
        ChainedTopicBuilder::<T, Self>{
            next_config: self,
            dbwriter: db,
        }
    }
}

impl<'a, T, NXC> ChainedTopicBuilder<'a, T, NXC> 
where 
    T: Clone + Send + Sync,
    NXC: 'a,
{
    pub fn persist_to<'b : 'a, T1: Clone + Send + Sync>(self, db : &'b database::DatabaseWriter<T1>) -> ChainedTopicBuilder<'a, T1, Self>
    {
        ChainedTopicBuilder::<T1, Self>{
            next_config: self,
            dbwriter: db,
        }
    }
}

impl<'r> From<&'r super::Trade> for models::TradeRecord {
    fn from(origin: &'r super::Trade) -> models::TradeRecord {
        models::TradeRecord {
            time: utils::FTimestamp(origin.timestamp).into(),
            market: origin.market.clone(),
            trade_id: origin.id as i64,
            price: origin.price,
            amount: origin.amount,
            quote_amount: origin.quote_amount,
            taker_side: if origin.ask_order_id < origin.bid_order_id {
                OrderSide::BID
            } else {
                OrderSide::ASK
            },
        }
    }
}
