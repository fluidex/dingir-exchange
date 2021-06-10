use crate::config::PersistPolicy;
use crate::history::{DatabaseHistoryWriter, HistoryWriter};
use crate::message::{ChannelMessageManager, MessageManager, UnifyMessageManager};

use super::persistor::{persistor_for_db, persistor_for_message, DummyPersistor, PersistExector};

pub struct DefaultPersistor {
    pub history_writer: DatabaseHistoryWriter,
    pub message_manager: Option<ChannelMessageManager>,
    pub policy: PersistPolicy,
}

pub struct PersistorGen<'c> {
    base: &'c mut DefaultPersistor,
    policy: PersistPolicy,
}

impl<'c> PersistorGen<'c> {
    pub fn get_persistor(self) -> Box<dyn PersistExector + 'c> {
        match self.policy {
            PersistPolicy::Dummy => Box::new(DummyPersistor::new(false)),
            PersistPolicy::ToDB => Box::new(persistor_for_db(&mut self.base.history_writer)),
            PersistPolicy::ToMessage => Box::new(persistor_for_message(self.base.message_manager.as_mut().unwrap())),
            PersistPolicy::Both => Box::new((
                persistor_for_db(&mut self.base.history_writer),
                persistor_for_message(self.base.message_manager.as_mut().unwrap()),
            )),
        }
    }
}

impl DefaultPersistor {
    pub fn is_real(&mut self, real: bool) -> PersistorGen<'_> {
        let policy = if real { self.policy } else { PersistPolicy::Dummy };
        PersistorGen { base: self, policy }
    }

    pub fn service_available(&self) -> bool {
        //if self.message_manager.as_ref().map(ChannelMessageManager::is_block).unwrap_or(true) {
        if self.message_manager.is_some() && self.message_manager.as_ref().unwrap().is_block() {
            log::warn!("message_manager full");
            return false;
        }
        if self.history_writer.is_block() {
            log::warn!("history_writer full");
            return false;
        }
        true
    }
}

pub trait IntoPersistor {
    fn service_available(&self) -> bool {
        true
    }
    fn get_persistor<'c>(&'c mut self, real: bool) -> Box<dyn PersistExector + 'c>;
}

impl IntoPersistor for DefaultPersistor {
    fn service_available(&self) -> bool {
        self.service_available()
    }
    fn get_persistor<'c>(&'c mut self, real: bool) -> Box<dyn PersistExector + 'c> {
        self.is_real(real).get_persistor()
    }
}

impl IntoPersistor for UnifyMessageManager {
    fn service_available(&self) -> bool {
        !self.is_block()
    }
    fn get_persistor<'c>(&'c mut self, _real: bool) -> Box<dyn PersistExector + 'c> {
        Box::new(persistor_for_message(self))
    }
}
