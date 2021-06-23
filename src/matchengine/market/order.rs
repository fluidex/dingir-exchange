use crate::types::{OrderSide, OrderType};
use crate::utils::intern_string;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

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

#[derive(Debug, Clone, Copy)]
pub struct MarketString(&'static str);

impl From<&'static str> for MarketString {
    fn from(str: &'static str) -> Self {
        MarketString(str)
    }
}

impl std::ops::Deref for MarketString {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl serde::ser::Serialize for MarketString {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

impl<'de> serde::de::Deserialize<'de> for MarketString {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(intern_string(&s).into())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub base: MarketString,
    pub quote: MarketString,
    pub market: MarketString,
    #[serde(rename = "type")]
    pub type_: OrderType, // enum
    pub side: OrderSide,
    pub user: u32,
    pub create_time: f64,
    pub update_time: f64,
    pub price: Decimal,
    pub amount: Decimal,
    pub taker_fee: Decimal,
    pub maker_fee: Decimal,
    pub remain: Decimal,
    pub frozen: Decimal,
    pub finished_base: Decimal,
    pub finished_quote: Decimal,
    pub finished_fee: Decimal,
    pub post_only: bool,
    pub signature: String, // TODO: bytes
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

    pub(super) fn deep(&self) -> Order {
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
    pub signature: String, // TODO: bytes
}
