#![allow(unused_imports)]
#![allow(clippy::single_component_path_imports)]

use crate::schema::operation_log;
use crate::schema::{balance_history, order_history, trade_history};
use crate::schema::{balance_slice, order_slice, slice_history};

pub type DecimalDbType = rust_decimal::Decimal;
pub type TimestampDbType = std::time::SystemTime;

#[derive(Queryable, Insertable, Debug, Clone)]
#[table_name = "balance_history"]
pub struct BalanceHistory {
    //pub id: i64,
    pub time: TimestampDbType,
    pub user_id: i32,
    pub asset: String,
    pub business: String,
    pub change: DecimalDbType,
    pub balance: DecimalDbType,
    // TODO: change it to jsonb
    pub detail: String,
}

#[derive(Queryable, Insertable, Debug, Clone)]
#[table_name = "order_history"]
pub struct OrderHistory {
    pub id: i64,
    pub create_time: TimestampDbType,
    pub finish_time: TimestampDbType,
    pub user_id: i32,
    pub market: String,
    // Type enum: MARKET or LIMIT
    pub t: i16,
    pub side: i16,
    pub price: DecimalDbType,
    pub amount: DecimalDbType,
    pub taker_fee: DecimalDbType,
    pub maker_fee: DecimalDbType,
    pub finished_base: DecimalDbType,
    pub finished_quote: DecimalDbType,
    pub finished_fee: DecimalDbType,
}

#[derive(Queryable, Insertable, Debug, Clone)]
#[table_name = "trade_history"]
pub struct TradeHistory {
    pub time: TimestampDbType,
    pub user_id: i32,
    pub market: String,
    pub trade_id: i64,
    pub order_id: i64,
    pub counter_order_id: i64,
    pub side: i16,
    pub role: i16,
    pub price: DecimalDbType,
    pub amount: DecimalDbType,
    pub quote_amount: DecimalDbType,
    pub fee: DecimalDbType,
    pub counter_order_fee: DecimalDbType,
}

// Can the following struct be auto generated in diesel?
#[derive(Queryable, Insertable, Debug, Clone)]
#[table_name = "operation_log"]
pub struct OperationLog {
    pub id: i64,
    pub time: TimestampDbType,
    pub method: String,
    // TODO: change it to jsonb
    pub params: String,
}

#[derive(Queryable, Insertable, Debug, Clone)]
#[table_name = "balance_slice"]
pub struct BalanceSlice {
    pub id: i32,
    pub slice_id: i64, // Unix timestamp
    pub user_id: i32,
    pub asset: String,
    pub t: i16, // Enum: AVAILABLE or FREEZE
    pub balance: DecimalDbType,
}

#[derive(Queryable, Insertable, Debug, Clone)]
#[table_name = "balance_slice"]
pub struct NewBalanceSlice {
    //pub id: i32,
    pub slice_id: i64, // Unix timestamp
    pub user_id: i32,
    pub asset: String,
    pub t: i16, // Enum: AVAILABLE or FREEZE
    pub balance: DecimalDbType,
}

#[derive(Queryable, Insertable, Debug, Clone)]
#[table_name = "order_slice"]
pub struct OrderSlice {
    pub id: i64,
    pub slice_id: i64,
    // Type enum: MARKET or LIMIT
    pub t: i16,
    pub side: i16,
    pub create_time: TimestampDbType,
    pub update_time: TimestampDbType,
    pub user_id: i32,
    pub market: String,
    //pub source: String,
    pub price: DecimalDbType,
    pub amount: DecimalDbType,
    pub taker_fee: DecimalDbType,
    pub maker_fee: DecimalDbType,
    pub remain: DecimalDbType,
    pub frozen: DecimalDbType,
    pub finished_base: DecimalDbType,
    pub finished_quote: DecimalDbType,
    pub finished_fee: DecimalDbType,
}

// xx_id here means the last persisted entry id
#[derive(Queryable, Insertable, Debug, Clone)]
#[table_name = "slice_history"]
pub struct SliceHistory {
    pub id: i32,
    pub time: i64,
    pub end_operation_log_id: i64,
    pub end_order_id: i64,
    pub end_trade_id: i64,
}
