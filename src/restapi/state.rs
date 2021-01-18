use super::types::UserInfo;

use sqlx::postgres::Postgres;
use std::collections::HashMap;
use std::sync::Mutex;
pub struct AppState {
    pub user_addr_map: Mutex<HashMap<String, UserInfo>>,
    pub db: sqlx::pool::Pool<Postgres>,
}
