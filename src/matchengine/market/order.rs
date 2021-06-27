use crate::primitives::*;
use crate::types::{OrderSide, OrderType};
use crate::utils::InternedString;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;
use std::cmp::Ordering;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

big_array! { BigArray; }

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct MarketKeyAsk {
    pub order_price: Decimal,
    pub order_id: u64,
}

#[derive(PartialEq, Eq)]
pub struct MarketKeyBid {
    pub order_price: Decimal,
    pub order_id: u64,
}

impl Ord for MarketKeyBid {
    fn cmp(&self, other: &Self) -> Ordering {
        let price_order = self.order_price.cmp(&other.order_price).reverse();
        if price_order != Ordering::Equal {
            price_order
        } else {
            self.order_id.cmp(&other.order_id)
        }
    }
}

#[cfg(test)]
#[test]
fn test_order_sort() {
    use rust_decimal::prelude::One;
    use rust_decimal::prelude::Zero;
    {
        let o1 = MarketKeyBid {
            order_price: Decimal::zero(),
            order_id: 5,
        };
        let o2 = MarketKeyBid {
            order_price: Decimal::zero(),
            order_id: 6,
        };
        let o3 = MarketKeyBid {
            order_price: Decimal::one(),
            order_id: 7,
        };
        assert!(o1 < o2);
        assert!(o3 < o2);
    }
    {
        let o1 = MarketKeyAsk {
            order_price: Decimal::zero(),
            order_id: 5,
        };
        let o2 = MarketKeyAsk {
            order_price: Decimal::zero(),
            order_id: 6,
        };
        let o3 = MarketKeyAsk {
            order_price: Decimal::one(),
            order_id: 7,
        };
        assert!(o1 < o2);
        assert!(o3 > o2);
    }
}

impl PartialOrd for MarketKeyBid {
    fn partial_cmp(&self, other: &MarketKeyBid) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Order {
    // Order can be seen as two part:
    // first, const part, these fields cannot be updated
    // then, the updatable part, which changes whenever a trade occurs
    pub id: u64,
    pub base: InternedString,
    pub quote: InternedString,
    pub market: InternedString,
    #[serde(rename = "type")]
    pub type_: OrderType, // enum
    pub side: OrderSide,
    pub user: u32,
    pub post_only: bool,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    pub price: Decimal,
    pub amount: Decimal,
    // fee rate when the order be treated as a taker
    pub maker_fee: Decimal,
    // fee rate when the order be treated as a taker, not useful when post_only
    pub taker_fee: Decimal,
    pub create_time: f64,

    // below are the changable parts
    // remain + finished_base == amount
    pub remain: Decimal,
    // frozen = if ask { amount (base) } else { amount * price (quote) }
    pub frozen: Decimal,
    pub finished_base: Decimal,
    pub finished_quote: Decimal,
    pub finished_fee: Decimal,
    pub update_time: f64,
}

/*
fn de_market_string<'de, D: serde::de::Deserializer<'de>>(_deserializer: D) -> Result<&'static str, D::Error> {
    Ok("Test")
}
*/

impl Order {
    pub fn get_ask_key(&self) -> MarketKeyAsk {
        MarketKeyAsk {
            order_price: self.price,
            order_id: self.id,
        }
    }
    pub fn get_bid_key(&self) -> MarketKeyBid {
        MarketKeyBid {
            order_price: self.price,
            order_id: self.id,
        }
    }
    pub fn is_ask(&self) -> bool {
        self.side == OrderSide::ASK
    }
}

#[derive(Clone, Debug)]
pub struct OrderRc(Arc<RwLock<Order>>);

/*
    simulate behavior like RefCell, the syncing is ensured by locking in higher rank
    here we use RwLock only for avoiding unsafe tag, we can just use raw pointer
    casted from ARc rather than RwLock here if we do not care about unsafe
*/
impl OrderRc {
    pub(super) fn new(order: Order) -> Self {
        OrderRc(Arc::new(RwLock::new(order)))
    }

    pub fn borrow(&self) -> RwLockReadGuard<'_, Order> {
        self.0.try_read().expect("Lock for parent entry ensure it")
    }

    pub(super) fn borrow_mut(&mut self) -> RwLockWriteGuard<'_, Order> {
        self.0.try_write().expect("Lock for parent entry ensure it")
    }

    pub fn deep(&self) -> Order {
        *self.borrow()
    }
}

pub struct OrderInput {
    pub user_id: u32,
    pub side: OrderSide,
    pub type_: OrderType,
    pub amount: Decimal,
    pub price: Decimal,
    pub quote_limit: Decimal,
    pub taker_fee: Decimal, // FIXME fee should be determined inside engine rather than take from input
    pub maker_fee: Decimal,
    pub market: String,
    pub post_only: bool,
    pub signature: [u8; 64],
}

pub struct OrderCommitment {
    // order_id
    // account_id
    // nonce
    token_sell: Fr,
    token_buy: Fr,
    total_sell: Fr,
    total_buy: Fr,
}

impl OrderCommitment {
    pub fn hash(&self) -> BigInt {
        // consistent with https://github.com/Fluidex/circuits/blob/d6e06e964b9d492f1fa5513bcc2295e7081c540d/helper.ts/state-utils.ts#L38
        // TxType::PlaceOrder
        let magic_head = u32_to_fr(4);
        let data = hash(&[
            magic_head,
            // TODO: sign nonce or order_id
            //u32_to_fr(self.order_id),
            self.token_sell,
            self.token_buy,
            self.total_sell,
            self.total_buy,
        ]);
        //data = hash([data, accountID, nonce]);
        // nonce and orderID seems redundant?

        // account_id is not needed if the hash is signed later?
        //data = hash(&[data, u32_to_fr(self.account_id)]);
        fr_to_bigint(&data)
    }
}
