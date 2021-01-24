use super::consumer::{self, RdConsumerExt, TypedMessageHandlerAsync, TypedMessageHandler, SyncTyped}; //crate::message::consumer
use crate::{database, models, types, utils};
use serde::Deserialize;
use std::cell::RefCell;
use std::marker::PhantomData;
use types::OrderSide;

use sqlx::migrate::Migrator;
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations/ts");

pub struct MsgDataPersistor<T: Clone + Send, UM = ()> {
    pub writer: RefCell<database::DatabaseWriterEntry<T>>,
    pub _phantom: PhantomData<UM>,
}

pub trait MsgDataTransformer<T: Clone + Send> : Send {
    type MsgType: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send;
    fn into<'r>(msg : &'r Self::MsgType) -> T;
}

impl<T, UM> MsgDataPersistor<T, UM>
where
    T: Clone + Send,
    UM: MsgDataTransformer<T>,
{
    pub fn write_in<'r>(&self, msg : &'r UM::MsgType) 
    {
        self.writer.borrow_mut().gen().append(UM::into(msg)).ok();
    }    
}

pub struct Deco<UM> (PhantomData<UM>);

impl<T, UM> MsgDataTransformer<T> for Deco<UM>
where
    T: Clone + Send,
    UM: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send,
    for<'r> &'r UM: Into<T>,
{
    type MsgType = UM;
    fn into<'r>(msg : &'r Self::MsgType) -> T {Into::into(msg)}    
}

impl<T: Clone + Send> MsgDataPersistor<T, ()> {

    pub fn new(src: &database::DatabaseWriter<T>) -> Self {
        MsgDataPersistor {
            writer: RefCell::new(src.get_entry().unwrap()),
            _phantom: PhantomData,
        }
    }

    fn set_transformer<UM> (self)-> MsgDataPersistor<T, UM> 
    {
        MsgDataPersistor {
            writer: self.writer,
            _phantom: PhantomData,
        }        
    }

    pub fn handle_message<UM> (self)-> SyncTyped<MsgDataPersistor<T, Deco<UM>>>
    where UM: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send,
    {
        SyncTyped::from(self.set_transformer())
    }
}

