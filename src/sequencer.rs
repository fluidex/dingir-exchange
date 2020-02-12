#[derive(Default)]
pub struct Sequencer {
    order_id: u64,
    deal_id: u64,
    operation_log_id: u64,
}

impl Sequencer {
    pub fn next_order_id(&mut self) -> u64 {
        self.order_id += 1;
        self.order_id
    }
    pub fn next_deal_id(&mut self) -> u64 {
        self.deal_id += 1;
        self.deal_id
    }
    pub fn next_operation_log_id(&mut self) -> u64 {
        self.operation_log_id += 1;
        self.operation_log_id
    }
    pub fn get_operation_log_id(&self) -> u64 {
        self.operation_log_id
    }
    pub fn get_deal_id(&self) -> u64 {
        self.deal_id
    }
    pub fn get_order_id(&self) -> u64 {
        self.order_id
    }
    pub fn set_operation_log_id(&mut self, id: u64) {
        log::debug!("set operation_log id {}", id);
        self.operation_log_id = id;
    }
    pub fn set_deal_id(&mut self, id: u64) {
        log::debug!("set deal id {}", id);
        self.operation_log_id = id;
    }
    pub fn set_order_id(&mut self, id: u64) {
        log::debug!("set order id {}", id);
        self.operation_log_id = id;
    }
}
