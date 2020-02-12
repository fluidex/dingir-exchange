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
        market: String::from(o.market),
        category: String::from(o.source),
        r#type: if o.type_0 == market::OrderType::LIMIT {
            OrderType::Limit as i32
        } else {
            OrderType::Market as i32
        },
        side: if o.side == market::OrderSide::ASK {
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
        left: o.left.to_string(),
        deal_stock: o.deal_stock.to_string(),
        deal_money: o.deal_money.to_string(),
        deal_fee: o.deal_fee.to_string(),
    }
}

pub fn order_input_from_proto(req: OrderPutRequest) -> Result<market::LimitOrderInput, rust_decimal::Error> {
    Ok(market::LimitOrderInput {
        user_id: req.user_id,
        side: if req.order_side == OrderSide::Ask as i32 {
            market::OrderSide::ASK
        } else {
            market::OrderSide::BID
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
        source: req.category,
        market: req.market,
    })
}
