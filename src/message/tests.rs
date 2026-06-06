use super::*;
use crate::models;
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde_json;
use std::collections::HashMap;
use std::str::FromStr;

use super::persist::MsgDataTransformer;

fn test_timestamp() -> NaiveDateTime {
    NaiveDateTime::from_timestamp(1609459200, 0) // 2021-01-01 00:00:00
}

fn test_decimal(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn test_account_desc() -> AccountDesc {
    AccountDesc {
        id: 42,
        l1_address: "0x1234".to_string(),
        l2_pubkey: "abcd".to_string(),
    }
}

fn test_balance_history() -> BalanceHistory {
    BalanceHistory {
        time: test_timestamp(),
        user_id: 1,
        business_id: 100,
        asset: "ETH".to_string(),
        business: "deposit".to_string(),
        market_price: test_decimal("1500.50"),
        change: test_decimal("10.5"),
        balance: test_decimal("100.0"),
        balance_available: test_decimal("90.0"),
        balance_frozen: test_decimal("10.0"),
        detail: "test detail".to_string(),
        signature: vec![1, 2, 3, 4],
    }
}

fn test_internal_tx() -> InternalTx {
    InternalTx {
        time: test_timestamp(),
        user_from: 1,
        user_to: 2,
        asset: "USDT".to_string(),
        amount: test_decimal("500.0"),
        signature: vec![5, 6, 7, 8],
    }
}

fn test_order() -> Order {
    Order {
        id: 123,
        base: "ETH".into(),
        quote: "USDT".into(),
        market: "ETH_USDT".into(),
        type_: crate::types::OrderType::LIMIT,
        side: crate::types::OrderSide::BID,
        user: 1,
        post_only: false,
        signature: [0u8; 64],
        price: test_decimal("1500.0"),
        amount: test_decimal("1.0"),
        maker_fee: test_decimal("0.001"),
        taker_fee: test_decimal("0.002"),
        create_time: 1609459200.0,
        remain: test_decimal("0.5"),
        frozen: test_decimal("750.0"),
        finished_base: test_decimal("0.5"),
        finished_quote: test_decimal("750.0"),
        finished_fee: test_decimal("0.75"),
        update_time: 1609459201.0,
    }
}

fn test_trade() -> Trade {
    Trade {
        id: 999,
        timestamp: 1609459200.0,
        market: "ETH_USDT".to_string(),
        base: "ETH".to_string(),
        quote: "USDT".to_string(),
        price: test_decimal("1500.0"),
        amount: test_decimal("1.0"),
        quote_amount: test_decimal("1500.0"),
        ask_user_id: 2,
        ask_order_id: 200,
        ask_role: crate::types::MarketRole::TAKER,
        ask_fee: test_decimal("1.5"),
        bid_user_id: 1,
        bid_order_id: 100,
        bid_role: crate::types::MarketRole::MAKER,
        bid_fee: test_decimal("1.0"),
        ask_order: None,
        bid_order: None,
        #[cfg(feature = "emit_state_diff")]
        state_before: Default::default(),
        #[cfg(feature = "emit_state_diff")]
        state_after: Default::default(),
    }
}

#[test]
fn test_user_message_from_account_desc() {
    let acc = test_account_desc();
    let msg = UserMessage::from(acc.clone());
    assert_eq!(msg.user_id, 42);
    assert_eq!(msg.l1_address, "0x1234");
    assert_eq!(msg.l2_pubkey, "abcd");
}

#[test]
fn test_balance_message_from_balance_history() {
    let bh = test_balance_history();
    let msg = BalanceMessage::from(&bh);
    assert_eq!(msg.user_id, 1);
    assert_eq!(msg.business_id, 100);
    assert_eq!(msg.asset, "ETH");
    assert_eq!(msg.business, "deposit");
    assert_eq!(msg.market_price, "1500.50");
    assert_eq!(msg.change, "10.5");
    assert_eq!(msg.balance, "100.0");
    assert_eq!(msg.balance_available, "90.0");
    assert_eq!(msg.balance_frozen, "10.0");
    assert_eq!(msg.detail, "test detail");
    assert_eq!(msg.signature, "\x01\x02\x03\x04");
    assert_eq!(msg.timestamp, 1609459200.0);
}

#[test]
fn test_deposit_message_from_balance_history() {
    let bh = test_balance_history();
    let msg = DepositMessage::from(&bh);
    assert_eq!(msg.user_id, 1);
    assert_eq!(msg.asset, "ETH");
    assert_eq!(msg.change, "10.5");
    assert_eq!(msg.balance, "100.0");
    assert_eq!(msg.detail, "test detail");
    assert_eq!(msg.timestamp, 1609459200.0);
}

#[test]
fn test_withdraw_message_from_balance_history() {
    let bh = test_balance_history();
    let msg = WithdrawMessage::from(&bh);
    assert_eq!(msg.user_id, 1);
    assert_eq!(msg.asset, "ETH");
    assert_eq!(msg.change, "10.5");
    assert_eq!(msg.balance, "100.0");
    assert_eq!(msg.signature, "\x01\x02\x03\x04");
    assert_eq!(msg.timestamp, 1609459200.0);
}

#[test]
fn test_transfer_message_from_internal_tx() {
    let tx = test_internal_tx();
    let msg = TransferMessage::from(tx);
    assert_eq!(msg.user_from, 1);
    assert_eq!(msg.user_to, 2);
    assert_eq!(msg.asset, "USDT");
    assert_eq!(msg.amount, "500.0");
    assert_eq!(msg.signature, "\x05\x06\x07\x08");
}

#[test]
fn test_order_message_from_order() {
    let order = test_order();
    let msg = OrderMessage::from_order(&order, OrderEventType::PUT);
    assert_eq!(msg.event, OrderEventType::PUT);
    assert_eq!(msg.base, "ETH");
    assert_eq!(msg.quote, "USDT");
    assert_eq!(msg.order.id, 123);
}

#[test]
fn test_message_enum_serialization() {
    let order = test_order();
    let order_msg = OrderMessage::from_order(&order, OrderEventType::PUT);
    let msg = Message::OrderMessage(Box::new(order_msg));
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"OrderMessage\""));
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    match deserialized {
        Message::OrderMessage(om) => assert_eq!(om.event, OrderEventType::PUT),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn test_balance_message_serialization() {
    let bh = test_balance_history();
    let msg = BalanceMessage::from(&bh);
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"user_id\":1"));
    assert!(json.contains("\"asset\":\"ETH\""));
}

