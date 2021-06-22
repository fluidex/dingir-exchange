pub mod timeutil;
pub use timeutil::*;
pub mod strings;
pub use strings::*;

use crate::server::OrderPutRequest;
pub fn order_hash(_req: &OrderPutRequest) -> String {
    String::default()
}

// TODO: use proper types?
pub fn eddsa_verify(_pubkey: &str, _msg: &str, _signature: &str) -> bool {
    true
}
