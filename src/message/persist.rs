use super::consumer::{MessageHandler, TypedHandler};
use crate::{database, models, types, utils};
use rdkafka::Message;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::BorrowedMessage;
use rdkafka::topic_partition_list::{Offset, TopicPartitionList};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::sync::Arc;
use types::OrderSide;

use sqlx::migrate::Migrator;
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations/ts");

pub trait MsgDataTransformer<T: Clone + Send>: Send {
    type MsgType: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send;
    fn into(msg: &Self::MsgType) -> Option<T>;
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

pub struct TopicConfig {
    pub topic: String,
    pub handler: Arc<dyn MessageHandler + Send + Sync>,
}

pub struct TopicHandlerBuilder<'a, T> {
    topic: String,
    handlers: Vec<Box<dyn Fn(&T, &BorrowedMessage<'_>) + Send + Sync>>,
    notify_fns: Vec<Box<dyn Fn() -> tokio::sync::watch::Receiver<database::TaskNotifyFlag> + Send + Sync + 'a>>,
    _phantom: PhantomData<T>,
}

impl<'a, T: DeserializeOwned + Send + Sync + std::fmt::Debug + 'static> TopicHandlerBuilder<'a, T> {
    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.to_string(),
            handlers: Vec::new(),
            notify_fns: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn persist_to<U>(mut self, db: &'a database::DatabaseWriter<U>) -> Self
    where
        U: Clone + Send + Sync + 'static,
        for<'r> &'r T: Into<U>,
    {
        let entry = db.get_entry().unwrap();
        self.handlers.push(Box::new(move |msg, origin| {
            let u: U = msg.into();
            let notify = database::TaskNotification::new(origin.partition(), origin.offset() as u64);
            entry.generate().append_with_notify(u, Some(notify)).ok();
        }));
        self.notify_fns.push(Box::new(move || db.listen_notify()));
        self
    }

    pub fn persist_transformed<U, Tr>(mut self, db: &'a database::DatabaseWriter<U>) -> Self
    where
        U: Clone + Send + Sync + 'static,
        Tr: MsgDataTransformer<U, MsgType = T>,
    {
        let entry = db.get_entry().unwrap();
        self.handlers.push(Box::new(move |msg, origin| {
            if let Some(u) = Tr::into(msg) {
                let notify = database::TaskNotification::new(origin.partition(), origin.offset() as u64);
                entry.generate().append_with_notify(u, Some(notify)).ok();
            }
        }));
        self.notify_fns.push(Box::new(move || db.listen_notify()));
        self
    }

    pub fn build(self) -> (TopicConfig, AutoCommitSetup<'a>) {
        let handlers = self.handlers;
        let handler = TypedHandler::new(move |msg: &T, origin: &BorrowedMessage<'_>| {
            for h in &handlers {
                h(msg, origin);
            }
        });
        let config = TopicConfig {
            topic: self.topic.clone(),
            handler: Arc::new(handler),
        };
        let setup = AutoCommitSetup {
            topic: self.topic,
            notify_fns: self.notify_fns,
        };
        (config, setup)
    }
}

pub struct AutoCommitSetup<'a> {
    topic: String,
    notify_fns: Vec<Box<dyn Fn() -> tokio::sync::watch::Receiver<database::TaskNotifyFlag> + Send + Sync + 'a>>,
}

