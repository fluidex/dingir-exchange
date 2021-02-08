use crate::market;
use rust_decimal::Decimal;

pub mod matchengine {
    tonic::include_proto!("matchengine");
}

pub use matchengine::matchengine_server::*;
pub use matchengine::*;
use rust_decimal::prelude::Zero;
use std::str::FromStr;

pub fn order_to_proto(o: &market::Order) -> OrderInfo {
    OrderInfo {
        id: o.id,
        market: String::from(&*o.market),
        order_type: if o.type_ == market::OrderType::LIMIT {
            OrderType::Limit as i32
        } else {
            OrderType::Market as i32
        },
        order_side: if o.side == market::OrderSide::ASK {
            OrderSide::Ask as i32
        } else {
            OrderSide::Bid as i32
        },
        user_id: o.user,
        create_time: o.create_time,
        update_time: o.update_time,
        price: o.price.to_string(),
        amount: o.amount.to_string(),
        taker_fee: o.taker_fee.to_string(),
        maker_fee: o.maker_fee.to_string(),
        remain: o.remain.to_string(),
        finished_base: o.finished_base.to_string(),
        finished_quote: o.finished_quote.to_string(),
        finished_fee: o.finished_fee.to_string(),
    }
}

pub fn order_input_from_proto(req: &OrderPutRequest) -> Result<market::OrderInput, rust_decimal::Error> {
    Ok(market::OrderInput {
        user_id: req.user_id,
        side: if req.order_side == OrderSide::Ask as i32 {
            market::OrderSide::ASK
        } else {
            market::OrderSide::BID
        },
        type_: if req.order_type == OrderType::Limit as i32 {
            market::OrderType::LIMIT
        } else {
            market::OrderType::MARKET
        },
        amount: Decimal::from_str(req.amount.as_str())?,
        price: Decimal::from_str(req.price.as_str())?,
        taker_fee: if req.taker_fee.is_empty() {
            Decimal::zero()
        } else {
            Decimal::from_str(req.taker_fee.as_str())?
        },
        maker_fee: if req.maker_fee.is_empty() {
            Decimal::zero()
        } else {
            Decimal::from_str(req.maker_fee.as_str())?
        },
        market: req.market.clone(),
    })
}
