use crate::database::{DatabaseWriter, DatabaseWriterConfig};
use crate::market;
use crate::models;
use crate::schema;
use crate::types::Deal;
use crate::utils;

use anyhow::Result;

type BalanceWriter = DatabaseWriter<schema::balance_history_example::table, models::BalanceHistory>;
type OrderWriter = DatabaseWriter<schema::order_history_example::table, models::OrderHistory>;
type DealWriter = DatabaseWriter<schema::deal_history_example::table, models::DealHistory>;

pub trait HistoryWriter {
    fn append_balance_history(&mut self, data: models::BalanceHistory);
    fn append_order_history(&mut self, order: &market::Order);
    fn append_deal_history(&mut self, deal: &Deal);
}

pub struct DummyHistoryWriter;
impl HistoryWriter for DummyHistoryWriter {
    fn append_balance_history(&mut self, _data: models::BalanceHistory) {}
    fn append_order_history(&mut self, _order: &market::Order) {}
    fn append_deal_history(&mut self, _deal: &Deal) {}
}

pub struct DatabaseHistoryWriter {
    pub balance_writer: BalanceWriter,
    pub deal_writer: DealWriter,
    pub order_writer: OrderWriter,
}

impl DatabaseHistoryWriter {
    pub fn new(config: &DatabaseWriterConfig) -> Result<DatabaseHistoryWriter> {
        Ok(DatabaseHistoryWriter {
            balance_writer: BalanceWriter::new(config)?,
            deal_writer: DealWriter::new(config)?,
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
            id: order.id,
            create_time: utils::timestamp_to_chrono(order.create_time),
            finish_time: utils::timestamp_to_chrono(order.update_time),
            user_id: order.user,
            market: order.market.to_string(),
            source: order.source.to_string(),
            t: order.type_0 as u8,
            side: 0,
            price: utils::decimal_r2b(&order.price),
            amount: utils::decimal_r2b(&order.amount),
            taker_fee: utils::decimal_r2b(&order.taker_fee),
            maker_fee: utils::decimal_r2b(&order.maker_fee),
            deal_stock: utils::decimal_r2b(&order.deal_stock),
            deal_money: utils::decimal_r2b(&order.deal_money),
            deal_fee: utils::decimal_r2b(&order.deal_fee),
        };
        self.order_writer.append(data);
    }

    fn append_deal_history(&mut self, deal: &Deal) {
        let ask_deal = models::DealHistory {
            time: utils::timestamp_to_chrono(deal.timestamp),
            user_id: deal.ask_user_id,
            market: deal.market.clone(),
            deal_id: deal.id,
            order_id: deal.ask_order_id,
            deal_order_id: deal.bid_order_id, // counter order
            side: market::OrderSide::ASK as u8,
            role: deal.ask_role as u8,
            price: utils::decimal_r2b(&deal.price),
            amount: utils::decimal_r2b(&deal.amount),
            deal: utils::decimal_r2b(&deal.deal),
            fee: utils::decimal_r2b(&deal.ask_fee),
            deal_fee: utils::decimal_r2b(&deal.bid_fee), // counter order
        };
        let bid_deal = models::DealHistory {
            time: utils::timestamp_to_chrono(deal.timestamp),
            user_id: deal.bid_user_id,
            market: deal.market.clone(),
            deal_id: deal.id,
            order_id: deal.bid_order_id,
            deal_order_id: deal.ask_order_id, // counter order
            side: market::OrderSide::BID as u8,
            role: deal.bid_role as u8,
            price: utils::decimal_r2b(&deal.price),
            amount: utils::decimal_r2b(&deal.amount),
            deal: utils::decimal_r2b(&deal.deal),
            fee: utils::decimal_r2b(&deal.bid_fee),
            deal_fee: utils::decimal_r2b(&deal.ask_fee), // counter order
        };
        self.deal_writer.append(ask_deal);
        self.deal_writer.append(bid_deal);
    }
}