// --- persist.rs tests ---

use super::persist::*;

#[test]
fn test_notify_track_item_val() {
    let left = NotifyTrackItem::Left(10);
    let right = NotifyTrackItem::Right(20);
    assert_eq!(left.val(), 10);
    assert_eq!(right.val(), 20);
    assert!(left.is_left());
    assert!(!left.is_right());
    assert!(!right.is_left());
    assert!(right.is_right());
}

#[test]
fn test_notify_track_item_resolve() {
    let mut a = NotifyTrackItem::Left(5);
    let b = NotifyTrackItem::Right(10);
    // a < b, so a gets updated to b, returns a's old value
    assert_eq!(a.resolve(b), 5);
    assert_eq!(a.val(), 10);

    let mut c = NotifyTrackItem::Left(20);
    let d = NotifyTrackItem::Right(15);
    // d < c, so d is smaller, c stays, returns d's value
    assert_eq!(c.resolve(d), 15);
    assert_eq!(c.val(), 20);
}

#[test]
fn test_notify_track_item_merge_same_side() {
    let mut a = NotifyTrackItem::Left(5);
    let b = NotifyTrackItem::Left(10);
    // same side: max, no resolved value
    assert_eq!(a.merge(b), None);
    assert_eq!(a.val(), 10);

    let mut c = NotifyTrackItem::Right(20);
    let d = NotifyTrackItem::Right(15);
    assert_eq!(c.merge(d), None);
    assert_eq!(c.val(), 20);
}

#[test]
fn test_notify_track_item_merge_cross_side() {
    let mut a = NotifyTrackItem::Left(5);
    let b = NotifyTrackItem::Right(10);
    // cross side: resolve, returns smaller value
    assert_eq!(a.merge(b), Some(5));
    assert_eq!(a.val(), 10);
}

#[test]
fn test_notify_tracker_merge() {
    let mut tracker = NotifyTracker(HashMap::new());
    let mut other = NotifyTracker(HashMap::new());

    // Insert some items
    tracker.insert(0, NotifyTrackItem::Left(5));
    tracker.insert(1, NotifyTrackItem::Left(10));

    other.insert(0, NotifyTrackItem::Right(8)); // cross, resolves to 5
    other.insert(1, NotifyTrackItem::Right(15)); // cross, resolves to 10
    other.insert(2, NotifyTrackItem::Left(20)); // new key

    let resolved = tracker.merge(other);
    assert_eq!(resolved.get(&0), Some(&5u64));
    assert_eq!(resolved.get(&1), Some(&10u64));
    // key 2 was new in other, inserted into tracker but not resolved
    assert_eq!(tracker.get(&2), Some(&NotifyTrackItem::Left(20)));
}

