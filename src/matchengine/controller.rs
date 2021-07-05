use super::rpc::*;
use crate::asset::{BalanceManager, BalanceType, BalanceUpdateController};
use crate::config::{self};
use crate::database::{DatabaseWriterConfig, OperationLogSender};
use crate::history::DatabaseHistoryWriter;
use crate::market::{self, OrderInput};
use crate::message::{FullOrderMessageManager, SimpleMessageManager};
use crate::models::{self};
use crate::persist::{CompositePersistor, DBBasedPersistor, DummyPersistor, FileBasedPersistor, MessengerBasedPersistor, PersistExector};
use crate::sequencer::Sequencer;
use crate::storage::config::MarketConfigs;
use crate::types::{ConnectionType, DbType, SimpleResult};
use crate::user_manager::{self, UserManager};
use crate::utils::{self, FTimestamp};

use anyhow::{anyhow, bail};
use rust_decimal::prelude::Zero;
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::json;
use sqlx::Connection;
use sqlx::Executor;
use tonic::{self, Status};

use std::collections::HashMap;
use std::convert::TryFrom;
use std::str::FromStr;

pub trait OperationLogConsumer {
    fn is_block(&self) -> bool;
    fn append_operation_log(&mut self, item: models::OperationLog) -> anyhow::Result<(), models::OperationLog>;
}

impl OperationLogConsumer for OperationLogSender {
    fn is_block(&self) -> bool {
        self.is_block()
    }
    fn append_operation_log(&mut self, item: models::OperationLog) -> anyhow::Result<(), models::OperationLog> {
        self.append(item)
    }
}

// TODO: reuse pool of two dbs when they are same?
fn create_persistor(settings: &config::Settings) -> Box<dyn PersistExector> {
    let persist_to_mq = true;
    let persist_to_mq_full_order = true;
    let persist_to_db = false;
    let persist_to_file = false;
    let mut persistor = Box::new(CompositePersistor::default());
    if !settings.brokers.is_empty() && persist_to_mq {
        persistor.add_persistor(Box::new(MessengerBasedPersistor::new(Box::new(
            SimpleMessageManager::new_and_run(&settings.brokers).unwrap(),
        ))));
    }
    if !settings.brokers.is_empty() && persist_to_mq_full_order {
        persistor.add_persistor(Box::new(MessengerBasedPersistor::new(Box::new(
            FullOrderMessageManager::new_and_run(&settings.brokers).unwrap(),
        ))));
    }
    if persist_to_db {
        // persisting to db is disabled now
        let pool = sqlx::Pool::<DbType>::connect_lazy(&settings.db_history).unwrap();
        persistor.add_persistor(Box::new(DBBasedPersistor::new(Box::new(
            DatabaseHistoryWriter::new(
                &DatabaseWriterConfig {
                    spawn_limit: 4,
                    apply_benchmark: true,
                    capability_limit: 8192,
                },
                &pool,
            )
            .unwrap(),
        ))));
    }
    if settings.brokers.is_empty() || persist_to_file {
        persistor.add_persistor(Box::new(FileBasedPersistor::new("persistor_output.txt")));
    }
    persistor
}

// match engine is single-threaded. So `Controller` is used as the only entrance
// for get and set the global state
pub struct Controller {
    //<LogHandlerType> where LogHandlerType: OperationLogConsumer + Send {
    pub settings: config::Settings,
    pub sequencer: Sequencer,
    pub user_manager: UserManager,
    pub balance_manager: BalanceManager,
    //    pub asset_manager: AssetManager,
    pub update_controller: BalanceUpdateController,
    pub markets: HashMap<String, market::Market>,
    // TODO: is it worth to use generics rather than dynamic pointer?
    pub log_handler: Box<dyn OperationLogConsumer + Send + Sync>,
    pub persistor: Box<dyn PersistExector>,
    // TODO: is this needed?
    pub dummy_persistor: Box<dyn PersistExector>,
    dbg_pool: sqlx::Pool<DbType>,
    market_load_cfg: MarketConfigs,
}

