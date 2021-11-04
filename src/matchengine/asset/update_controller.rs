use super::balance_manager::{BalanceManager, BalanceType};
use crate::models;
use crate::persist::PersistExector;
use fluidex_common::utils::timeutil::{current_timestamp, FTimestamp};
pub use models::BalanceHistory;

use anyhow::{bail, Result};
use fluidex_common::rust_decimal::Decimal;
use ttl_cache::TtlCache;

use std::time::Duration;

const BALANCE_MAP_INIT_SIZE_ASSET: usize = 64;
const PERSIST_ZERO_BALANCE_UPDATE: bool = false;

pub struct BalanceUpdateParams {
    pub balance_type: BalanceType,
    pub business_type: BusinessType,
    pub user_id: u32,
    pub business_id: u64,
    pub asset: String,
    pub business: String,
    pub market_price: Decimal,
    pub change: Decimal,
    pub detail: serde_json::Value,
    pub signature: Vec<u8>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum BusinessType {
    Deposit,
    Trade,
    Transfer,
    Withdraw,
}

#[derive(PartialEq, Eq, Hash)]
struct BalanceUpdateKey {
    pub balance_type: BalanceType,
    pub business_type: BusinessType,
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
        persistor: &mut impl PersistExector,
        mut params: BalanceUpdateParams,
    ) -> Result<()> {
        let asset = params.asset;
        let balance_type = params.balance_type;
        let business = params.business;
        let business_type = params.business_type;
        let business_id = params.business_id;
        let user_id = params.user_id;
        let cache_key = BalanceUpdateKey {
            balance_type,
            business_type,
            user_id,
            asset: asset.clone(),
            business: business.clone(),
            business_id,
        };
        if self.cache.contains_key(&cache_key) {
            bail!("duplicate request");
        }
        let old_balance = balance_manager.get(user_id, balance_type, &asset);
        let change = params.change;
        let abs_change = change.abs();
        if change.is_sign_positive() {
            balance_manager.add(user_id, balance_type, &asset, &abs_change);
        } else if change.is_sign_negative() {
            if old_balance < abs_change {
                bail!("balance not enough");
            }
            balance_manager.sub(user_id, balance_type, &asset, &abs_change);
        }
        log::debug!("change user balance: {} {} {}", user_id, asset, change);
        self.cache.insert(cache_key, true, Duration::from_secs(3600));
        if persistor.real_persist() && (PERSIST_ZERO_BALANCE_UPDATE || !change.is_zero()) {
            params.detail["id"] = serde_json::Value::from(business_id);
            let balance_available = balance_manager.get(user_id, BalanceType::AVAILABLE, &asset);
            let balance_frozen = balance_manager.get(user_id, BalanceType::FREEZE, &asset);
            let balance_history = BalanceHistory {
                time: FTimestamp(current_timestamp()).into(),
                user_id: user_id as i32,
                business_id: business_id as i64,
                asset,
                business,
                market_price: params.market_price,
                change,
                balance: balance_available + balance_frozen,
                balance_available,
                balance_frozen,
                detail: params.detail.to_string(),
                signature: params.signature,
            };
            persistor.put_balance(&balance_history);
            match params.business_type {
                BusinessType::Deposit => persistor.put_deposit(&balance_history),
                BusinessType::Withdraw => persistor.put_withdraw(&balance_history),
                _ => {}
            }
        }
        Ok(())
    }
}

impl Default for BalanceUpdateController {
    fn default() -> Self {
        Self::new()
    }
}
