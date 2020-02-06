use crate::asset::{AssetManager, BalanceManager, BalanceType, BalanceUpdateController};
use crate::database::OperlogSender;
use crate::market;
use crate::{config, utils};
use rust_decimal::Decimal;
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;

use tonic::{self, Request, Response, Status};

//use rust_decimal::Decimal;
use crate::models;
use crate::types::SimpleResult;

pub mod matchengine {
    tonic::include_proto!("matchengine");
}

pub use matchengine::matchengine_server::*;
pub use matchengine::*;

use crate::message::KafkaMessageSender;
//use crate::me_history::HistoryWriter;
use crate::database::DatabaseWriterConfig;

use crate::history::DatabaseHistoryWriter;
use rust_decimal::prelude::Zero;
use std::collections::HashMap;

use std::str::FromStr;

pub struct GrpcStub {
    pub settings: config::Settings,
    pub sequencer: Rc<RefCell<market::Sequencer>>,
    pub balance_manager: Rc<RefCell<BalanceManager>>,
    pub asset_manager: AssetManager,
    pub update_controller: Rc<RefCell<BalanceUpdateController>>,
    pub markets: HashMap<String, market::Market>,
    pub log_handler: OperlogSender,
}

impl GrpcStub {
    pub fn new(settings: config::Settings) -> GrpcStub {
        let balance_manager = Rc::new(RefCell::new(BalanceManager::new(&settings.assets).unwrap()));
        let message_sender = Rc::new(RefCell::new(KafkaMessageSender::new(&settings.brokers).unwrap()));
        let history_writer = Rc::new(RefCell::new(
            DatabaseHistoryWriter::new(&DatabaseWriterConfig {
                database_url: settings.db_history.clone(),
                run_daemon: true,
            })
            .unwrap(),
        ));
        let update_controller = Rc::new(RefCell::new(BalanceUpdateController::new(
            balance_manager.clone(),
            message_sender.clone(),
            history_writer.clone(),
        )));
        let asset_manager = AssetManager::new(&settings.assets).unwrap();
        let sequencer = Rc::new(RefCell::new(market::Sequencer {
            order_id_start: 0,
            deals_id_start: 0,
            operlog_id_start: 0,
        }));
        let mut markets = HashMap::new();

        for entry in &settings.markets {
            let market = market::Market::new(
                entry,
                balance_manager.clone(),
                sequencer.clone(),
                history_writer.clone(),
                message_sender.clone(),
            )
            .unwrap();
            markets.insert(entry.name.clone(), market);
        }
        let log_handler = OperlogSender::new(&DatabaseWriterConfig {
            database_url: settings.db_log.clone(),
            run_daemon: true,
        })
        .unwrap();
        GrpcStub {
            settings,
            sequencer,
            asset_manager,
            balance_manager,
            update_controller,
            markets,
            log_handler,
        }
    }
    pub fn update_balance(&mut self, real: bool, req: &BalanceUpdateRequest) -> std::result::Result<BalanceUpdateResponse, Status> {
        if !self.asset_manager.asset_exist(&req.asset) {
            return Err(Status::invalid_argument("invalid asset"));
        }
        let prec = self.asset_manager.asset_prev_show(&req.asset);
        let change_result = Decimal::from_str(req.delta.as_str());
        if change_result.is_err() {
            return Err(Status::invalid_argument("invalid amount"));
        }
        let change = change_result.unwrap().round_dp(prec);
        let detail_json: serde_json::Value = if req.detail.is_empty() {
            json!({})
        } else {
            match serde_json::from_str(req.detail.as_str()) {
                Err(_) => return Err(Status::invalid_argument("invalid detail")),
                Ok(detail) => detail,
            }
        };
        let _is_valid = self.update_controller.borrow_mut().update_user_balance(
            real,
            req.user_id,
            req.asset.as_str(),
            req.business.clone(),
            req.business_id,
            change,
            detail_json,
        );
        let result = BalanceUpdateResponse::default();
        Ok(result)
    }
    // reload 1000 in batch and replay
    pub fn replay(&mut self, method: &str, params: serde_json::Value) -> SimpleResult {
        match method {
            "balance_update" => {
                let req: BalanceUpdateRequest = serde_json::from_value(params).unwrap();
                self.update_balance(false, &req)?;
                Ok(())
            }
            _ => unreachable!(),
        }
    }
    pub fn append_operlog(&mut self, method: &str, params: String) {
        let operlog = models::Operlog {
            id: self.sequencer.borrow_mut().next_operlog_id(),
            time: utils::current_native_date_time(),
            method: method.to_owned(),
            params,
        };
        self.log_handler.append(operlog)
    }
}
pub(crate) static mut G_STUB: Option<&mut GrpcStub> = None;