const ORDER_LIST_MAX_LEN: usize = 100;
const OPERATION_REGISTER_USER: &str = "register_user";
const OPERATION_BALANCE_UPDATE: &str = "balance_update";
const OPERATION_ORDER_CANCEL: &str = "order_cancel";
const OPERATION_ORDER_CANCEL_ALL: &str = "order_cancel_all";
const OPERATION_ORDER_PUT: &str = "order_put";
const OPERATION_TRANSFER: &str = "transfer";

pub fn create_controller(cfgs: (config::Settings, MarketConfigs)) -> Controller {
    let settings = cfgs.0;
    let main_pool = sqlx::Pool::<DbType>::connect_lazy(&settings.db_log).unwrap();
    let user_manager = UserManager::new(); // load from db later
    let balance_manager = BalanceManager::new(&settings.assets).unwrap();

    let update_controller = BalanceUpdateController::new();
    //        let asset_manager = AssetManager::new(&settings.assets).unwrap();
    let sequencer = Sequencer::default();
    let mut markets = HashMap::new();
    for entry in &settings.markets {
        let market = market::Market::new(entry, &settings, &balance_manager).unwrap();
        markets.insert(entry.name.clone(), market);
    }

    let persistor = create_persistor(&settings);
    let log_handler = OperationLogSender::new(&DatabaseWriterConfig {
        spawn_limit: 4,
        apply_benchmark: true,
        capability_limit: 8192,
    })
    .start_schedule(&main_pool)
    .unwrap();
    Controller {
        settings,
        sequencer,
        //            asset_manager,
        user_manager,
        balance_manager,
        update_controller,
        markets,
        log_handler: Box::<OperationLogSender>::new(log_handler),
        persistor,
        dummy_persistor: DummyPersistor::new_box(),
        dbg_pool: main_pool,
        market_load_cfg: cfgs.1,
    }
}

