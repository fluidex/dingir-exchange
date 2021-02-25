use crate::asset::{self, BalanceManager, BalanceType, BalanceUpdateController};
use crate::database::OperationLogSender;
use crate::storage::config::MarketConfigs;
use crate::market;
use crate::sequencer::Sequencer;
use crate::utils::FTimestamp;
use crate::{config, utils};
use anyhow::anyhow;
use rust_decimal::Decimal;
use serde_json::json;
use tonic::{self, Status};

//use rust_decimal::Decimal;
use crate::models::{self};
use crate::types;
use types::{ConnectionType, DbType, SimpleResult};

use crate::dto::*;

use crate::database::DatabaseWriterConfig;
use crate::message::{new_message_manager_with_kafka_backend, ChannelMessageManager};

use crate::history::DatabaseHistoryWriter;
use crate::history::HistoryWriter;
use rust_decimal::prelude::Zero;
use std::collections::HashMap;

use sqlx::Connection;
use sqlx::Executor;

use serde::Serialize;
use std::str::FromStr;

pub use config::PersistPolicy;

pub struct Persistor {
    history_writer: DatabaseHistoryWriter,
    message_manager: Option<ChannelMessageManager>,
    policy: PersistPolicy,
}

pub struct PersistorGen<'c> {
    base: &'c mut Persistor,
    policy: PersistPolicy,
}

impl<'c> PersistorGen<'c> {
    fn persist_for_market(self, market_tag: (String, String)) -> Box<dyn market::PersistExector + 'c> {
        match self.policy {
            PersistPolicy::Dummy => Box::new(market::DummyPersistor(false)),
            PersistPolicy::ToDB => Box::new(market::persistor_for_db(&mut self.base.history_writer)),
            PersistPolicy::ToMessage => Box::new(market::persistor_for_message(
                self.base.message_manager.as_mut().unwrap(),
                market_tag,
            )),
            PersistPolicy::Both => Box::new((
                market::persistor_for_db(&mut self.base.history_writer),
                market::persistor_for_message(self.base.message_manager.as_mut().unwrap(), market_tag),
            )),
        }
    }

    fn persist_for_balance(self) -> Box<dyn asset::PersistExector + 'c> {
        match self.policy {
            PersistPolicy::Dummy => Box::new(asset::DummyPersistor(false)),
            PersistPolicy::ToDB => Box::new(asset::persistor_for_db(&mut self.base.history_writer)),
            PersistPolicy::ToMessage => Box::new(asset::persistor_for_message(self.base.message_manager.as_mut().unwrap())),
            PersistPolicy::Both => Box::new((
                asset::persistor_for_db(&mut self.base.history_writer),
                asset::persistor_for_message(self.base.message_manager.as_mut().unwrap()),
            )),
        }
    }
}

impl Persistor {
    fn is_real(&mut self, real: bool) -> PersistorGen<'_> {
        let policy = if real { self.policy } else { PersistPolicy::Dummy };

        PersistorGen { base: self, policy }
    }

    fn service_available(&self) -> bool {
        if self.message_manager.as_ref().map(ChannelMessageManager::is_block).unwrap_or(true) {
            log::warn!("message_manager full");
            return false;
        }
        if self.history_writer.is_block() {
            log::warn!("history_writer full");
            return false;
        }
        true
    }
}

pub struct Controller {
    pub settings: config::Settings,
    pub sequencer: Sequencer,
    pub balance_manager: BalanceManager,
//    pub asset_manager: AssetManager,
    pub update_controller: BalanceUpdateController,
    pub markets: HashMap<String, market::Market>,
    pub log_handler: OperationLogSender,
    pub persistor: Persistor,
    dbg_pool: sqlx::Pool<DbType>,
    market_load_cfg: MarketConfigs,
    //pub(crate) rt: tokio::runtime::Handle,
}

const ORDER_LIST_MAX_LEN: usize = 100;
const OPERATION_BALANCE_UPDATE: &str = "balance_update";
const OPERATION_ORDER_CANCEL: &str = "order_cancel";
const OPERATION_ORDER_CANCEL_ALL: &str = "order_cancel_all";
const OPERATION_ORDER_PUT: &str = "order_put";

