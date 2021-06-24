use super::rpc::*;
use crate::market;

use anyhow::{anyhow, bail, Result};
use arrayref::array_ref;
use rust_decimal::prelude::Zero;
use rust_decimal::Decimal;

use std::convert::TryFrom;
use std::str::FromStr;

pub fn str_to_decimal(s: &str, allow_empty: bool) -> Result<Decimal, rust_decimal::Error> {
    if allow_empty && s.is_empty() {
        Ok(Decimal::zero())
    } else {
        Ok(Decimal::from_str(s)?)
    }
}

impl From<market::Order> for OrderInfo {
    fn from(o: market::Order) -> Self {
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
            post_only: o.post_only,
        }
    }
}

impl TryFrom<OrderPutRequest> for market::OrderInput {
    type Error = anyhow::Error;

    fn try_from(req: OrderPutRequest) -> std::result::Result<Self, Self::Error> {
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
            amount: str_to_decimal(&req.amount, false).map_err(|_| anyhow!("invalid amount"))?,
            price: str_to_decimal(&req.price, req.order_type == OrderType::Market as i32).map_err(|_| anyhow!("invalid price"))?,
            quote_limit: str_to_decimal(&req.quote_limit, true).map_err(|_| anyhow!("invalid quote limit"))?,
            taker_fee: str_to_decimal(&req.taker_fee, true).map_err(|_| anyhow!("invalid taker fee"))?,
            maker_fee: str_to_decimal(&req.maker_fee, true).map_err(|_| anyhow!("invalid maker fee"))?,
            market: req.market.clone(),
            post_only: req.post_only,
            signature: if req.signature.is_empty() {
                log::debug!("empty signature. should only happen in tests");
                [0; 64]
            } else {
                let sig = req.signature.trim_start_matches("0x");
                let v: Vec<u8> = hex::decode(sig)?;
                if v.len() != 64 {
                    bail!("invalid signature length");
                }
                *array_ref!(v[..64], 0, 64)
            },
        })
    }
}
