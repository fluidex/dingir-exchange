use crate::database::{DatabaseWriter, DatabaseWriterConfig};
use crate::market;
use crate::modelsnew as models;
use crate::types::Trade;

use crate::utils::timestamp_to_system_time;
use crate::utils::FTimestamp;
use anyhow::Result;

type BalanceWriter = DatabaseWriter<models::BalanceHistory>;
type OrderWriter = DatabaseWriter<models::OrderHistory>;
type TradeWriter = DatabaseWriter<models::TradeHistory>;

pub trait HistoryWriter {
    fn append_balance_history(&mut self, data: models::BalanceHistory);
    fn append_order_history(&mut self, order: &market::Order);
    fn append_trade_history(&mut self, trade: &Trade);
}

pub struct DummyHistoryWriter;
impl HistoryWriter for DummyHistoryWriter {
    fn append_balance_history(&mut self, _data: models::BalanceHistory) {}
    fn append_order_history(&mut self, _order: &market::Order) {}
    fn append_trade_history(&mut self, _trade: &Trade) {}
}

pub struct DatabaseHistoryWriter {
    pub balance_writer: BalanceWriter,
    pub trade_writer: TradeWriter,
    pub order_writer: OrderWriter,
}

impl DatabaseHistoryWriter {
    pub fn new(config: &DatabaseWriterConfig) -> Result<DatabaseHistoryWriter> {
        Ok(DatabaseHistoryWriter {
            balance_writer: BalanceWriter::new(config)?,
            trade_writer: TradeWriter::new(config)?,
            order_writer: OrderWriter::new(config)?,
        })
    }
}

impl HistoryWriter for DatabaseHistoryWriter {
    fn append_balance_history(&mut self, data: models::BalanceHistory) {
        self.balance_writer.append(data);
    }
    fn append_order_history(&mut self, order: &market::Order) {
        let data = models::OrderHistory {
            id: order.id as i64,
            create_time: FTimestamp(order.create_time).into(),
            finish_time: FTimestamp(order.update_time).into(),
            user_id: order.user as i32,
            market: order.market.to_string(),
            t: order.type_ as i16,
            side: 0,
            price: order.price,
            amount: order.amount,
            taker_fee: order.taker_fee,
            maker_fee: order.maker_fee,
            finished_base: order.finished_base,
            finished_quote: order.finished_quote,
            finished_fee: order.finished_fee,
        };
        self.order_writer.append(data);
    }

    fn append_trade_history(&mut self, trade: &Trade) {
        let ask_trade = models::TradeHistory {
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
        };
        let bid_trade = models::TradeHistory {
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
        };
        self.trade_writer.append(ask_trade);
        self.trade_writer.append(bid_trade);
    }
}
