use crate::history::HistoryWriter;
use crate::message::{BalanceMessage, MessageManager};

pub use crate::models::BalanceHistory;

pub trait PersistExector {
    fn real_persist(&self) -> bool {
        true
    }
    fn put_balance(&mut self, balance: BalanceHistory);
    fn register_user(&mut self, balance: BalanceHistory);
}

impl PersistExector for Box<dyn PersistExector + '_> {
    fn put_balance(&mut self, balance: BalanceHistory) {
        self.as_mut().put_balance(balance)
    }
    fn register_user(&mut self, balance: BalanceHistory) {
        self.as_mut().put_balance(balance)
    }
}

pub struct DummyPersistor(pub bool);
impl PersistExector for DummyPersistor {
    fn real_persist(&self) -> bool {
        self.0
    }
    fn put_balance(&mut self, _balance: BalanceHistory) {}
    fn register_user(&mut self, _balance: BalanceHistory) {}
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
    fn register_user(&mut self, balance: BalanceHistory) {
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
}

pub struct DBAsPersistor<'a, T>(&'a mut T);

impl<T: HistoryWriter> PersistExector for DBAsPersistor<'_, T> {
    fn put_balance(&mut self, balance: BalanceHistory) {
        self.0.append_balance_history(balance);
    }
    fn register_user(&mut self, balance: BalanceHistory) {
        self.0.append_balance_history(balance);
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
    fn register_user(&mut self, balance: BalanceHistory) {
        self.0.put_balance(balance.clone());
        self.1.put_balance(balance);
    }
}

pub fn persistor_for_message<T: MessageManager>(messenger: &mut T) -> MessengerAsPersistor<'_, T> {
    MessengerAsPersistor(messenger)
}

pub fn persistor_for_db<T: HistoryWriter>(history_writer: &mut T) -> DBAsPersistor<'_, T> {
    DBAsPersistor(history_writer)
}
