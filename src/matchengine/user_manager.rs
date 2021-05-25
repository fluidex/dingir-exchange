use crate::message::UserMessage;
use crate::message::{ MessageManager};
pub use crate::models::AccountDesc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::history::HistoryWriter;


#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
pub struct UserInfo {
    pub l1_address: String,
    pub l2_pubkey: String,
}

#[derive(Clone)]
pub struct UserManager {
    pub users: HashMap<u32, UserInfo>,
}

impl UserManager {
    pub fn new() -> Self {
        Self { users: HashMap::new() }
    }
}

impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}

pub trait PersistExector {
    fn real_persist(&self) -> bool {
        true
    }
    fn register_user(&mut self, user: AccountDesc);
}

impl PersistExector for Box<dyn PersistExector + '_> {
    fn register_user(&mut self, user: AccountDesc) {
        self.as_mut().register_user(user)
    }
}

pub struct DummyPersistor(pub bool);
impl PersistExector for DummyPersistor {
    fn real_persist(&self) -> bool {
        self.0
    }
    fn register_user(&mut self, _user: AccountDesc) {}
}

pub(super) struct MessengerAsPersistor<'a, T>(&'a mut T);

impl<T: MessageManager> PersistExector for MessengerAsPersistor<'_, T> {
    fn register_user(&mut self, user: AccountDesc) {
        self.0.push_user_message(&UserMessage {
            user_id: user.id as u32,
            l1_address: user.l1_address,
            l2_pubkey: user.l2_pubkey,
        });
    }
}

impl<T1: PersistExector, T2: PersistExector> PersistExector for (T1, T2) {
    fn real_persist(&self) -> bool {
        self.0.real_persist() || self.1.real_persist()
    }
    fn register_user(&mut self, user: AccountDesc) {
        self.0.register_user(user.clone());
        self.1.register_user(user);
    }
}

pub(super) struct DBAsPersistor<'a, T>(&'a mut T);

impl<T: HistoryWriter> PersistExector for DBAsPersistor<'_, T> {
    fn register_user(&mut self, user: AccountDesc) {
        self.0.append_user(user);
    }
}

pub(super) fn persistor_for_message<T: MessageManager>(messenger: &mut T) -> MessengerAsPersistor<'_, T> {
    MessengerAsPersistor(messenger)
}

pub(super) fn persistor_for_db<T: HistoryWriter>(history_writer: &mut T) -> DBAsPersistor<'_, T> {
    DBAsPersistor(history_writer)
}

// pub fn persistor_for_message<T: MessageManager>(messenger: &mut T) -> MessengerAsPersistor<'_, T> {
//     MessengerAsPersistor(messenger)
// }

// pub fn persistor_for_db<T: HistoryWriter>(history_writer: &mut T) -> DBAsPersistor<'_, T> {
//     DBAsPersistor(history_writer)
// }


