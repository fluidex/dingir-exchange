#[derive(Default)]
pub struct Sequencer {
    order_id: u64,
    trade_id: u64,
    msg_id: u64,
    operation_log_id: u64,
}

impl Sequencer {
    pub fn reset(&mut self) {
        self.set_operation_log_id(0);
        self.set_order_id(0);
        self.set_trade_id(0);
        self.set_msg_id(0);
    }
    pub fn next_order_id(&mut self) -> u64 {
        self.order_id += 1;
        //log::debug!("next_order_id {}", self.order_id);
        self.order_id
    }
    pub fn next_trade_id(&mut self) -> u64 {
        self.trade_id += 1;
        self.trade_id
    }
    pub fn next_operation_log_id(&mut self) -> u64 {
        self.operation_log_id += 1;
        self.operation_log_id
    }
    pub fn next_msg_id(&mut self) -> u64 {
        self.msg_id += 1;
        self.msg_id
    }
    pub fn get_operation_log_id(&self) -> u64 {
        self.operation_log_id
    }
    pub fn get_trade_id(&self) -> u64 {
        self.trade_id
    }
    pub fn get_order_id(&self) -> u64 {
        self.order_id
    }
    pub fn get_msg_id(&self) -> u64 {
        self.msg_id
    }
    pub fn set_operation_log_id(&mut self, id: u64) {
        log::debug!("set operation_log id {}", id);
        self.operation_log_id = id;
    }
    pub fn set_trade_id(&mut self, id: u64) {
        log::debug!("set trade id {}", id);
        self.trade_id = id;
    }
    pub fn set_order_id(&mut self, id: u64) {
        log::debug!("set order id {}", id);
        self.order_id = id;
    }
    pub fn set_msg_id(&mut self, id: u64) {
        log::debug!("set msg id {}", id);
        self.msg_id = id;
    }
}
