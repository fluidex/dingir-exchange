use crate::types::MarketRole;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VerboseOrderState {
    pub price: Decimal,
    pub amount: Decimal,
    pub finished_base: Decimal,
    pub finished_quote: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VerboseBalanceState {
    pub bid_user_base: Decimal,
    pub bid_user_quote: Decimal,
    pub ask_user_base: Decimal,
    pub ask_user_quote: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VerboseTradeState {
    // emit all the related state
    pub ask_order_state: VerboseOrderState,
    pub bid_order_state: VerboseOrderState,
    pub balance: VerboseBalanceState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trade {
    pub id: u64,
    pub timestamp: f64, // unix epoch timestamp,
    pub market: String,
    pub base: String,
    pub quote: String,
    pub price: Decimal,
    pub amount: Decimal,
    pub quote_amount: Decimal,

    pub ask_user_id: u32,
    pub ask_order_id: u64,
    pub ask_role: MarketRole, // take/make
    pub ask_fee: Decimal,

    pub bid_user_id: u32,
    pub bid_order_id: u64,
    pub bid_role: MarketRole,
    pub bid_fee: Decimal,

    #[cfg(feature = "emit_state_diff")]
    pub state_before: VerboseTradeState,
    #[cfg(feature = "emit_state_diff")]
    pub state_after: VerboseTradeState,
}