#[test]
fn test_closed_order_transformer() {
    let order = test_order();
    let finish_msg = OrderMessage {
        event: OrderEventType::FINISH,
        order: order.clone(),
        base: "ETH".to_string(),
        quote: "USDT".to_string(),
    };
    let put_msg = OrderMessage {
        event: OrderEventType::PUT,
        order: order.clone(),
        base: "ETH".to_string(),
        quote: "USDT".to_string(),
    };

    let result = <ClosedOrder as MsgDataTransformer<models::OrderHistory>>::into(&finish_msg);
    assert!(result.is_some());
    let hist = result.unwrap();
    assert_eq!(hist.user_id, 1);

    let result = <ClosedOrder as MsgDataTransformer<models::OrderHistory>>::into(&put_msg);
    assert!(result.is_none());
}

#[test]
fn test_ask_trade_transformer() {
    let trade = test_trade();
    let user_trade = <AskTrade as MsgDataTransformer<models::UserTrade>>::into(&trade).unwrap();
    assert_eq!(user_trade.user_id, 2);
    assert_eq!(user_trade.side, crate::types::OrderSide::ASK as i16);
    assert_eq!(user_trade.order_id, 200);
    assert_eq!(user_trade.counter_order_id, 100);
    assert_eq!(user_trade.role, crate::types::MarketRole::TAKER as i16);
}

#[test]
fn test_bid_trade_transformer() {
    let trade = test_trade();
    let user_trade = <BidTrade as MsgDataTransformer<models::UserTrade>>::into(&trade).unwrap();
    assert_eq!(user_trade.user_id, 1);
    assert_eq!(user_trade.side, crate::types::OrderSide::BID as i16);
    assert_eq!(user_trade.order_id, 100);
    assert_eq!(user_trade.counter_order_id, 200);
    assert_eq!(user_trade.role, crate::types::MarketRole::MAKER as i16);
}

#[test]
fn test_trade_into_market_trade() {
    let trade = test_trade();
    let mt: models::MarketTrade = (&trade).into();
    assert_eq!(mt.market, "ETH_USDT");
    assert_eq!(mt.trade_id, 999);
    assert_eq!(mt.price, test_decimal("1500.0"));
    assert_eq!(mt.amount, test_decimal("1.0"));
    assert_eq!(mt.quote_amount, test_decimal("1500.0"));
    // ask_order_id (200) > bid_order_id (100), so taker_side = ASK
    assert_eq!(mt.taker_side, crate::types::OrderSide::ASK);
}

#[test]
fn test_balance_message_into_balance_history() {
    let bh = test_balance_history();
    let msg = BalanceMessage::from(&bh);
    let back: models::BalanceHistory = (&msg).into();
    assert_eq!(back.user_id, 1);
    assert_eq!(back.asset, "ETH");
    assert_eq!(back.business, "deposit");
    assert_eq!(back.market_price, test_decimal("1500.50"));
    assert_eq!(back.change, test_decimal("10.5"));
    assert_eq!(back.balance, test_decimal("100.0"));
    assert_eq!(back.balance_available, test_decimal("90.0"));
    assert_eq!(back.balance_frozen, test_decimal("10.0"));
    assert_eq!(back.detail, "test detail");
    assert_eq!(back.signature, vec![1, 2, 3, 4]);
}

#[test]
fn test_user_message_into_account_desc() {
    let msg = UserMessage {
        user_id: 99,
        l1_address: "0xabc".to_string(),
        l2_pubkey: "0xdef".to_string(),
    };
    let acc: models::AccountDesc = (&msg).into();
    assert_eq!(acc.id, 99);
    assert_eq!(acc.l1_address, "0xabc");
    assert_eq!(acc.l2_pubkey, "0xdef");
}

#[test]
fn test_transfer_message_into_internal_tx() {
    let msg = TransferMessage {
        time: 1609459200.0,
        user_from: 1,
        user_to: 2,
        asset: "USDT".to_string(),
        amount: "500.0".to_string(),
        signature: "hello".to_string(),
    };
    let tx: models::InternalTx = (&msg).into();
    assert_eq!(tx.user_from, 1);
    assert_eq!(tx.user_to, 2);
    assert_eq!(tx.asset, "USDT");
    assert_eq!(tx.amount, test_decimal("500.0"));
    assert_eq!(tx.signature, b"hello");
}

#[test]
fn test_order_message_into_order_history() {
    let order = test_order();
    let msg = OrderMessage::from_order(&order, OrderEventType::FINISH);
    let hist: models::OrderHistory = (&msg).into();
    assert_eq!(hist.user_id, 1);
    assert_eq!(hist.market, "ETH_USDT");
    assert_eq!(hist.order_side, crate::types::OrderSide::BID);
}
