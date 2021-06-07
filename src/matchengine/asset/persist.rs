use crate::history::HistoryWriter;
use crate::message::{BalanceMessage, MessageManager, TransferMessage};

pub use crate::models::{BalanceHistory, InternalTx};

pub trait PersistExector {
    fn real_persist(&self) -> bool {
        true
    }
    fn put_balance(&mut self, balance: BalanceHistory);
    fn put_transfer(&mut self, tx: InternalTx);
}

impl PersistExector for Box<dyn PersistExector + '_> {
    fn put_balance(&mut self, balance: BalanceHistory) {
        self.as_mut().put_balance(balance)
    }
    fn put_transfer(&mut self, tx: InternalTx) {
        self.as_mut().put_transfer(tx)
    }
}

pub struct DummyPersistor(pub bool);
impl PersistExector for DummyPersistor {
    fn real_persist(&self) -> bool {
        self.0
    }
    fn put_balance(&mut self, _balance: BalanceHistory) {}
    fn put_transfer(&mut self, _tx: InternalTx) {}
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
            time: tx.time.timestamp() as f64, // TODO: use milli seconds
            user_from: tx.user_from as u32,
            user_to: tx.user_to as u32,
            asset: tx.asset.to_string(),
            amount: tx.amount.to_string(),
        });
    }
}

pub struct DBAsPersistor<'a, T>(&'a mut T);

impl<T: HistoryWriter> PersistExector for DBAsPersistor<'_, T> {
    fn put_balance(&mut self, balance: BalanceHistory) {
        self.0.append_balance_history(balance);
    }
    fn put_transfer(&mut self, tx: InternalTx) {
        unimplemented!();
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
}

pub fn persistor_for_message<T: MessageManager>(messenger: &mut T) -> MessengerAsPersistor<'_, T> {
    MessengerAsPersistor(messenger)
}

pub fn persistor_for_db<T: HistoryWriter>(history_writer: &mut T) -> DBAsPersistor<'_, T> {
    DBAsPersistor(history_writer)
}
