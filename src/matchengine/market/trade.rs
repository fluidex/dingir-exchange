use crate::market::Order;
use crate::types::MarketRole;
use crate::types::OrderSide;
use crate::utils::InternedString;
use fluidex_common::rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VerboseOrderState {
    pub user_id: u32,
    pub order_id: u64,
    pub order_side: OrderSide,
    pub finished_base: Decimal,
    pub finished_quote: Decimal,
    pub finished_fee: Decimal,
    //pub remain: Decimal,
    //pub frozen: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VerboseBalanceState {
    pub user_id: u32,
    pub asset: InternedString,
    // total = balance_available + balance_frozen
    pub balance: Decimal,
    //pub balance_available: Deimcal,
    //pub balance_frozen: Deimcal,
}

// TODO: rename this?
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VerboseTradeState {
    // emit all the related state
    pub order_states: Vec<VerboseOrderState>,
    pub balance_states: Vec<VerboseBalanceState>,
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
    pub ask_broker_id: String,
    pub ask_account_id: String,
    pub ask_order_id: u64,
    pub ask_role: MarketRole, // take/make
    pub ask_fee: Decimal,

    pub bid_user_id: u32,
    pub bid_broker_id: String,
    pub bid_account_id: String,
    pub bid_order_id: u64,
    pub bid_role: MarketRole,
    pub bid_fee: Decimal,

    // only not none when this is this order's first trade
    pub ask_order: Option<Order>,
    pub bid_order: Option<Order>,

    #[cfg(feature = "emit_state_diff")]
    pub state_before: VerboseTradeState,
    #[cfg(feature = "emit_state_diff")]
    pub state_after: VerboseTradeState,
}
