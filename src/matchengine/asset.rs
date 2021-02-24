use crate::history::HistoryWriter;
use crate::message::{BalanceMessage, MessageManager};
use crate::models;
use crate::utils;
use crate::{config, utils::FTimestamp};
pub use models::BalanceHistory;

use anyhow::{bail, Result};
use rust_decimal::prelude::Zero;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ttl_cache::TtlCache;

use num_enum::TryFromPrimitive;
use std::collections::HashMap;

use std::time::Duration;

const BALANCE_MAP_INIT_SIZE_ASSET: usize = 64;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash, Copy, TryFromPrimitive)]
#[repr(i16)]
pub enum BalanceType {
    AVAILABLE = 1,
    FREEZE = 2,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
pub struct BalanceMapKey {
    pub user_id: u32,
    pub balance_type: BalanceType,
    pub asset: String,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
pub struct AssetInfo {
    pub prec_save: u32,
    pub prec_show: u32,
}

#[derive(Clone)]
pub struct AssetManager {
    pub assets: HashMap<String, AssetInfo>,
}

impl AssetManager {
    pub fn new(asset_config: &[config::Asset]) -> Result<AssetManager> {
        println!("asset {:?}", asset_config);
        let mut assets = HashMap::new();
        for item in asset_config.iter() {
            assets.insert(
                item.name.clone(),
                AssetInfo {
                    prec_save: item.prec_save,
                    prec_show: item.prec_show,
                },
            );
        }
        Ok(AssetManager { assets })
    }
    pub fn asset_exist(&self, name: &str) -> bool {
        self.assets.contains_key(name)
    }
    pub fn asset_get(&self, name: &str) -> Option<&AssetInfo> {
        self.assets.get(name)
    }
    pub fn asset_prec(&self, name: &str) -> u32 {
        self.asset_get(name).unwrap().prec_save
    }
    pub fn asset_prec_show(&self, name: &str) -> u32 {
        self.asset_get(name).unwrap().prec_show
    }
}

//#[derive(default)]
pub struct BalanceManager {
    pub asset_manager: AssetManager,
    pub balances: HashMap<BalanceMapKey, Decimal>,
}

#[derive(Default)]
pub struct BalanceStatus {
    pub total: Decimal,
    pub available_count: u32,
    pub available: Decimal,
    pub frozen_count: u32,
    pub frozen: Decimal,
}

impl BalanceManager {
    pub fn new(asset_config: &[config::Asset]) -> Result<BalanceManager> {
        let asset_manager = AssetManager::new(asset_config)?;
        Ok(BalanceManager {
            asset_manager,
            balances: HashMap::new(),
        })
    }