impl Controller {
    pub fn new(cfgs: (config::Settings, MarketConfigs)) -> Controller {
        let settings = cfgs.0;
        let history_pool = sqlx::Pool::<DbType>::connect_lazy(&settings.db_history).unwrap();
        let balance_manager = BalanceManager::new(&settings.assets).unwrap();
        let message_manager = new_message_manager_with_kafka_backend(&settings.brokers).unwrap();
        let history_writer = DatabaseHistoryWriter::new(
            &DatabaseWriterConfig {
                spawn_limit: 4,
                apply_benchmark: true,
                capability_limit: 8192,
            },
            &history_pool,
        )
        .unwrap();
        let update_controller = BalanceUpdateController::new();
//        let asset_manager = AssetManager::new(&settings.assets).unwrap();
        let sequencer = Sequencer::default();
        let mut markets = HashMap::new();
        for entry in &settings.markets {
            let market = market::Market::new(entry, &balance_manager).unwrap();
            markets.insert(entry.name.clone(), market);
        }
        let main_pool = if settings.db_log == settings.db_history {
            history_pool
        } else {
            sqlx::Pool::<DbType>::connect_lazy(&settings.db_log).unwrap()
        };

        let log_handler = OperationLogSender::new(&DatabaseWriterConfig {
            spawn_limit: 4,
            apply_benchmark: true,
            capability_limit: 8192,
        })
        .start_schedule(&main_pool)
        .unwrap();

        let persist_policy = settings.history_persist_policy;

        Controller {
            settings,
            sequencer,
//            asset_manager,
            balance_manager,
            update_controller,
            markets,
            log_handler,
            persistor: Persistor {
                history_writer,
                message_manager: Some(message_manager),
                policy: persist_policy,
            },
            dbg_pool: main_pool,
            market_load_cfg: cfgs.1,
            //            rt: tokio::runtime::Handle::current(),
        }
    }

    pub fn asset_list(&self, _req: AssetListRequest) -> Result<AssetListResponse, Status> {
        let result = AssetListResponse {
            asset_lists: self
                .settings
                .assets
                .iter()
                .map(|item| asset_list_response::AssetInfo {
                    name: item.name.clone(),
                    precision: item.prec_show,
                })
                .collect(),
        };
        Ok(result)
    }
    pub fn balance_query(&self, req: BalanceQueryRequest) -> Result<BalanceQueryResponse, Status> {
        let all_asset_param_valid = req
            .assets
            .iter()
            .all(|asset_param| self.settings.assets.iter().any(|asset| asset.name.eq(asset_param)));
        if !all_asset_param_valid {
            return Err(Status::invalid_argument("invalid asset"));
        }
        let query_assets = if req.assets.is_empty() {
            self.settings.assets.iter().map(|asset| asset.name.clone()).collect()
        } else {
            req.assets
        };
        let user_id = req.user_id;
        let balance_manager = &self.balance_manager;
        let balances = query_assets
            .into_iter()
            .map(|asset_name| {
                let available = balance_manager
                    .get_with_round(user_id, BalanceType::AVAILABLE, &asset_name)
                    .to_string();
                let frozen = balance_manager
                    .get_with_round(user_id, BalanceType::FREEZE, &asset_name)
                    .to_string();
                balance_query_response::AssetBalance {
                    asset_name,
                    available,
                    frozen,
                }
            })
            .collect();
        Ok(BalanceQueryResponse { balances })
    }
    pub fn order_query(&self, req: OrderQueryRequest) -> Result<OrderQueryResponse, Status> {
        if !self.markets.contains_key(&req.market) {
            return Err(Status::invalid_argument("invalid market"));
        }
        if req.user_id == 0 {
            return Err(Status::invalid_argument("invalid user_id"));
        }
        // TODO: magic number
        let max_order_num = 100;
        let default_order_num = 10;
        let limit = if req.limit <= 0 {
            default_order_num
        } else if req.limit > max_order_num {
            max_order_num
        } else {
            req.limit
        };
        let market = self
            .markets
            .get(&req.market)
            .ok_or_else(|| Status::invalid_argument("invalid market"))?;
        let total_order_count = market.users.get(&req.user_id).map(|order_map| order_map.len()).unwrap_or(0);
        let orders = market
            .users
            .get(&req.user_id)
            .map(|order_map| {
                order_map
                    .values()
                    .rev()
                    .skip(req.offset as usize)
                    .take(limit as usize)
                    .map(|order_rc| order_to_proto(&order_rc.borrow()))
                    .collect()
            })
            .unwrap_or_else(Vec::new);
        let result = OrderQueryResponse {
            offset: req.offset,
            limit,
            total: total_order_count as i32,
            orders,
        };
        Ok(result)
    }
    pub fn order_book_depth(&self, req: OrderBookDepthRequest) -> Result<OrderBookDepthResponse, Status> {
        // TODO cache
        let market = self
            .markets
            .get(&req.market)
            .ok_or_else(|| Status::invalid_argument("invalid market"))?;
        // TODO check interval
        let interval = if req.interval.is_empty() {
            Decimal::zero()
        } else {
            Decimal::from_str(&req.interval).map_err(|_| Status::invalid_argument("invalid interval"))?
        };
        let depth = market.depth(req.limit as usize, &interval);
        let convert = |price_info: &Vec<market::PriceInfo>| {
            price_info
                .iter()
                .map(|price_info| order_book_depth_response::PriceInfo {
                    price: price_info.price.to_string(),
                    amount: price_info.amount.to_string(),
                })
                .collect::<Vec<_>>()
        };
        Ok(OrderBookDepthResponse {
            asks: convert(&depth.asks),
            bids: convert(&depth.bids),
        })
    }

