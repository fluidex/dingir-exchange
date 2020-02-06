use chrono::Utc;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_timestamp() -> f64 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap().as_micros() as f64 / 1_000_000_f64;
    since_the_epoch
    //current_native_date_time().timestamp_millis() as f64
}

pub fn timestamp_to_chrono(t: f64) -> chrono::NaiveDateTime {
    let sec = t as i64;
    let ns = ((t - sec as f64) * 1e9) as u32;
    chrono::NaiveDateTime::from_timestamp(sec, ns)
}

pub fn decimal_r2b(d: &rust_decimal::Decimal) -> bigdecimal::BigDecimal {
    bigdecimal::BigDecimal::from_str(&d.to_string()).unwrap()
}

pub fn current_native_date_time() -> chrono::NaiveDateTime {
    Utc::now().naive_utc()
}
