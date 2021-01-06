#![macro_use]

use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

pub type SimpleResult = anyhow::Result<()>;

macro_rules! simple_err {
    ($msg:literal $(,)?) => {
        std::result::Result::Err(anyhow::anyhow!($msg))
    };
    ($err:expr $(,)?) => ({
        std::result::Result::Err(anyhow::anyhow!($err))
    });
    ($fmt:expr, $($arg:tt)*) => {
        std::result::Result::Err(anyhow::anyhow!($fmt, $($arg)*))
    };
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
#[repr(i16)]
pub enum MarketRole {
    MAKER = 1,
    TAKER = 2,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Trade {
    pub id: u64,
    pub timestamp: f64, // unix epoch timestamp,
    pub market: String,
    pub base: String,
    pub quote: String,
    pub price: rust_decimal::Decimal,
    pub amount: rust_decimal::Decimal,
    pub quote_amount: rust_decimal::Decimal,

    pub ask_user_id: u32,
    pub ask_order_id: u64,
    pub ask_role: MarketRole, // take/make
    pub ask_fee: rust_decimal::Decimal,

    pub bid_user_id: u32,
    pub bid_order_id: u64,
    pub bid_role: MarketRole,
    pub bid_fee: rust_decimal::Decimal,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum OrderEventType {
    PUT = 1,
    UPDATE = 2,
    FINISH = 3,
}

//pub type DbType = diesel::mysql::Mysql;
//pub type ConnectionType = diesel::mysql::MysqlConnection;
pub type DbType = sqlx::Postgres;
pub type ConnectionType = sqlx::postgres::PgConnection;
pub type DBErrType = sqlx::Error;