impl<'a> AutoCommitSetup<'a> {
    pub fn auto_commit_start(&self, cr: Arc<StreamConsumer>) -> AutoCommitRet {
        let mut receiver = self.build_tracker();
        let topic_name = self.topic.clone();
        let (tx, mut rx) = tokio::sync::oneshot::channel();

        AutoCommitRet(
            tokio::spawn(async move {
                log::info!("start auto commiting for topic {}", topic_name);
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
                                    log::error!("Encounter error in kafka commit: {}", e);
                                }
                            } else {
                                break;
                            }
                        }
                        _ = &mut rx => { break; }
                    }
                }
                log::info!("exit auto commiting for topic {}", topic_name);
                receiver.final_status()
            }),
            self.topic.clone(),
            tx,
        )
    }

    fn build_tracker(&self) -> NotifyTrackerReceiver {
        let mut trackers: Vec<NotifyTrackerReceiver> = self
            .notify_fns
            .iter()
            .map(|f| NotifyTrackerReceiver {
                status: NotifyTracker(HashMap::new()),
                listener: f(),
                next: None,
            })
            .collect();
        for i in (1..trackers.len()).rev() {
            let next = Box::new(trackers.remove(i));
            trackers[i - 1].next = Some(next);
        }
        trackers.into_iter().next().unwrap_or_else(|| NotifyTrackerReceiver {
            status: NotifyTracker(HashMap::new()),
            listener: tokio::sync::watch::channel(std::collections::HashMap::new()).1,
            next: None,
        })
    }
}

#[derive(Debug, Clone)]
pub enum NotifyTrackItem {
    Left(u64),
    Right(u64),
}

impl NotifyTrackItem {
    fn is_left(&self) -> bool {
        matches!(self, NotifyTrackItem::Left(_))
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
        NotifyTracker(input.iter().map(|(k, v)| (*k, mf(*v))).collect())
    }
    fn merge(&mut self, another: NotifyTracker) -> TaskNotifyFlag {
        another
            .0
            .into_iter()
            .filter_map(|(k, v)| self.entry(k).or_insert_with(|| v.clone()).merge(v).map(|u| (k, u)))
            .collect()
    }
}

pub struct NotifyTrackerReceiver {
    status: NotifyTracker,
    listener: tokio::sync::watch::Receiver<TaskNotifyFlag>,
    next: Option<Box<NotifyTrackerReceiver>>,
}

impl NotifyTrackerReceiver {
    fn final_status(self) -> TaskNotifyFlag {
        self.status.0.into_iter().map(|(k, v)| (k, v.val_into())).collect()
    }

    fn changed(&mut self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<TaskNotifyFlag>> + Send + '_>> {
        let listener = &mut self.listener;
        let status = &mut self.status;
        if let Some(next_iter) = self.next.as_mut() {
            Box::pin(async move {
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
                            } else {
                                return None
                            }
                        }
                    }
                }
            })
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

pub struct AutoCommitRet(tokio::task::JoinHandle<TaskNotifyFlag>, String, tokio::sync::oneshot::Sender<()>);

impl AutoCommitRet {
    async fn commit(thread_h: tokio::task::JoinHandle<TaskNotifyFlag>, topic: &str, cr: &StreamConsumer) {
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

    pub async fn interrut_and_commit(self, cr: &StreamConsumer) {
        let AutoCommitRet(ret_h, topic, tx) = self;
        tx.send(()).unwrap();
        Self::commit(ret_h, &topic, cr).await
    }

    pub async fn final_commit(self, cr: &StreamConsumer) {
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
            business_id: origin.business_id as i64,
            asset: origin.asset.clone(),
            business: origin.business.clone(),
            market_price: DecimalDbType::from_str(&origin.market_price).unwrap_or_else(decimal_warning),
            change: DecimalDbType::from_str(&origin.change).unwrap_or_else(decimal_warning),
            balance: DecimalDbType::from_str(&origin.balance).unwrap_or_else(decimal_warning),
            balance_available: DecimalDbType::from_str(&origin.balance_available).unwrap_or_else(decimal_warning),
            balance_frozen: DecimalDbType::from_str(&origin.balance_frozen).unwrap_or_else(decimal_warning),
            detail: origin.detail.clone(),
            signature: origin.signature.as_bytes().to_vec(),
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
            asset: origin.asset.clone(),
            amount: DecimalDbType::from_str(&origin.amount).unwrap_or_else(decimal_warning),
            signature: origin.signature.as_bytes().to_vec(),
        }
    }
}
