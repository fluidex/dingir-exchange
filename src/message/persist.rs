use super::consumer::{self, RdConsumerExt}; //crate::message::consumer
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
    pub fn new(src: &database::DatabaseWriter<U>) -> Self {
        MsgDataPersistor::<U, UM> {
            writer: RefCell::new(src.get_entry().unwrap()),
            _phantom: PhantomData,
        }
    }
}

//An simple handler, just persist it by DatabaseWriter
impl<'c, C, U, UM> consumer::TypedMessageHandler<'c, C> for MsgDataPersistor<U, UM>
where
    UM: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send + Sync,
    U: Clone + Send + Sync + From<UM>,
    C: RdConsumerExt + 'static,
{
    type DataType = UM;
    fn on_message(&self, msg: UM, _cr: &'c C::SelfType) {
        self.writer.borrow_mut().gen().append(From::from(msg)).ok();
    }
    fn on_no_msg(&self, _cr: &'c C::SelfType) {} //do nothing
}

impl From<types::Trade> for models::TradeRecord {
    fn from(origin: types::Trade) -> models::TradeRecord {
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