#[derive(Default)]
pub struct GrpcHandler {}

/*
struct MyStatus(tonic::Status);
impl std::convert::From<rust_decimal::Error> for MyStatus {
    fn from(error: rust_decimal::Error) -> Self {
        tonic::Status::invalid_argument(format!("invalid decimal {}", error))
    }
}
*/

#[tonic::async_trait]
impl Matchengine for GrpcHandler {
    async fn asset_list(&self, request: Request<AssetListRequest>) -> Result<Response<AssetListResponse>, Status> {
        println!("Got a request: {:?}", request);

        let stub = unsafe { G_STUB.as_mut().unwrap() };

        let reply = AssetListResponse {
            asset_lists: stub
                .settings
                .assets
                .iter()
                .map(|item| {
                    let mut entry: asset_list_response::AssetInfo = Default::default();

                    entry.name = item.name.clone();
                    entry.precision = item.prec_show;

                    entry
                })
                .collect(),
        };

        Ok(Response::new(reply))
    }

    async fn balance_query(&self, request: Request<BalanceQueryRequest>) -> Result<Response<BalanceQueryResponse>, Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        let req = request.into_inner();

        let query_assets = if req.assets.is_empty() {
            stub.settings.assets.iter().map(|asset| asset.name.clone()).collect()
        } else {
            req.assets
        };
        let mut result = BalanceQueryResponse::default();
        // TODO check invalid asset
        for asset_name in query_assets.iter() {
            let available = stub
                .balance_manager
                .borrow_mut()
                .get_with_round(req.user_id, BalanceType::AVAILABLE, &asset_name)
                .to_string();
            let freeze = stub
                .balance_manager
                .borrow_mut()
                .get_with_round(req.user_id, BalanceType::FREEZE, &asset_name)
                .to_string();
            result
                .balances
                .insert(asset_name.clone(), balance_query_response::AssetBalance { available, freeze });
        }
        Ok(Response::new(result))
    }

    async fn balance_update(&self, request: Request<BalanceUpdateRequest>) -> Result<Response<BalanceUpdateResponse>, Status> {
        println!("Got a request: {:?}", request);
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        let req = request.into_inner();
        let result = stub.update_balance(true, &req);
        match result {
            Ok(resp) => {
                //
                // TODO operlog after exec or before exec?
                stub.append_operlog("balance_update", serde_json::to_string(&req).unwrap());
                Ok(Response::new(resp))
            }
            Err(e) => Err(e),
        }
    }

    async fn asset_summary(
        &self,
        _request: tonic::Request<AssetSummaryRequest>,
    ) -> Result<tonic::Response<AssetSummaryResponse>, tonic::Status> {
        unimplemented!()
    }

    async fn order_query(&self, _request: tonic::Request<OrderQueryRequest>) -> Result<tonic::Response<OrderQueryResponse>, tonic::Status> {
        unimplemented!()
    }
    async fn order_cancel(&self, request: tonic::Request<OrderCancelRequest>) -> Result<tonic::Response<OrderInfo>, tonic::Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        let req = request.into_inner();
        if !stub.markets.contains_key(&req.market) {
            return Err(Status::invalid_argument("invalid market"));
        }
        let market = stub.markets.get_mut(&req.market).unwrap();
        let order = match market.get(req.order_id) {
            Some(o) => o,
            None => return Err(Status::invalid_argument("invalid order_id")),
        };
        if order.user != req.user_id {
            return Err(Status::invalid_argument("invalid user"));
        }
        market.cancel(true, order.id);
        Ok(Response::new(order_to_proto(&order)))
    }
    async fn order_book(&self, _request: tonic::Request<OrderBookRequest>) -> Result<tonic::Response<OrderBookResponse>, tonic::Status> {
        unimplemented!()
    }
    async fn order_book_depth(
        &self,
        request: tonic::Request<OrderBookDepthRequest>,
    ) -> Result<tonic::Response<OrderBookDepthResponse>, tonic::Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        let req = request.into_inner();

        if !stub.markets.contains_key(&req.market) {
            return Err(Status::invalid_argument("invalid market"));
        }
        /*
        if !req.interval.is_empty() {
            match
        } !["1", "0.1", "0.001", ""].any(|value| req.interval.as_str() == value) {
            return Err(Status::invalid_argument("invalid interval"));
        }
        let bucket_fn = |x: &Decimal| {
            if (req.interval == )
        }
        */
        let depth = stub.markets.get_mut(&req.market).unwrap().depth(req.limit as usize);
        let convert = |price_info: &Vec<market::PriceInfo>| {
            price_info
                .iter()
                .map(|price_info| order_book_depth_response::PriceInfo {
                    price: price_info.price.to_string(),
                    amount: price_info.amount.to_string(),
                })
                .collect::<Vec<_>>()
        };
        Ok(Response::new(OrderBookDepthResponse {
            asks: convert(&depth.asks),
            bids: convert(&depth.bids),
        }))
    }
    async fn order_detail(&self, request: tonic::Request<OrderDetailRequest>) -> Result<tonic::Response<OrderInfo>, tonic::Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        let req = request.into_inner();
        if !stub.markets.contains_key(&req.market) {
            return Err(Status::invalid_argument("invalid market"));
        }
        match stub.markets.get_mut(&req.market).unwrap().get(req.order_id) {
            Some(o) => Ok(Response::new(order_to_proto(&o))),
            None => Err(Status::invalid_argument("invalid order_id")),
        }
    }
    async fn market_list(&self, _request: tonic::Request<MarketListRequest>) -> Result<tonic::Response<MarketListResponse>, tonic::Status> {
        unimplemented!()
    }
    async fn market_summary(
        &self,
        request: tonic::Request<MarketSummaryRequest>,
    ) -> Result<tonic::Response<MarketSummaryResponse>, tonic::Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        let req = request.into_inner();

        let markets: Vec<String> = if req.markets.is_empty() {
            stub.markets.keys().cloned().collect()
        } else {
            for market in &req.markets {
                if !stub.markets.contains_key(market) {
                    return Err(Status::invalid_argument("invalid market"));
                }
            }
            req.markets
        };
        let summaries: Vec<_> = markets
            .iter()
            .map(|market| {
                let status = stub.markets.get_mut(market).unwrap().status();
                market_summary_response::MarketSummary {
                    name: status.name,
                    ask_count: status.ask_count as i32,
                    ask_amount: status.ask_amount.to_string(),
                    bid_count: status.bid_count as i32,
                    bid_amount: status.bid_amount.to_string(),
                }
            })
            .collect();
        Ok(Response::new(MarketSummaryResponse {
            market_summaries: summaries,
        }))
    }

    async fn order_put(&self, request: Request<OrderPutRequest>) -> Result<Response<OrderInfo>, Status> {
        println!("Got a request: {:?}", request);

        let stub = unsafe { G_STUB.as_mut().unwrap() };
        let req = request.into_inner();
        println!("Got a request: {:?}", req);
        if req.order_type == OrderType::Market as i32 {
            return Err(Status::unimplemented("market order"));
        }
        if !stub.markets.contains_key(&req.market) {
            return Err(Status::invalid_argument("invalid market"));
        }
        // TODO is there a better method
        let order_input_result = (|| -> Result<market::LimitOrderInput, rust_decimal::Error> {
            Ok(market::LimitOrderInput {
                user_id: req.user_id,
                side: if req.order_side == OrderSide::Ask as i32 {
                    market::OrderSide::ASK
                } else {
                    market::OrderSide::BID
                },
                amount: Decimal::from_str(req.amount.as_str())?,
                price: Decimal::from_str(req.price.as_str())?,
                // FIXME
                taker_fee: if req.taker_fee.is_empty() {
                    Decimal::zero()
                } else {
                    Decimal::from_str(req.taker_fee.as_str())?
                },
                maker_fee: if req.maker_fee.is_empty() {
                    Decimal::zero()
                } else {
                    Decimal::from_str(req.maker_fee.as_str())?
                },
                source: req.category,
                market: req.market,
            })
        })();
        let order_input = match order_input_result {
            Ok(o) => o,
            Err(e) => return Err(Status::invalid_argument(format!("invalid decimal {}", e))),
        };

        let order = stub
            .markets
            .get_mut(&order_input.market)
            .unwrap()
            .market_put_limit_order(true, &order_input);
        match order {
            Ok(o) => Ok(Response::new(order_to_proto(&o))),
            Err(e) => Err(Status::internal(format!("{:?}", e))),
        }
    }
}

pub fn order_to_proto(o: &market::Order) -> OrderInfo {
    OrderInfo {
        id: o.id,
        market: String::from(o.market),
        category: String::from(o.source),
        r#type: if o.type_0 == market::OrderType::LIMIT {
            OrderType::Limit as i32
        } else {
            OrderType::Market as i32
        },
        side: if o.side == market::OrderSide::ASK {
            OrderSide::Ask as i32
        } else {
            OrderSide::Bid as i32
        },
        user_id: o.user,
        create_time: o.create_time,
        update_time: o.update_time,
        price: o.price.to_string(),
        amount: o.amount.to_string(),
        taker_fee: o.taker_fee.to_string(),
        maker_fee: o.maker_fee.to_string(),
        left: o.left.to_string(),
        deal_stock: o.deal_stock.to_string(),
        deal_money: o.deal_money.to_string(),
        deal_fee: o.deal_fee.to_string(),
    }
}
