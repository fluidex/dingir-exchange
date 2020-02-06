//#[allow(clippy::single_component_path_imports)]

use crate::schema::{balance_history_example, deal_history_example, operlog_example, order_history_example};
//use rust_decimal::Decimal;

// Can the following struct be auto generated in diesel?
#[derive(Queryable, Insertable, Debug, Clone)]
#[table_name = "operlog_example"]
pub struct Operlog {
    pub id: u64,
    pub time: chrono::NaiveDateTime,
    pub method: String,
    // TODO: change it to jsonb
    pub params: String,
}

#[derive(Queryable, Insertable, Debug, Clone)]
#[table_name = "balance_history_example"]
pub struct BalanceHistory {
    //pub id: u64,
    pub time: chrono::NaiveDateTime,
    pub user_id: u32,
    pub asset: String,
    pub business: String,
    // TODO: bigdecimal or rust-decimal?
    pub change: bigdecimal::BigDecimal,
    pub balance: bigdecimal::BigDecimal,
    // TODO: change it to jsonb
    pub detail: String,
}

#[derive(Queryable, Insertable, Debug, Clone)]
#[table_name = "order_history_example"]
pub struct OrderHistory {
    pub id: u64,
    pub create_time: chrono::NaiveDateTime,
    pub finish_time: chrono::NaiveDateTime,
    pub user_id: u32,
    pub market: String,
    pub source: String,
    // Type enum: MARKET or LIMIT
    pub t: u8,
    pub side: u8,
    pub price: bigdecimal::BigDecimal,
    pub amount: bigdecimal::BigDecimal,
    pub taker_fee: bigdecimal::BigDecimal,
    pub maker_fee: bigdecimal::BigDecimal,
    pub deal_stock: bigdecimal::BigDecimal,
    pub deal_money: bigdecimal::BigDecimal,
    pub deal_fee: bigdecimal::BigDecimal,
}

#[derive(Queryable, Insertable, Debug, Clone)]
#[table_name = "deal_history_example"]
pub struct DealHistory {
    pub time: chrono::NaiveDateTime,
    pub user_id: u32,
    pub market: String,
    pub deal_id: u64,
    pub order_id: u64,
    pub deal_order_id: u64,
    pub side: u8,
    pub role: u8,
    pub price: bigdecimal::BigDecimal,
    pub amount: bigdecimal::BigDecimal,
    pub deal: bigdecimal::BigDecimal,
    pub fee: bigdecimal::BigDecimal,
    pub deal_fee: bigdecimal::BigDecimal,
}
