use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;
lazy_static! {
    pub static ref STRING_POOL: Mutex<HashMap<String, &'static str>> = Default::default();
}

pub fn intern_string(s: &str) -> &'static str {
    *STRING_POOL
        .lock()
        .unwrap()
        .entry(s.to_owned())
        .or_insert_with(|| Box::leak(s.to_string().into_boxed_str()))
}