    pub fn reset(&mut self) {
        self.balances.clear()
    }
    pub fn get(&self, user_id: u32, balance_type: BalanceType, asset: &str) -> Decimal {
        self.get_by_key(&BalanceMapKey {
            user_id,
            balance_type,
            asset: asset.to_owned(),
        })
    }
    pub fn get_with_round(&self, user_id: u32, balance_type: BalanceType, asset: &str) -> Decimal {
        let balance: Decimal = self.get(user_id, balance_type, asset);
        let prec_save = self.asset_manager.asset_prec(asset);
        let prec_show = self.asset_manager.asset_prec_show(asset);
        let balance_show = if prec_save == prec_show {
            balance
        } else {
            balance.round_dp(prec_show)
        };
        balance_show
    }
    pub fn get_by_key(&self, key: &BalanceMapKey) -> Decimal {
        *self.balances.get(key).unwrap_or(&Decimal::zero())
    }
    pub fn del(&mut self, user_id: u32, balance_type: BalanceType, asset: &str) {
        self.balances.remove(&BalanceMapKey {
            user_id,
            balance_type,
            asset: asset.to_owned(),
        });
    }
    pub fn set(&mut self, user_id: u32, balance_type: BalanceType, asset: &str, amount: &Decimal) {
        let key = BalanceMapKey {
            user_id,
            balance_type,
            asset: asset.to_owned(),
        };
        self.set_by_key(key, amount);
    }
    pub fn set_by_key(&mut self, key: BalanceMapKey, amount: &Decimal) {
        debug_assert!(amount.is_sign_positive());
        let amount = amount.round_dp(self.asset_manager.asset_prec(&key.asset));
        //log::debug!("set balance: {:?}, {}", key, amount);
        self.balances.insert(key, amount);
    }
    pub fn add(&mut self, user_id: u32, balance_type: BalanceType, asset: &str, amount: &Decimal) -> Decimal {
        debug_assert!(amount.is_sign_positive());
        let amount = amount.round_dp(self.asset_manager.asset_prec(asset));
        let key = BalanceMapKey {
            user_id,
            balance_type,
            asset: asset.to_owned(),
        };
        let old_value = self.get_by_key(&key);
        let new_value = old_value + amount;
        self.set_by_key(key, &new_value);
        new_value
    }
    pub fn sub(&mut self, user_id: u32, balance_type: BalanceType, asset: &str, amount: &Decimal) -> Decimal {
        debug_assert!(amount.is_sign_positive());
        let amount = amount.round_dp(self.asset_manager.asset_prec(asset));
        let key = BalanceMapKey {
            user_id,
            balance_type,
            asset: asset.to_owned(),
        };
        let old_value = self.get_by_key(&key);
        debug_assert!(old_value.ge(&amount));
        let new_value = old_value - amount;
        // TODO don't remove it. Skip when sql insert
        /*
        if result.is_zero() {
            self.balances.remove(&key);
        } else {
            self.balances.insert(key, result);
        }
        */
        self.set_by_key(key, &new_value);
        new_value
    }
    pub fn frozen(&mut self, user_id: u32, asset: &str, amount: &Decimal) {
        debug_assert!(amount.is_sign_positive());
        let amount = amount.round_dp(self.asset_manager.asset_prec(asset));
        let key = BalanceMapKey {
            user_id,
            balance_type: BalanceType::AVAILABLE,
            asset: asset.to_owned(),
        };
        let old_available_value = self.get_by_key(&key);
        debug_assert!(old_available_value.ge(&amount));
        self.sub(user_id, BalanceType::AVAILABLE, asset, &amount);
        self.add(user_id, BalanceType::FREEZE, asset, &amount);
    }
    pub fn unfrozen(&mut self, user_id: u32, asset: &str, amount: &Decimal) {
        debug_assert!(amount.is_sign_positive());
        let amount = amount.round_dp(self.asset_manager.asset_prec(asset));
        let key = BalanceMapKey {
            user_id,
            balance_type: BalanceType::FREEZE,
            asset: asset.to_owned(),
        };
        let old_frozen_value = self.get_by_key(&key);
        debug_assert!(
            old_frozen_value.ge(&amount),
            "unfreeze larger than frozen {} > {}",
            amount,
            old_frozen_value
        );
        self.add(user_id, BalanceType::AVAILABLE, asset, &amount);
        self.sub(user_id, BalanceType::FREEZE, asset, &amount);
    }
    pub fn total(&self, user_id: u32, asset: &str) -> Decimal {
        self.get(user_id, BalanceType::AVAILABLE, asset) + self.get(user_id, BalanceType::FREEZE, asset)
    }
    pub fn status(&self, asset: &str) -> BalanceStatus {
        let mut result = BalanceStatus::default();
        for (k, amount) in self.balances.iter() {
            if k.asset.eq(asset) && !amount.is_zero() {
                result.total += amount;
                if k.balance_type == BalanceType::AVAILABLE {
                    result.available_count += 1;
                    result.available += amount;
                } else {
                    result.frozen_count += 1;
                    result.frozen += amount;
                }
            }
        }
        result
    }
}

#[derive(PartialEq, Eq, Hash)]
struct BalanceUpdateKey {
    pub user_id: u32,
    pub asset: String,
    pub business: String,
    pub business_id: u64,
}

pub struct BalanceUpdateController {
    cache: TtlCache<BalanceUpdateKey, bool>,
}

pub trait PersistExector {
    fn real_persist(&self) -> bool {
        true
    }
    fn put_balance(&mut self, balance: BalanceHistory);
}

impl PersistExector for Box<dyn PersistExector + '_> {
    fn put_balance(&mut self, balance: BalanceHistory) {
        self.as_mut().put_balance(balance)
    }
}

pub(super) struct DummyPersistor(pub(super) bool);
impl PersistExector for DummyPersistor {
    fn real_persist(&self) -> bool {
        self.0
    }
    fn put_balance(&mut self, _balance: BalanceHistory) {}
}

pub(super) struct MessengerAsPersistor<'a, T>(&'a mut T);

impl<T: MessageManager> PersistExector for MessengerAsPersistor<'_, T> {
    fn put_balance(&mut self, balance: BalanceHistory) {
        self.0.push_balance_message(&BalanceMessage {
            timestamp: balance.time.timestamp() as f64,
            user_id: balance.user_id as u32,
            asset: balance.asset.clone(),
            business: balance.business.clone(),
            change: balance.change.to_string(),
            balance: balance.balance.to_string(),
            detail: balance.detail,
        });
    }
}

pub(super) struct DBAsPersistor<'a, T>(&'a mut T);

impl<T: HistoryWriter> PersistExector for DBAsPersistor<'_, T> {
    fn put_balance(&mut self, balance: BalanceHistory) {
        self.0.append_balance_history(balance);
    }
}

impl<T1: PersistExector, T2: PersistExector> PersistExector for (T1, T2) {
    fn real_persist(&self) -> bool {
        self.0.real_persist() || self.1.real_persist()
    }
    fn put_balance(&mut self, balance: BalanceHistory) {
        self.0.put_balance(balance.clone());
        self.1.put_balance(balance);
    }
}

pub(super) fn persistor_for_message<T: MessageManager>(messenger: &mut T) -> MessengerAsPersistor<'_, T> {
    MessengerAsPersistor(messenger)
}

pub(super) fn persistor_for_db<T: HistoryWriter>(history_writer: &mut T) -> DBAsPersistor<'_, T> {
    DBAsPersistor(history_writer)
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
