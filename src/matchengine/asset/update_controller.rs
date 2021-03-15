use super::balance_manager::{BalanceManager, BalanceType};
use super::persist::PersistExector;
use crate::models;
use crate::utils;
use crate::utils::FTimestamp;
pub use models::BalanceHistory;

use anyhow::{bail, Result};
use rust_decimal::Decimal;
use ttl_cache::TtlCache;

use std::time::Duration;

const BALANCE_MAP_INIT_SIZE_ASSET: usize = 64;

#[derive(PartialEq, Eq, Hash)]
struct BalanceUpdateKey {
    pub user_id: u32,
    pub asset: String,
    pub business: String,
    pub business_id: u64,
}

//pub trait BalanceUpdateValidator {
//    pub fn is_valid()
//}

// TODO: this class needs to be refactored
// Currently it has two purpose: (1) filter duplicate (2) generate message
pub struct BalanceUpdateController {
    cache: TtlCache<BalanceUpdateKey, bool>,
}

impl BalanceUpdateController {
    pub fn new() -> BalanceUpdateController {
        let capacity = 1_000_000;
        BalanceUpdateController {
            cache: TtlCache::new(capacity),
        }
    }
    pub fn reset(&mut self) {
        self.cache.clear()
    }
    pub fn on_timer(&mut self) {
        self.cache.clear()
    }
    pub fn timer_interval(&self) -> Duration {
        Duration::from_secs(60)
    }
    // return false if duplicate
    pub fn update_user_balance(
        &mut self,
        balance_manager: &mut BalanceManager,
        mut persistor: impl PersistExector,
        user_id: u32,
        asset: &str,
        business: String,
        business_id: u64,
        change: Decimal,
        mut detail: serde_json::Value,
    ) -> Result<()> {
        let cache_key = BalanceUpdateKey {
            user_id,
            asset: asset.to_string(),
            business: business.clone(),
            business_id,
        };
        if self.cache.contains_key(&cache_key) {
            bail!("duplicate request");
        }
        let old_balance = balance_manager.get(user_id, BalanceType::AVAILABLE, &asset);
        let abs_change = change.abs();
        let new_balance = if change.is_sign_positive() {
            balance_manager.add(user_id, BalanceType::AVAILABLE, &asset, &abs_change)
        } else if change.is_sign_negative() {
            if old_balance < abs_change {
                bail!("balance not enough");
            }
            balance_manager.sub(user_id, BalanceType::AVAILABLE, &asset, &abs_change)
        } else {
            old_balance
        };
        log::debug!("change user balance: {} {} {}", user_id, asset, change);
        self.cache.insert(cache_key, true, Duration::from_secs(3600));

        if persistor.real_persist() {
            detail["id"] = serde_json::Value::from(business_id);
            persistor.put_balance(BalanceHistory {
                time: FTimestamp(utils::current_timestamp()).into(),
                user_id: user_id as i32,
                asset: asset.to_string(),
                business,
                change,
                balance: new_balance,
                detail: detail.to_string(),
            });
        }
        Ok(())
    }
}

impl Default for BalanceUpdateController {
    fn default() -> Self {
        Self::new()
    }
}
