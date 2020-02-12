#![macro_use]

use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

pub type SimpleResult = anyhow::Result<()>;

#[macro_export]
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
#[repr(u8)]
pub enum MarketRole {
    MAKER = 1,
    TAKER = 2,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Deal {
    pub id: u64,
    pub timestamp: f64, // unix epoch timestamp,
    pub market: String,
    pub stock: String,
    pub money: String,
    pub price: rust_decimal::Decimal,
    pub amount: rust_decimal::Decimal,
    pub deal: rust_decimal::Decimal,

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
