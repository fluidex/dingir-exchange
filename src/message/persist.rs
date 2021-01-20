use super::consumer::{self, RdConsumerExt}; //crate::message::consumer
use crate::{database, models, types, utils};
use serde::Deserialize;
use std::marker::PhantomData;
use tonic::async_trait;
use types::OrderSide;

use sqlx::migrate::Migrator;
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations/ts");

pub struct MsgDataPersistor<'a, U: Clone + Send, UM> {
    pub writer: &'a database::DatabaseWriter<U>,
    pub phantom: PhantomData<UM>,
}

//An simple handler, just persist it by DatabaseWriter
#[async_trait]
impl<'a, 'c, C, U, UM> consumer::TypedMessageHandler<'c, C> for MsgDataPersistor<'a, U, UM>
where
    UM: 'static + for<'de> Deserialize<'de> + std::fmt::Debug + Send + Sync,
    U: Clone + Send + Sync + From<UM>,
    C: RdConsumerExt + 'static,
{
    type DataType = UM;
    async fn on_message(&self, msg: UM, _cr: &'c C::SelfType) {
        self.writer.append(From::from(msg));
    }
    async fn on_no_msg(&self, _cr: &'c C::SelfType) {} //do nothing
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
