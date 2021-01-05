use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sqlx::types::chrono::{NaiveDateTime};

pub fn system_time_to_timestamp(t: SystemTime) -> f64 {
    t.duration_since(UNIX_EPOCH).unwrap().as_micros() as f64 / 1_000_000_f64
}

pub fn timestamp_to_system_time(t: f64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs_f64(t)
}

pub fn current_system_time() -> SystemTime {
    SystemTime::now()
}

pub fn current_timestamp() -> f64 {
    system_time_to_timestamp(current_system_time())
}

pub struct FTimestamp(pub f64);

impl Into<NaiveDateTime> for FTimestamp
{
    fn into(self) -> NaiveDateTime
    {
        NaiveDateTime::from_timestamp(self.0 as i64, 0)
    }
}

impl From<&NaiveDateTime> for FTimestamp
{
    fn from(f : &NaiveDateTime) -> FTimestamp
    {
        FTimestamp(f.timestamp() as f64)
    }
}

pub fn current_naive_time() -> NaiveDateTime
{
    chrono::Local::now().naive_local()
}