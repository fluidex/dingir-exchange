use orchestra::rpc::exchange;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct EthLogGuard {
    block_number: u64,
    history: HashSet<EthLogMetadata>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct EthLogMetadata {
    pub block_number: u64,
    pub tx_hash: String,
    pub log_index: String,
}

impl EthLogGuard {
    pub fn new(block_number: u64) -> Self {
        Self {
            block_number,
            history: HashSet::new(),
        }
    }

    pub fn accept(&self, log_meta: &EthLogMetadata) -> bool {
        if log_meta.block_number < self.block_number || (log_meta.block_number == self.block_number && self.history.contains(log_meta)) {
            false
        } else {
            true
        }
    }

    pub fn accept_optional(&self, log_meta: &Option<EthLogMetadata>) -> bool {
        match log_meta {
            None => true,
            Some(meta) => self.accept(meta),
        }
    }

    pub fn update(&mut self, log_meta: EthLogMetadata) {
        assert!(self.accept(&log_meta));

        if log_meta.block_number > self.block_number {
            self.block_number = log_meta.block_number;
            self.history.clear();
        }

        assert!(self.history.insert(log_meta));
    }

    pub fn update_optional(&mut self, log_meta: Option<EthLogMetadata>) {
        if let Some(meta) = log_meta {
            self.update(meta)
        }
    }
}

impl From<&exchange::EthLogMetadata> for EthLogMetadata {
    fn from(e: &exchange::EthLogMetadata) -> Self {
        Self {
            block_number: e.block_number,
            tx_hash: e.tx_hash.clone(),
            log_index: e.log_index.clone(),
        }
    }
}