    pub fn order_detail(&self, req: OrderDetailRequest) -> Result<OrderInfo, Status> {
        let market = self
            .markets
            .get(&req.market)
            .ok_or_else(|| Status::invalid_argument("invalid market"))?;
        let order = market
            .get(req.order_id)
            .ok_or_else(|| Status::invalid_argument("invalid order_id"))?;
        Ok(order_to_proto(&order))
    }

    pub fn market_list(&self, _req: MarketListRequest) -> Result<MarketListResponse, Status> {
        let markets = self
            .markets
            .values()
            .map(|market| market_list_response::MarketInfo {
                name: String::from(market.name),
                base: market.base.clone(),
                quote: market.quote.clone(),
                fee_precision: market.fee_prec,
                base_precision: market.base_prec,
                quote_precision: market.quote_prec,
                min_amount: market.min_amount.to_string(),
            })
            .collect();
        Ok(MarketListResponse { markets })
    }

    pub fn market_summary(&self, req: MarketSummaryRequest) -> Result<MarketSummaryResponse, Status> {
        let markets: Vec<String> = if req.markets.is_empty() {
            self.markets.keys().cloned().collect()
        } else {
            for market in &req.markets {
                if !self.markets.contains_key(market) {
                    return Err(Status::invalid_argument("invalid market"));
                }
            }
            req.markets
        };
        let market_summaries = markets
            .iter()
            .map(|market| {
                let status = self.markets.get(market).unwrap().status();
                market_summary_response::MarketSummary {
                    name: status.name,
                    ask_count: status.ask_count as i32,
                    ask_amount: status.ask_amount.to_string(),
                    bid_count: status.bid_count as i32,
                    bid_amount: status.bid_amount.to_string(),
                    trade_count: status.trade_count,
                }
            })
            .collect();
        Ok(MarketSummaryResponse { market_summaries })
    }

    fn check_service_available(&self) -> bool {
        if self.log_handler.is_block() {
            log::warn!("log_handler full");
            return false;
        }
        self.persistor.service_available()
    }

    pub fn update_balance(&mut self, real: bool, req: BalanceUpdateRequest) -> std::result::Result<BalanceUpdateResponse, Status> {
        if !self.check_service_available() {
            return Err(Status::unavailable(""));
        }
        if !self.balance_manager.asset_manager.asset_exist(&req.asset) {
            return Err(Status::invalid_argument("invalid asset"));
        }
        let prec = self.balance_manager.asset_manager.asset_prec_show(&req.asset);
        let change_result = Decimal::from_str(req.delta.as_str()).map_err(|_| Status::invalid_argument("invalid amount"))?;
        let change = change_result.round_dp(prec);
        let detail_json: serde_json::Value = if req.detail.is_empty() {
            json!({})
        } else {
            serde_json::from_str(req.detail.as_str()).map_err(|_| Status::invalid_argument("invalid detail"))?
        };
        self.update_controller
            .update_user_balance(
                &mut self.balance_manager,
                self.persistor.is_real(real).persist_for_balance(),
                req.user_id,
                req.asset.as_str(),
                req.business.clone(),
                req.business_id,
                change,
                detail_json,
            )
            .map_err(|e| Status::invalid_argument(format!("{}", e)))?;

        // TODO how to handle this error?
        // TODO operation_log after exec or before exec?
        if real {
            self.append_operation_log(OPERATION_BALANCE_UPDATE, &req);
        }
        Ok(BalanceUpdateResponse::default())
    }

