use super::asset_manager::AssetManager;
use crate::config;
pub use crate::models::BalanceHistory;

use anyhow::Result;
use fluidex_common::rust_decimal::prelude::Zero;
use fluidex_common::rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::dto::UserIdentifier;
use num_enum::TryFromPrimitive;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash, Copy, TryFromPrimitive)]
#[repr(i16)]
pub enum BalanceType {
    AVAILABLE = 1,
    FREEZE = 2,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
pub struct BalanceMapKey {
    pub user_id: String,
    pub broker_id: String,
    pub account_id: String,
    pub balance_type: BalanceType,
    pub asset: String,
}

#[derive(Default)]
pub struct BalanceStatus {
    pub total: Decimal,
    pub available_count: u32,
    pub available: Decimal,
    pub frozen_count: u32,
    pub frozen: Decimal,
}

//#[derive(default)]
pub struct BalanceManager {
    pub asset_manager: AssetManager,
    pub balances: HashMap<BalanceMapKey, Decimal>,
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
    pub fn get(&self, user_info: UserIdentifier, balance_type: BalanceType, asset: &str) -> Decimal {
        self.get_by_key(&BalanceMapKey {
            user_id: user_info.user_id,
            broker_id: user_info.broker_id,
            account_id: user_info.account_id,
            balance_type,
            asset: asset.to_owned(),
        })
    }
    pub fn get_with_round(&self, user_info: UserIdentifier, balance_type: BalanceType, asset: &str) -> Decimal {
        let balance: Decimal = self.get(user_info, balance_type, asset);
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
    pub fn del(&mut self, user_info: UserIdentifier, balance_type: BalanceType, asset: &str) {
        self.balances.remove(&BalanceMapKey {
            user_id: user_info.user_id,
            broker_id: user_info.broker_id,
            account_id: user_info.account_id,
            balance_type,
            asset: asset.to_owned(),
        });
    }
    pub fn set(&mut self, user_info: UserIdentifier, balance_type: BalanceType, asset: &str, amount: &Decimal) {
        let key = BalanceMapKey {
            user_id: user_info.user_id,
            broker_id: user_info.broker_id,
            account_id: user_info.account_id,
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
    pub fn add(&mut self, user_info: UserIdentifier, balance_type: BalanceType, asset: &str, amount: &Decimal) -> Decimal {
        debug_assert!(amount.is_sign_positive());
        let amount = amount.round_dp(self.asset_manager.asset_prec(asset));
        let key = BalanceMapKey {
            user_id: user_info.user_id,
            broker_id: user_info.broker_id,
            account_id: user_info.account_id,
            balance_type,
            asset: asset.to_owned(),
        };
        let old_value = self.get_by_key(&key);
        let new_value = old_value + amount;
        self.set_by_key(key, &new_value);
        new_value
    }
    pub fn sub(&mut self, user_info: UserIdentifier, balance_type: BalanceType, asset: &str, amount: &Decimal) -> Decimal {
        debug_assert!(amount.is_sign_positive());
        let amount = amount.round_dp(self.asset_manager.asset_prec(asset));
        let key = BalanceMapKey {
            user_id: user_info.user_id,
            broker_id: user_info.broker_id,
            account_id: user_info.account_id,
            balance_type,
            asset: asset.to_owned(),
        };
        let old_value = self.get_by_key(&key);
        debug_assert!(old_value.ge(&amount));
        let new_value = old_value - amount;
        debug_assert!(new_value.is_sign_positive());
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
    pub fn frozen(&mut self, user_info: UserIdentifier, asset: &str, amount: &Decimal) {
        debug_assert!(amount.is_sign_positive());
        let amount = amount.round_dp(self.asset_manager.asset_prec(asset));
        let key = BalanceMapKey {
            user_id: user_info.user_id.clone(),
            broker_id: user_info.broker_id.clone(),
            account_id: user_info.account_id.clone(),
            balance_type: BalanceType::AVAILABLE,
            asset: asset.to_owned(),
        };
        let old_available_value = self.get_by_key(&key);
        debug_assert!(old_available_value.ge(&amount));
        self.sub(user_info.clone(), BalanceType::AVAILABLE, asset, &amount);
        self.add(user_info, BalanceType::FREEZE, asset, &amount);
    }
    pub fn unfrozen(&mut self, user_info: UserIdentifier, asset: &str, amount: &Decimal) {
        debug_assert!(amount.is_sign_positive());
        let amount = amount.round_dp(self.asset_manager.asset_prec(asset));
        let key = BalanceMapKey {
            user_id: user_info.user_id.clone(),
            broker_id: user_info.broker_id.clone(),
            account_id: user_info.account_id.clone(),
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
        self.add(user_info.clone(), BalanceType::AVAILABLE, asset, &amount);
        self.sub(user_info, BalanceType::FREEZE, asset, &amount);
    }
    pub fn total(&self, user_info: UserIdentifier, asset: &str) -> Decimal {
        self.get(user_info.clone(), BalanceType::AVAILABLE, asset) + self.get(user_info, BalanceType::FREEZE, asset)
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