//An simple handler, just persist it by DatabaseWriter
impl<'c, C, T, UM> TypedMessageHandler<'c, C> for MsgDataPersistor<T, Deco<UM>>
where
    UM: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send,
    for<'r> &'r UM: Into<T>,
    T: Clone + Send,
    C: RdConsumerExt + 'static,
{
    type DataType = UM;
    fn on_message(&self, msg: UM, _cr: &'c C::SelfType) {
        self.write_in(&msg);
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
    UM: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send,
{
    type DataType = UM;
    fn on_message(&self, _msg: UM, _cr: &'c C::SelfType) {}
    fn on_no_msg(&self, _cr: &'c C::SelfType) {}
}

pub struct ChainedHandler<T1, T2> (T1, T2);

impl<'c, C, U, UM, UT, T> TypedMessageHandlerAsync<'c, C> for ChainedHandler<MsgDataPersistor<U, UT>, T> 
where
    C: RdConsumerExt + 'static,
    UM: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send,
    UT: MsgDataTransformer<U, MsgType = UM>,
    U: Clone + Send,
    T: TypedMessageHandlerAsync<'c, C, DataType = UM> + 'static,
{
    type DataType = UM;
    fn on_message(&self, msg: UM, cr: &'c C::SelfType) 
        -> consumer::PinBox<dyn futures::Future<Output = ()> + Send>{
        self.0.write_in(&msg);
        self.1.on_message(msg, cr)
    }
    fn on_no_msg(&self, cr: &'c C::SelfType) -> consumer::PinBox<dyn futures::Future<Output = ()> + Send>
    {self.1.on_no_msg(cr)}    
}

//Config builder ...
pub trait TypedTopicConfig
{
    type BaseMsgType;
    fn topic_name(&self) -> &str;
}

pub trait FromTopicConfig<T>
{
    fn from_config<'r, C: RdConsumerExt>(cfg : &'r T) -> Self;
}

pub trait TypedTopicHandlerData<C: RdConsumerExt> : Sized
{
    type DataType : 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send;
    type HandlerType : for <'r> TypedMessageHandlerAsync<'r, C, DataType = Self::DataType> + FromTopicConfig<Self> + 'static;
}

pub struct TopicConfig<U> {
    topic: String,
    _phantom: PhantomData<U>,
}

impl<U> TypedTopicConfig for TopicConfig<U>
{
    type BaseMsgType = U;
    fn topic_name(&self) -> &str {&self.topic}
}

impl<U> FromTopicConfig<TopicConfig<U>> for consumer::Synced<EmptyHandler<U>>
where 
    U: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send,
{
    fn from_config<'r, C: RdConsumerExt>(_origin : &'r TopicConfig<U>) -> Self
    {
        consumer::Synced::from(EmptyHandler{_phantom: PhantomData})
    }
}

impl<C, U> TypedTopicHandlerData<C>  for TopicConfig<U>
where 
    C: RdConsumerExt + 'static,
    U: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send,
{
    type DataType = U;
    type HandlerType = consumer::Synced<EmptyHandler<U>>;
}

pub struct ChainedTopicBuilder<'a, T, UT, NXC> 
where 
    T: Clone + Send,
{
    next_config: NXC,
    dbwriter: &'a database::DatabaseWriter<T>,
    _phantom: PhantomData<UT>,
}

impl<'a, T, UT, NXC> TypedTopicConfig for ChainedTopicBuilder<'a, T, UT, NXC>
where 
    T: Clone + Send,
    NXC: TypedTopicConfig + 'a,
{
    type BaseMsgType = NXC::BaseMsgType;
    fn topic_name(&self) -> &str {&self.next_config.topic_name()}
}

impl<'a, T, UT, NXC, T1> FromTopicConfig<ChainedTopicBuilder<'a, T, UT, NXC>> for ChainedHandler<MsgDataPersistor<T, UT>, T1>
where 
    T: Clone + Send,
    T1: FromTopicConfig<NXC>,
{
    fn from_config<'r, C: RdConsumerExt>(origin : &'r ChainedTopicBuilder<'a, T, UT, NXC>) -> Self
    {
        ChainedHandler(
            MsgDataPersistor::new(origin.dbwriter).set_transformer(),
            T1::from_config::<C>(&origin.next_config),
        )
    }
}

impl<'a, C, T, U, UT, NXC> TypedTopicHandlerData<C> for ChainedTopicBuilder<'a, T, UT, NXC>
where 
    C: RdConsumerExt + 'static,
    MsgDataPersistor<T, UT>: 'static,
    U: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send,
    UT: MsgDataTransformer<T, MsgType = U>,
    T: Clone + Send,
    NXC: TypedTopicHandlerData<C, DataType = U>,
{
    type DataType = U;
    type HandlerType = ChainedHandler<MsgDataPersistor<T, UT>, NXC::HandlerType>;
}

impl<'a, C, T, U, UT, NXC> consumer::TopicBuilder<C> for ChainedTopicBuilder<'a, T, UT, NXC>
where 
    C: RdConsumerExt + 'static,
    MsgDataPersistor<T, UT>: 'static,
    U: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send,
    UT: MsgDataTransformer<T, MsgType = U>,
    T: Clone + Send,
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

    pub fn persist_to<'a, T: Clone + Send + Sync>(self, db : &'a database::DatabaseWriter<T>) -> ChainedTopicBuilder<'a, T, Deco<U>, Self>
    {
        ChainedTopicBuilder{
            next_config: self,
            dbwriter: db,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T, UT, NXC> ChainedTopicBuilder<'a, T, UT, NXC> 
where 
    T: Clone + Send + Sync,
    NXC: TypedTopicConfig + 'a,
{
    pub fn persist_to<'b : 'a, T1: Clone + Send + Sync>(self, db : &'b database::DatabaseWriter<T1>) 
        -> ChainedTopicBuilder<'a, T1, Deco<NXC::BaseMsgType>, Self>
    {
        ChainedTopicBuilder{
            next_config: self,
            dbwriter: db,
            _phantom: PhantomData,
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
