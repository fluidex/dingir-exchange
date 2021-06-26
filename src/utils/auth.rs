use crate::matchengine::rpc::*;

pub fn order_hash(_req: &OrderPutRequest) -> String {
    String::default()
}

// TODO: use proper types?
pub fn eddsa_verify(_pubkey: &str, _msg: &str, _signature: &str) -> bool {
    true
}
