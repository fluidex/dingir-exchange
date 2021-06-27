use crate::config;
use crate::market::OrderCommitment;
use crate::matchengine::rpc::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        log::info!("asset {:?}", asset_config);
        let mut assets = HashMap::new();
        for item in asset_config.iter() {
            assets.insert(
                item.id.clone(),
                AssetInfo {
                    prec_save: item.prec_save,
                    prec_show: item.prec_show,
                },
            );
        }
        Ok(AssetManager { assets })
    }

    pub fn append(&mut self, asset_config: &[config::Asset]) {
        //log::info()
        for item in asset_config.iter() {
            let ret = self.assets.insert(
                item.id.clone(),
                AssetInfo {
                    prec_save: item.prec_save,
                    prec_show: item.prec_show,
                },
            );
            if ret.is_some() {
                log::info!("Update asset {}", item.id);
            } else {
                log::info!("Append new asset {}", item.id);
            }
        }
    }

    pub fn asset_exist(&self, id: &str) -> bool {
        self.assets.contains_key(id)
    }
    pub fn asset_get(&self, id: &str) -> Option<&AssetInfo> {
        self.assets.get(id)
    }
    pub fn asset_prec(&self, id: &str) -> u32 {
        self.asset_get(id).unwrap().prec_save
    }
    pub fn asset_prec_show(&self, id: &str) -> u32 {
        self.asset_get(id).unwrap().prec_show
    }

    // pub fn exchange_order_to_rollup_order
    pub fn commit_order(&self, o: &OrderPutRequest) -> Result<OrderCommitment> {
        unimplemented!()
    }
}