    pub fn order_put(&mut self, real: bool, req: OrderPutRequest) -> Result<OrderInfo, Status> {
        if !self.check_service_available() {
            return Err(Status::unavailable(""));
        }
        if !self.markets.contains_key(&req.market) {
            return Err(Status::invalid_argument("invalid market"));
        }
        let market = self.markets.get_mut(&req.market).unwrap();
        let balance_manager = &mut self.balance_manager;
        let persistor = self.persistor.is_real(real).persist_for_market(market.tag());

        let order_input = order_input_from_proto(&req).map_err(|e| Status::invalid_argument(format!("invalid decimal {}", e)))?;

        let order = market
            .put_order(&mut self.sequencer, balance_manager.into(), persistor, order_input)
            .map_err(|e| Status::unknown(format!("{}", e)))?;
        if real {
            self.append_operation_log(OPERATION_ORDER_PUT, &req);
        }
        Ok(order_to_proto(&order))
    }

    pub fn order_cancel(&mut self, real: bool, req: OrderCancelRequest) -> Result<OrderInfo, tonic::Status> {
        if !self.check_service_available() {
            return Err(Status::unavailable(""));
        }
        let market = self
            .markets
            .get_mut(&req.market)
            .ok_or_else(|| Status::invalid_argument("invalid market"))?;
        let order = market
            .get(req.order_id)
            .ok_or_else(|| Status::invalid_argument("invalid order_id"))?;
        if order.user != req.user_id {
            return Err(Status::invalid_argument("invalid user"));
        }
        let balance_manager = &mut self.balance_manager;
        let persistor = self.persistor.is_real(real).persist_for_market(market.tag());

        market.cancel(balance_manager.into(), persistor, order.id);
        if real {
            self.append_operation_log(OPERATION_ORDER_CANCEL, &req);
        }
        Ok(order_to_proto(&order))
    }

    pub fn order_cancel_all(&mut self, real: bool, req: OrderCancelAllRequest) -> Result<OrderCancelAllResponse, tonic::Status> {
        if !self.check_service_available() {
            return Err(Status::unavailable(""));
        }
        let market = self
            .markets
            .get_mut(&req.market)
            .ok_or_else(|| Status::invalid_argument("invalid market"))?;
        let total = market.cancel_all_for_user(
            (&mut self.balance_manager).into(),
            self.persistor.is_real(real).persist_for_market(market.tag()),
            req.user_id,
        ) as u32;
        if real {
            self.append_operation_log(OPERATION_ORDER_CANCEL_ALL, &req);
        }
        Ok(OrderCancelAllResponse { total })
    }

    pub async fn debug_dump(&self, _req: DebugDumpRequest) -> Result<DebugDumpResponse, Status> {
        async {
            let mut connection = ConnectionType::connect(&self.settings.db_log).await?;
            crate::persist::dump_to_db(&mut connection, utils::current_timestamp() as i64, self).await
        }
        .await
        .map_err(|err| Status::unknown(format!("{}", err)))?;
        Ok(DebugDumpResponse {})
    }

    fn reset_state(&mut self) {
        self.sequencer.reset();
        for market in self.markets.values_mut() {
            market.reset();
        }
        //self.log_handler.reset();
        self.update_controller.reset();
        self.balance_manager.reset();
        //Ok(())
    }

    pub async fn market_reload(&mut self, from_scratch : bool) -> Result<(), Status> {

        if from_scratch {
            self.market_load_cfg.reset_load_time();
        }

        //assets and markets can be updated respectively, and must be handled one
        //after another
        let new_assets = self.market_load_cfg.load_asset_from_db(&self.dbg_pool)
            .await.map_err(|e| tonic::Status::internal(e.to_string()))?;
        
        self.balance_manager.asset_manager.append(&new_assets);

        let new_markets = self.market_load_cfg.load_market_from_db(&self.dbg_pool)
        .await.map_err(|e| tonic::Status::internal(e.to_string()))?;

        for entry in new_markets.into_iter() {
            let handle_ret = if self.markets.get(&entry.name).is_none() {
                market::Market::new(&entry, &self.balance_manager)
                    .and_then(|mk| {
                        self.markets.insert(entry.name, mk);
                        Ok(())
                    })
            }else {
                Err(anyhow!("market {} is duplicated", entry.name))
            };

            if let Err(e) = handle_ret {
                log::error!("On handle append market fail: {}", e);
            }
        }

        Ok(())
    }

