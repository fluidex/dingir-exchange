use super::consumer::{self, RdConsumerExt, SyncTyped, TypedMessageHandler, TypedMessageHandlerAsync}; //crate::message::consumer
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

pub trait MsgDataTransformer<T: Clone + Send>: Send {
    type MsgType: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send;
    fn into(msg: &Self::MsgType) -> Option<T>;
}

use fluidex_common::rdkafka::{self, message::BorrowedMessage, Message};

impl<'c, C, T, UM> TypedMessageHandler<'c, C> for MsgDataPersistor<T, UM>
where
    UM: MsgDataTransformer<T>,
    T: Clone + Send,
    C: RdConsumerExt + 'static,
{
    type DataType = UM::MsgType;
    fn on_message(&self, msg_origin: &Self::DataType, origin_msg: &BorrowedMessage<'c>, _cr: &'c C::SelfType) {
        if let Some(msg) = UM::into(msg_origin) {
            let notify = database::TaskNotification::new(origin_msg.partition(), origin_msg.offset() as u64);
            self.writer.borrow_mut().gen().append_with_notify(msg, Some(notify)).ok();
        }
    }
    fn on_no_msg(&self, _cr: &'c C::SelfType) {} //do nothing
}

pub struct Deco<UM>(PhantomData<UM>);

impl<T, UM> MsgDataTransformer<T> for Deco<UM>
where
    T: Clone + Send,
    UM: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send,
    for<'r> &'r UM: Into<T>,
{
    type MsgType = UM;
    fn into(msg: &Self::MsgType) -> Option<T> {
        Some(Into::into(msg))
    }
}

impl<T: Clone + Send> MsgDataPersistor<T, ()> {
    pub fn new(src: &database::DatabaseWriter<T>) -> Self {
        MsgDataPersistor {
            writer: RefCell::new(src.get_entry().unwrap()),
            _phantom: PhantomData,
        }
    }

    fn set_transformer<UM>(self) -> MsgDataPersistor<T, UM> {
        MsgDataPersistor {
            writer: self.writer,
            _phantom: PhantomData,
        }
    }

    pub fn handle_message<UM>(self) -> SyncTyped<MsgDataPersistor<T, Deco<UM>>>
    where
        UM: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send,
    {
        SyncTyped::from(self.set_transformer())
    }
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
    fn on_message(&self, _msg: &UM, _origin_msg: &BorrowedMessage<'c>, _cr: &'c C::SelfType) {}
    fn on_no_msg(&self, _cr: &'c C::SelfType) {}
}

pub struct ChainedHandler<T1, T2>(T1, T2);

impl<'c, C, UM, T1, T2> TypedMessageHandlerAsync<'c, C> for ChainedHandler<T1, T2>
where
    C: RdConsumerExt + 'static,
    UM: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send,
    T1: TypedMessageHandlerAsync<'c, C, DataType = UM> + 'static,
    T2: TypedMessageHandlerAsync<'c, C, DataType = UM> + 'static,
{
    type DataType = UM;
    fn on_message(
        &self,
        msg: &UM,
        origin_msg: &BorrowedMessage<'c>,
        cr: &'c C::SelfType,
    ) -> consumer::PinBox<dyn futures::Future<Output = ()> + Send> {
        let f0 = self.0.on_message(msg, origin_msg, cr);
        let f1 = self.1.on_message(msg, origin_msg, cr);
        std::boxed::Box::pin(async move {
            f0.await;
            f1.await;
        })
    }
    fn on_no_msg(&self, cr: &'c C::SelfType) -> consumer::PinBox<dyn futures::Future<Output = ()> + Send> {
        let f0 = self.0.on_no_msg(cr);
        let f1 = self.1.on_no_msg(cr);
        std::boxed::Box::pin(async move {
            f0.await;
            f1.await;
        })
    }
}

//Config builder ...
pub trait TypedTopicConfig {
    type BaseMsgType;
    fn topic_name(&self) -> &str;
}

pub trait TypedTopicHandlerData<C: RdConsumerExt>: Sized {
    type DataType: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send;
    type HandlerType: for<'r> TypedMessageHandlerAsync<'r, C, DataType = Self::DataType> + for<'r> From<&'r Self> + 'static;
}

pub struct TopicConfig<U> {
    topic: String,
    _phantom: PhantomData<U>,
}

impl<U> TypedTopicConfig for TopicConfig<U> {
    type BaseMsgType = U;
    fn topic_name(&self) -> &str {
        &self.topic
    }
}

impl<U> From<&TopicConfig<U>> for consumer::Synced<EmptyHandler<U>>
where
    U: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send,
{
    fn from(_origin: &TopicConfig<U>) -> Self {
        consumer::Synced::from(EmptyHandler { _phantom: PhantomData })
    }
}

impl<C, U> TypedTopicHandlerData<C> for TopicConfig<U>
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
    fn topic_name(&self) -> &str {
        &self.next_config.topic_name()
    }
}

impl<'a, T, UT, NXC, T1> From<&ChainedTopicBuilder<'a, T, UT, NXC>> for ChainedHandler<consumer::Synced<MsgDataPersistor<T, UT>>, T1>
where
    T: Clone + Send,
    T1: for<'r> From<&'r NXC>,
{
    fn from(origin: &ChainedTopicBuilder<'a, T, UT, NXC>) -> Self {
        ChainedHandler(
            MsgDataPersistor::new(origin.dbwriter).set_transformer().into(),
            T1::from(&origin.next_config),
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
    type HandlerType = ChainedHandler<consumer::Synced<MsgDataPersistor<T, UT>>, NXC::HandlerType>;
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
    fn topic_name(&self) -> &str {
        <Self as TypedTopicConfig>::topic_name(&self)
    }
    fn topic_handler(&self) -> Self::HandlerType {
        consumer::Typed::from(<<Self as TypedTopicHandlerData<C>>::HandlerType>::from(self))
    }
}

impl<U> TopicConfig<U> {
    pub fn new(tpn: &str) -> Self {
        TopicConfig {
            topic: tpn.to_string(),
            _phantom: PhantomData,
        }
    }

    pub fn persist_to<T: Clone + Send>(self, db: &database::DatabaseWriter<T>) -> ChainedTopicBuilder<'_, T, Deco<U>, Self> {
        ChainedTopicBuilder {
            next_config: self,
            dbwriter: db,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T, UT, NXC> ChainedTopicBuilder<'a, T, UT, NXC>
where
    T: Clone + Send,
    NXC: TypedTopicConfig + 'a,
{
    pub fn persist_to<'b: 'a, T1: Clone + Send>(
        self,
        db: &'b database::DatabaseWriter<T1>,
    ) -> ChainedTopicBuilder<'a, T1, Deco<NXC::BaseMsgType>, Self> {
        ChainedTopicBuilder {
            next_config: self,
            dbwriter: db,
            _phantom: PhantomData,
        }
    }

    pub fn with_tr<UT1>(self) -> ChainedTopicBuilder<'a, T, UT1, NXC> {
        ChainedTopicBuilder {
            next_config: self.next_config,
            dbwriter: self.dbwriter,
            _phantom: PhantomData,
        }
    }

    /*    pub fn auto_commit<'r, 'C : RdConsumerExt + 'static + Sync>(&self, cr : &'r C) {
        //self.dbwriter
    }*/
}

#[derive(Debug, Clone)]
pub enum NotifyTrackItem {
    Left(u64),
    Right(u64),
}

impl NotifyTrackItem {
    fn is_left(&self) -> bool {
        match self {
            NotifyTrackItem::Left(_) => true,
            NotifyTrackItem::Right(_) => false,
        }
    }
    fn is_right(&self) -> bool {
        !self.is_left()
    }

    fn val(&self) -> u64 {
        match self {
            NotifyTrackItem::Left(v) => *v,
            NotifyTrackItem::Right(v) => *v,
        }
    }

    fn val_into(self) -> u64 {
        match self {
            NotifyTrackItem::Left(v) => v,
            NotifyTrackItem::Right(v) => v,
        }
    }

    fn resolve(&mut self, another: NotifyTrackItem) -> u64 {
        let self_v = self.val();
        let another_v = another.val();

        if self_v < another_v {
            *self = another;
            self_v
        } else {
            another_v
        }
    }

    //demo: https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=b62ca543775ab42eaa79a76a0d29dd22
    fn merge(&mut self, another: NotifyTrackItem) -> Option<u64> {
        match self {
            NotifyTrackItem::Left(v) => {
                if another.is_left() {
                    *v = std::cmp::max(*v, another.val());
                    None
                } else {
                    Some(self.resolve(another))
                }
            }
            NotifyTrackItem::Right(v) => {
                if another.is_right() {
                    *v = std::cmp::max(*v, another.val());
                    None
                } else {
                    Some(self.resolve(another))
                }
            }
        }
    }
}

use database::TaskNotifyFlag;
use std::collections::HashMap;

pub struct NotifyTracker(HashMap<i32, NotifyTrackItem>);

impl std::ops::Deref for NotifyTracker {
    type Target = HashMap<i32, NotifyTrackItem>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for NotifyTracker {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl NotifyTracker {
    fn map_to(input: &TaskNotifyFlag, mf: fn(u64) -> NotifyTrackItem) -> NotifyTracker {
        NotifyTracker(
            input
                .iter()
                .map(|item| {
                    let (k, v) = item;
                    (*k, mf(*v))
                })
                .collect(),
        )
    }

    fn merge(&mut self, another: NotifyTracker) -> TaskNotifyFlag {
        another
            .0
            .into_iter()
            .filter_map(|item| {
                let (k, v) = item;
                self.entry(k).or_insert_with(|| v.clone()).merge(v).map(|u| (k, u))
            })
            .collect()
    }
}

//stack-base recuisive tracker receiver
pub struct NotifyTrackerReceiver {
    status: NotifyTracker,
    listener: tokio::sync::watch::Receiver<TaskNotifyFlag>,
    next: Option<Box<NotifyTrackerReceiver>>,
}

impl NotifyTrackerReceiver {
    fn final_status(self) -> TaskNotifyFlag {
        self.status
            .0
            .into_iter()
            .map(|item| {
                let (k, v) = item;
                (k, v.val_into())
            })
            .collect()
    }

    fn changed(&mut self) -> consumer::PinBox<dyn futures::Future<Output = Option<TaskNotifyFlag>> + Send + '_> {
        let listener = &mut self.listener;
        let status = &mut self.status;
        if let Some(next_iter) = self.next.as_mut() {
            let async_block = async move {
                loop {
                    tokio::select! {
                        Ok(_) = listener.changed() => {
                            let ret = status.merge(NotifyTracker::map_to(&listener.borrow(), NotifyTrackItem::Left));
                            if !ret.is_empty() {
                                return Some(ret);
                            }
                        }
                        may_nc = next_iter.changed() => {
                            if let Some(nc) = may_nc {
                                let ret = status.merge(NotifyTracker::map_to(&nc, NotifyTrackItem::Right));
                                if !ret.is_empty() {
                                    return Some(ret);
                                }
                            }else {
                                //we die from botton
                                return None
                            }

                        }
                    }
                }
            };
            Box::pin(async_block)
        } else {
            Box::pin(async move {
                match listener.changed().await {
                    Ok(_) => {
                        let ret = listener.borrow();
                        status.merge(NotifyTracker::map_to(&ret, NotifyTrackItem::Left));
                        Some(ret.clone())
                    }
                    _ => None,
                }
            })
        }
    }
}

pub trait HandleWriterNotify {
    fn get_tracker(&self) -> Option<NotifyTrackerReceiver>;
}

impl<U> HandleWriterNotify for TopicConfig<U> {
    fn get_tracker(&self) -> Option<NotifyTrackerReceiver> {
        None
    }
}

impl<'a, T, UT, NXC> HandleWriterNotify for ChainedTopicBuilder<'a, T, UT, NXC>
where
    T: Clone + Send,
    NXC: HandleWriterNotify + 'a,
{
    fn get_tracker(&self) -> Option<NotifyTrackerReceiver> {
        Some(NotifyTrackerReceiver {
            status: NotifyTracker(HashMap::new()),
            listener: self.dbwriter.listen_notify(),
            next: self.next_config.get_tracker().map(Box::new),
        })
    }
}

use fluidex_common::rdkafka::consumer::Consumer;
use fluidex_common::rdkafka::topic_partition_list::{Offset, TopicPartitionList};

pub struct AutoCommitRet(tokio::task::JoinHandle<TaskNotifyFlag>, String, tokio::sync::oneshot::Sender<()>);

impl<'a, T, UT, NXC> ChainedTopicBuilder<'a, T, UT, NXC>
where
    T: Clone + Send,
    NXC: HandleWriterNotify + TypedTopicConfig + 'a,
{
    pub fn auto_commit_start<C>(&self, cr: std::sync::Arc<C>) -> AutoCommitRet
    where
        C: RdConsumerExt + Send + Sync + 'static,
    {
        let mut receiver = HandleWriterNotify::get_tracker(self).expect("should ensure it");
        let topic_name = TypedTopicConfig::topic_name(self).to_string();
        let (tx, mut rx) = tokio::sync::oneshot::channel();

        AutoCommitRet(
            tokio::spawn(async move {
                log::info!("start auto commiting for topic {}", topic_name);
                let cr = cr.to_self();
                loop {
                    tokio::select! {
                        may_notify = receiver.changed() => {
                            if let Some(notify) = may_notify {

                                let mut tplist = TopicPartitionList::new();

                                for (k, v) in notify.into_iter() {
                                    log::debug!("Commit {} for offset {}@{}", &topic_name, k, v+1);
                                    tplist.add_partition_offset(&topic_name, k, Offset::from_raw(v as i64+1)).ok();
                                }

                                if let Err(e) = cr.commit(&tplist, rdkafka::consumer::CommitMode::Async) {
                                    //omit error, just log it
                                    log::error!("Encounter error in kafka commit: {}", e);
                                }

                            }else {
                                break;
                            }

                        }
                        _ = &mut rx => {break;}
                    }
                }
                log::info!("exit auto commiting for topic {}", topic_name);
                receiver.final_status()
            }),
            TypedTopicConfig::topic_name(self).to_string(),
            tx,
        )
    }
}

impl AutoCommitRet {
    async fn commit<C: RdConsumerExt>(thread_h: tokio::task::JoinHandle<TaskNotifyFlag>, topic: &str, cr: &C) {
        let cr = cr.to_self();
        let ret_notify = thread_h.await.unwrap();
        log::debug!("Enter final Commit for topic {}: {:?}", topic, ret_notify);
        if !ret_notify.is_empty() {
            let mut tplist = TopicPartitionList::new();

            for (k, v) in ret_notify.into_iter() {
                log::debug!("Final Commit {} for offset {}@{}", topic, k, v + 1);
                tplist.add_partition_offset(topic, k, Offset::from_raw(v as i64 + 1)).ok();
            }

            cr.commit(&tplist, rdkafka::consumer::CommitMode::Async).unwrap();
        }
    }

    pub async fn interrut_and_commit<C: RdConsumerExt>(self, cr: &C) {
        let AutoCommitRet(ret_h, topic, tx) = self;
        tx.send(()).unwrap();
        Self::commit(ret_h, &topic, cr).await
    }

    pub async fn final_commit<C: RdConsumerExt>(self, cr: &C) {
        let AutoCommitRet(ret_h, topic, _) = self;
        Self::commit(ret_h, &topic, cr).await
    }
}

/*------ Mixed some transform here -------- */
use crate::market;
use crate::utils::FTimestamp;

impl<'r> From<&'r super::Trade> for models::MarketTrade {
    fn from(origin: &'r super::Trade) -> Self {
        models::MarketTrade {
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

use crate::models::DecimalDbType;
use crate::types::OrderEventType;
use std::str::FromStr;

impl<'r> From<&'r super::OrderMessage> for models::OrderHistory {
    fn from(origin: &'r super::OrderMessage) -> Self {
        models::OrderHistory::from(&origin.order)
    }
}

pub struct ClosedOrder();

impl MsgDataTransformer<models::OrderHistory> for ClosedOrder {
    type MsgType = super::OrderMessage;
    fn into(order: &Self::MsgType) -> Option<models::OrderHistory> {
        match order.event {
            OrderEventType::FINISH => Some(order.into()),
            _ => None,
        }
    }
}

fn decimal_warning<E: std::error::Error>(e: E) -> DecimalDbType {
    log::error!("Decimal decode fail {}", e);
    DecimalDbType::default()
}

impl<'r> From<&'r super::BalanceMessage> for models::BalanceHistory {
    fn from(origin: &'r super::BalanceMessage) -> Self {
        models::BalanceHistory {
            time: utils::FTimestamp::from(&origin.timestamp).into(),
            user_id: origin.user_id as i32,
            asset: origin.asset.clone(),
            business: origin.business.clone(),
            change: DecimalDbType::from_str(&origin.change).unwrap_or_else(decimal_warning),
            balance: DecimalDbType::from_str(&origin.balance).unwrap_or_else(decimal_warning),
            balance_available: DecimalDbType::from_str(&origin.balance_available).unwrap_or_else(decimal_warning),
            balance_frozen: DecimalDbType::from_str(&origin.balance_frozen).unwrap_or_else(decimal_warning),
            detail: origin.detail.clone(),
        }
    }
}

pub struct AskTrade();

impl MsgDataTransformer<models::UserTrade> for AskTrade {
    type MsgType = super::Trade;
    fn into(trade: &Self::MsgType) -> Option<models::UserTrade> {
        Some(models::UserTrade {
            time: FTimestamp(trade.timestamp).into(),
            user_id: trade.ask_user_id as i32,
            market: trade.market.clone(),
            trade_id: trade.id as i64,
            order_id: trade.ask_order_id as i64,
            counter_order_id: trade.bid_order_id as i64, // counter order
            side: market::OrderSide::ASK as i16,
            role: trade.ask_role as i16,
            price: trade.price,
            amount: trade.amount,
            quote_amount: trade.quote_amount,
            fee: trade.ask_fee,
            counter_order_fee: trade.bid_fee, // counter order
        })
    }
}

pub struct BidTrade();

impl MsgDataTransformer<models::UserTrade> for BidTrade {
    type MsgType = super::Trade;
    fn into(trade: &Self::MsgType) -> Option<models::UserTrade> {
        Some(models::UserTrade {
            time: FTimestamp(trade.timestamp).into(),
            user_id: trade.bid_user_id as i32,
            market: trade.market.clone(),
            trade_id: trade.id as i64,
            order_id: trade.bid_order_id as i64,
            counter_order_id: trade.ask_order_id as i64, // counter order
            side: market::OrderSide::BID as i16,
            role: trade.bid_role as i16,
            price: trade.price,
            amount: trade.amount,
            quote_amount: trade.quote_amount,
            fee: trade.bid_fee,
            counter_order_fee: trade.ask_fee, // counter order
        })
    }
}

impl<'r> From<&'r super::UserMessage> for models::AccountDesc {
    fn from(origin: &'r super::UserMessage) -> Self {
        Self {
            id: origin.user_id as i32, // TODO: will this overflow?
            l1_address: origin.l1_address.clone(),
            l2_pubkey: origin.l2_pubkey.clone(),
        }
    }
}

impl<'r> From<&'r super::TransferMessage> for models::InternalTx {
    fn from(origin: &'r super::TransferMessage) -> Self {
        Self {
            time: FTimestamp(origin.time).into(),
            user_from: origin.user_from as i32, // TODO: will this overflow?
            user_to: origin.user_to as i32,     // TODO: will this overflow?
            signature: origin.signature.clone(),
            asset: origin.asset.clone(),
            amount: DecimalDbType::from_str(&origin.amount).unwrap_or_else(decimal_warning),
        }
    }
}
