use crate::history::HistoryWriter;
use crate::matchengine::market::Order;
use crate::matchengine::market::Trade;
use crate::message::UserMessage;
use crate::message::{BalanceMessage, MessageManager, OrderMessage, TransferMessage};
pub use crate::models::AccountDesc;
pub use crate::models::{BalanceHistory, InternalTx};
use crate::types::OrderEventType;
use crate::utils::FTimestamp;

pub trait PersistExector {
    fn real_persist(&self) -> bool {
        true
    }
    fn put_balance(&mut self, balance: BalanceHistory);
    fn put_transfer(&mut self, tx: InternalTx);
    fn put_order(&mut self, order: &Order, at_step: OrderEventType);
    fn put_trade(&mut self, trade: &Trade);
    fn register_user(&mut self, user: AccountDesc);
}

impl PersistExector for Box<dyn PersistExector + '_> {
    fn put_balance(&mut self, balance: BalanceHistory) {
        self.as_mut().put_balance(balance)
    }
    fn put_transfer(&mut self, tx: InternalTx) {
        self.as_mut().put_transfer(tx)
    }
    fn put_order(&mut self, order: &Order, at_step: OrderEventType) {
        self.as_mut().put_order(order, at_step)
    }
    fn put_trade(&mut self, trade: &Trade) {
        self.as_mut().put_trade(trade)
    }
    fn register_user(&mut self, user: AccountDesc) {
        self.as_mut().register_user(user)
    }
}

pub struct DummyPersistor {
    pub real_persist: bool,
}
impl DummyPersistor {
    pub fn new(real_persist: bool) -> Self {
        Self { real_persist }
    }
}
impl PersistExector for DummyPersistor {
    fn real_persist(&self) -> bool {
        self.real_persist
    }
    fn put_balance(&mut self, _balance: BalanceHistory) {}
    fn put_transfer(&mut self, _tx: InternalTx) {}
    fn put_order(&mut self, _order: &Order, _as_step: OrderEventType) {}
    fn put_trade(&mut self, _trade: &Trade) {}
    fn register_user(&mut self, _user: AccountDesc) {}
}

pub struct MessengerAsPersistor<'a, T>(&'a mut T);

impl<T: MessageManager> PersistExector for MessengerAsPersistor<'_, T> {
    fn put_balance(&mut self, balance: BalanceHistory) {
        self.0.push_balance_message(&BalanceMessage {
            timestamp: balance.time.timestamp() as f64,
            user_id: balance.user_id as u32,
            asset: balance.asset.clone(),
            business: balance.business.clone(),
            change: balance.change.to_string(),
            balance: balance.balance.to_string(),
            detail: balance.detail,
        });
    }
    fn put_transfer(&mut self, tx: InternalTx) {
        self.0.push_transfer_message(&TransferMessage {
            time: FTimestamp::from(&tx.time).into(),
            user_from: tx.user_from as u32,
            user_to: tx.user_to as u32,
            asset: tx.asset.to_string(),
            amount: tx.amount.to_string(),
        });
    }
    fn put_order(&mut self, order: &Order, at_step: OrderEventType) {
        self.0.push_order_message(&OrderMessage {
            event: at_step,
            order: order.clone(),
            base: order.base.to_string(),
            quote: order.quote.to_string(),
        });
    }
    fn put_trade(&mut self, trade: &Trade) {
        self.0.push_trade_message(trade);
    }
    fn register_user(&mut self, user: AccountDesc) {
        self.0.push_user_message(&UserMessage {
            user_id: user.id as u32,
            l1_address: user.l1_address,
            l2_pubkey: user.l2_pubkey,
        });
    }
}

pub struct DBAsPersistor<'a, T>(&'a mut T);

impl<T: HistoryWriter> PersistExector for DBAsPersistor<'_, T> {
    fn put_balance(&mut self, balance: BalanceHistory) {
        self.0.append_balance_history(balance);
    }
    fn put_transfer(&mut self, tx: InternalTx) {
        self.0.append_internal_transfer(tx);
    }
    fn put_order(&mut self, order: &Order, at_step: OrderEventType) {
        //only persist on finish
        match at_step {
            OrderEventType::FINISH => self.0.append_order_history(order),
            OrderEventType::EXPIRED => self.0.append_expired_order_history(order),
            OrderEventType::PUT => (),
            _ => (),
        }
    }
    fn put_trade(&mut self, trade: &Trade) {
        self.0.append_pair_user_trade(trade);
    }
    fn register_user(&mut self, user: AccountDesc) {
        self.0.append_user(user);
    }
}

impl<T1: PersistExector, T2: PersistExector> PersistExector for (T1, T2) {
    fn real_persist(&self) -> bool {
        self.0.real_persist() || self.1.real_persist()
    }
    fn put_balance(&mut self, balance: BalanceHistory) {
        self.0.put_balance(balance.clone());
        self.1.put_balance(balance);
    }
    fn put_transfer(&mut self, tx: InternalTx) {
        self.0.put_transfer(tx.clone());
        self.1.put_transfer(tx);
    }
    fn put_order(&mut self, order: &Order, at_step: OrderEventType) {
        self.0.put_order(order, at_step);
        self.1.put_order(order, at_step);
    }
    fn put_trade(&mut self, trade: &Trade) {
        self.0.put_trade(trade);
        self.1.put_trade(trade);
    }
    fn register_user(&mut self, user: AccountDesc) {
        self.0.register_user(user.clone());
        self.1.register_user(user);
    }
}

pub fn persistor_for_message<T: MessageManager>(messenger: &mut T) -> MessengerAsPersistor<'_, T> {
    MessengerAsPersistor(messenger)
}

pub fn persistor_for_db<T: HistoryWriter>(history_writer: &mut T) -> DBAsPersistor<'_, T> {
    DBAsPersistor(history_writer)
}