    pub async fn debug_reset(&mut self, _req: DebugResetRequest) -> Result<DebugResetResponse, Status> {
        async {
            println!("do full reset: memory and db");
            self.reset_state();
            // waiting for pending db writes
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            /*
            notice: migration in sqlx is rather crude. It simply add operating records into
            _sqlx_migrations table and once an operating is recorded, it never try to reapply
            corresponding actions (even the table has been drop accidentily).

            and it is still not handle some edge case well: like create a the existed seq
            in postgresql cause an error from migrator

            that means you can not simply drop some table (because the migrations recorded
            in table _sqlx_migrations forbid it reroll,
            you can not even drop the all the talbes include _sqlx_migrations because some
            other object left in database will lead migrator fail ...

            now the way i found is drop and re-create the database ..., maybe a throughout
            dropping may also work?
            */
            /*
            let drop_cmd = format!("drop table if exists _sqlx_migrations, {}, {}, {}, {}, {}, {}, {}",
                tablenames::BALANCEHISTORY,
                tablenames::BALANCESLICE,
                tablenames::SLICEHISTORY,
                tablenames::OPERATIONLOG,
                tablenames::ORDERHISTORY,
                tablenames::USERTRADE,
                tablenames::ORDERSLICE);
            */
            // sqlx::query seems unable to handle multi statements, so `execute` is used here

            let db_str = self.settings.db_log.clone();
            let down_cmd = include_str!("../../migrations/reset/down.sql");
            let up_cmd = include_str!("../../migrations/reset/up.sql");
            let mut connection = ConnectionType::connect(&db_str).await?;
            connection.execute(down_cmd).await?;
            let mut connection = ConnectionType::connect(&db_str).await?;
            connection.execute(up_cmd).await?;

            //To workaround https://github.com/launchbadge/sqlx/issues/954: migrator is not Send
            let db_str = self.settings.db_log.clone();
            let thr_handle = std::thread::spawn(move || {
                let rt: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("build another runtime for migration");

                let ret = rt.block_on(async move {
                    let mut conn = ConnectionType::connect(&db_str).await?;
                    crate::persist::MIGRATOR.run(&mut conn).await?;
                    crate::message::persist::MIGRATOR.run(&mut conn).await
                });

                println!("migration task done");
                ret
            });

            tokio::task::spawn_blocking(move || thr_handle.join().unwrap()).await.unwrap()
        }
        .await
        .map_err(|err| Status::unknown(format!("{}", err)))?;
        Ok(DebugResetResponse {})
    }

    pub async fn debug_reload(&mut self, _req: DebugReloadRequest) -> Result<DebugReloadResponse, Status> {
        async {
            self.reset_state();
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let mut connection = ConnectionType::connect(&self.settings.db_log).await?;
            crate::persist::init_from_db(&mut connection, self).await
        }
        .await
        .map_err(|err| Status::unknown(format!("{}", err)))?;
        Ok(DebugReloadResponse {})
    }

    // reload 1000 in batch and replay
    pub fn replay(&mut self, method: &str, params: &str) -> SimpleResult {
        match method {
            OPERATION_BALANCE_UPDATE => {
                self.update_balance(false, serde_json::from_str(params)?)?;
            }
            OPERATION_ORDER_CANCEL => {
                self.order_cancel(false, serde_json::from_str(params)?)?;
            }
            OPERATION_ORDER_CANCEL_ALL => {
                self.order_cancel_all(false, serde_json::from_str(params)?)?;
            }
            OPERATION_ORDER_PUT => {
                self.order_put(false, serde_json::from_str(params)?)?;
            }
            _ => return Err(anyhow!("invalid operation {}", method)),
        }
        Ok(())
    }
    fn append_operation_log<Operation>(&mut self, method: &str, req: &Operation)
    where
        Operation: Serialize,
    {
        let params = serde_json::to_string(req).unwrap();
        let operation_log = models::OperationLog {
            id: self.sequencer.next_operation_log_id() as i64,
            time: FTimestamp(utils::current_timestamp()).into(),
            method: method.to_owned(),
            params,
        };
        self.log_handler.append(operation_log).ok();
    }
}

#[cfg(sqlxverf)]
fn sqlverf_clear_slice() {
    sqlx::query!("drop table if exists balance_history, balance_slice");
}

//use the ownership should make us has no dangling pointer
pub(crate) static mut G_STUB: Option<Controller> = None;
pub(crate) static mut G_RT: *const tokio::runtime::Runtime = std::ptr::null();