impl Controller {
    //fn get_persistor(&mut self, real: bool) -> &mut Box<dyn PersistExector> {
    //if real {&mut self.persistor} else { &mut self.dummy_persistor }
    //}
    //fn get_persistor(&mut self, real: bool) -> Box<dyn PersistExector> {
    //    if real {self.persistor} else { self.dummy_persistor }
    //}
    pub fn asset_list(&self, _req: AssetListRequest) -> Result<AssetListResponse, Status> {
        let result = AssetListResponse {
            asset_lists: self
                .settings
                .assets
                .iter()
                .map(|item| asset_list_response::AssetInfo {
                    symbol: item.symbol.clone(),
                    name: item.name.clone(),
                    chain_id: item.chain_id as i32,
                    token_address: item.token_address.clone(),
                    precision: item.prec_show,
                    logo_uri: item.logo_uri.clone(),
                    inner_id: item.rollup_token_id,
                })
                .collect(),
        };
        Ok(result)
    }
    pub fn balance_query(&self, req: BalanceQueryRequest) -> Result<BalanceQueryResponse, Status> {
        let all_asset_param_valid = req
            .assets
            .iter()
            .all(|asset_param| self.settings.assets.iter().any(|asset| asset.id.eq(asset_param)));
        if !all_asset_param_valid {
            return Err(Status::invalid_argument("invalid asset"));
        }
        let query_assets = if req.assets.is_empty() {
            self.settings.assets.iter().map(|asset| asset.id.clone()).collect()
        } else {
            req.assets
        };
        let user_id = req.user_id;
        let balance_manager = &self.balance_manager;
        let balances = query_assets
            .into_iter()
            .map(|asset_id| {
                let available = balance_manager
                    .get_with_round(user_id, BalanceType::AVAILABLE, &asset_id)
                    .to_string();
                let frozen = balance_manager.get_with_round(user_id, BalanceType::FREEZE, &asset_id).to_string();
                balance_query_response::AssetBalance {
                    asset_id,
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
                    .map(|order_rc| OrderInfo::from(order_rc.deep()))
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
        Ok(OrderInfo::from(order))
    }

    pub fn market_list(&self, _req: MarketListRequest) -> Result<MarketListResponse, Status> {
        let markets = self
            .markets
            .values()
            .map(|market| market_list_response::MarketInfo {
                name: String::from(market.name),
                base: market.base.into(),
                quote: market.quote.into(),
                fee_precision: market.fee_prec,
                amount_precision: market.amount_prec,
                price_precision: market.price_prec,
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

    pub fn register_user(&mut self, real: bool, mut req: UserInfo) -> std::result::Result<UserInfo, Status> {
        if !self.check_service_available() {
            return Err(Status::unavailable(""));
        }

        let last_user_id = self.user_manager.users.len() as u32;
        req.user_id = last_user_id + 1;
        // TODO: check user_id
        // if last_user_id + 1 != req.user_id {
        //     return Err(Status::invalid_argument("inconsist user_id"));
        // }

        self.user_manager.users.insert(
            req.user_id,
            user_manager::UserInfo {
                l1_address: req.l1_address.clone(),
                l2_pubkey: req.l2_pubkey.clone(),
            },
        );

        if real {
            let mut detail: serde_json::Value = json!({});
            detail["id"] = serde_json::Value::from(req.user_id);
            self.persistor.register_user(models::AccountDesc {
                id: req.user_id as i32,
                l1_address: req.l1_address.clone(),
                l2_pubkey: req.l2_pubkey.clone(),
            });
        }

        if real {
            self.append_operation_log(OPERATION_REGISTER_USER, &req);
        }
        Ok(UserInfo {
            user_id: req.user_id,
            l1_address: req.l1_address,
            l2_pubkey: req.l2_pubkey,
        })
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
        //let persistor = self.get_persistor(real);
        let persistor = if real { &mut self.persistor } else { &mut self.dummy_persistor };
        self.update_controller
            .update_user_balance(
                &mut self.balance_manager,
                persistor,
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
        let total_order_num: usize = self
            .markets
            .iter()
            .map(|(_, market)| market.get_order_num_of_user(req.user_id))
            .sum();
        debug_assert!(total_order_num <= self.settings.user_order_num_limit);
        if total_order_num == self.settings.user_order_num_limit {
            return Err(Status::unavailable("too many active orders for user"));
        }
        let market = self.markets.get_mut(&req.market).unwrap();
        let balance_manager = &mut self.balance_manager;
        //let persistor = self.get_persistor(real);
        let persistor = if real { &mut self.persistor } else { &mut self.dummy_persistor };
        let order_input = OrderInput::try_from(req.clone()).map_err(|e| Status::invalid_argument(format!("invalid decimal {}", e)))?;
        let order = market
            .put_order(&mut self.sequencer, balance_manager.into(), persistor, order_input)
            .map_err(|e| Status::unknown(format!("{}", e)))?;
        if real {
            self.append_operation_log(OPERATION_ORDER_PUT, &req);
        }
        Ok(OrderInfo::from(order))
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
        //let persistor = self.get_persistor(real);
        let persistor = if real { &mut self.persistor } else { &mut self.dummy_persistor };
        market.cancel(balance_manager.into(), persistor, order.id);
        if real {
            self.append_operation_log(OPERATION_ORDER_CANCEL, &req);
        }
        Ok(OrderInfo::from(order))
    }

    pub fn order_cancel_all(&mut self, real: bool, req: OrderCancelAllRequest) -> Result<OrderCancelAllResponse, tonic::Status> {
        if !self.check_service_available() {
            return Err(Status::unavailable(""));
        }
        let market = self
            .markets
            .get_mut(&req.market)
            .ok_or_else(|| Status::invalid_argument("invalid market"))?;
        //let persistor = self.get_persistor(real);
        let persistor = if real { &mut self.persistor } else { &mut self.dummy_persistor };
        let total = market.cancel_all_for_user((&mut self.balance_manager).into(), persistor, req.user_id) as u32;
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
        self.user_manager.reset();
        //Ok(())
    }

    pub async fn market_reload(&mut self, from_scratch: bool) -> Result<(), Status> {
        if from_scratch {
            self.market_load_cfg.reset_load_time();
        }

        //assets and markets can be updated respectively, and must be handled one
        //after another
        let new_assets = self
            .market_load_cfg
            .load_asset_from_db(&self.dbg_pool)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

        self.balance_manager.asset_manager.append(&new_assets);

        let new_markets = self
            .market_load_cfg
            .load_market_from_db(&self.dbg_pool)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

        for entry in new_markets.into_iter() {
            let handle_ret = if self.markets.get(&entry.name).is_none() {
                market::Market::new(&entry, &self.settings, &self.balance_manager).map(|mk| {
                    self.markets.insert(entry.name, mk);
                })
            } else {
                Err(anyhow!("market {} is duplicated", entry.name))
            };

            if let Err(e) = handle_ret {
                log::error!("On handle append market fail: {}", e);
            }
        }

        Ok(())
    }

    pub fn transfer(&mut self, real: bool, req: TransferRequest) -> Result<TransferResponse, Status> {
        if !self.check_service_available() {
            return Err(Status::unavailable(""));
        }

        let asset_id = &req.asset;
        if !self.balance_manager.asset_manager.asset_exist(asset_id) {
            return Err(Status::invalid_argument("invalid asset"));
        }

        let from_user_id = req.from;
        let to_user_id = req.to;
        if !self.user_manager.users.contains_key(&to_user_id) {
            return Err(Status::invalid_argument("invalid to_user"));
        }

        let balance_manager = &self.balance_manager;
        let balance_from = balance_manager.get(from_user_id, BalanceType::AVAILABLE, asset_id);

        let zero = Decimal::from(0);
        let delta = Decimal::from_str(&req.delta).unwrap_or(zero);

        if delta <= zero || delta > balance_from {
            return Ok(TransferResponse {
                success: false,
                asset: asset_id.to_owned(),
                balance_from: balance_from.to_string(),
            });
        }

        let prec = self.balance_manager.asset_manager.asset_prec_show(asset_id);
        let change = delta.round_dp(prec);

        let business = "transfer";
        let timestamp = FTimestamp(utils::current_timestamp());
        let business_id = (timestamp.0 * 1_000_f64) as u64; // milli-seconds
        let detail_json: serde_json::Value = if req.memo.is_empty() {
            json!({})
        } else {
            serde_json::from_str(req.memo.as_str()).map_err(|_| Status::invalid_argument("invalid memo"))?
        };

        //let persistor = self.get_persistor(real);
        let persistor = if real { &mut self.persistor } else { &mut self.dummy_persistor };
        self.update_controller
            .update_user_balance(
                &mut self.balance_manager,
                persistor,
                from_user_id,
                asset_id,
                business.to_owned(),
                business_id,
                -change,
                detail_json.clone(),
            )
            .map_err(|e| Status::invalid_argument(format!("{}", e)))?;

        let persistor = if real { &mut self.persistor } else { &mut self.dummy_persistor };
        self.update_controller
            .update_user_balance(
                &mut self.balance_manager,
                persistor,
                to_user_id,
                asset_id,
                business.to_owned(),
                business_id,
                change,
                detail_json,
            )
            .map_err(|e| Status::invalid_argument(format!("{}", e)))?;

        if real {
            self.persistor.put_transfer(models::InternalTx {
                time: timestamp.into(),
                user_from: from_user_id as i32, // TODO: will this overflow?
                user_to: to_user_id as i32,     // TODO: will this overflow?
                asset: asset_id.to_string(),
                amount: change,
            });

            self.append_operation_log(OPERATION_TRANSFER, &req);
        }

        Ok(TransferResponse {
            success: true,
            asset: asset_id.to_owned(),
            balance_from: (balance_from - change).to_string(),
        })
    }

    pub async fn debug_reset(&mut self, _req: DebugResetRequest) -> Result<DebugResetResponse, Status> {
        async {
            log::info!("do full reset: memory and db");
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

                log::info!("migration task done");
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
            OPERATION_TRANSFER => {
                self.transfer(false, serde_json::from_str(params)?)?;
            }
            OPERATION_REGISTER_USER => {
                self.register_user(false, serde_json::from_str(params)?)?;
            }
            _ => bail!("invalid operation {}", method),
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
        (*self.log_handler).append_operation_log(operation_log).ok();
    }
}

#[cfg(sqlxverf)]
fn sqlverf_clear_slice() -> impl std::any::Any {
    sqlx::query!("drop table if exists balance_history, balance_slice")
}
