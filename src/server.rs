use tonic::{self, Request, Response, Status};

//use rust_decimal::Decimal;

pub use crate::dto::*;

//use crate::me_history::HistoryWriter;

use crate::controller::G_STUB;

#[derive(Default)]
pub struct GrpcHandler {}

#[tonic::async_trait]
impl Matchengine for GrpcHandler {
    async fn asset_list(&self, request: Request<AssetListRequest>) -> Result<Response<AssetListResponse>, Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        Ok(Response::new(stub.asset_list(request.into_inner())?))
    }

    async fn balance_query(&self, request: Request<BalanceQueryRequest>) -> Result<Response<BalanceQueryResponse>, Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        Ok(Response::new(stub.balance_query(request.into_inner())?))
    }

    async fn order_query(&self, request: tonic::Request<OrderQueryRequest>) -> Result<tonic::Response<OrderQueryResponse>, tonic::Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        Ok(Response::new(stub.order_query(request.into_inner())?))
    }
    //async fn order_book(&self, _request: tonic::Request<OrderBookRequest>) -> Result<tonic::Response<OrderBookResponse>, tonic::Status> {
    //    unimplemented!()
    //}
    async fn order_book_depth(
        &self,
        request: tonic::Request<OrderBookDepthRequest>,
    ) -> Result<tonic::Response<OrderBookDepthResponse>, tonic::Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        Ok(Response::new(stub.order_book_depth(request.into_inner())?))
    }
    async fn order_detail(&self, request: tonic::Request<OrderDetailRequest>) -> Result<tonic::Response<OrderInfo>, tonic::Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        Ok(Response::new(stub.order_detail(request.into_inner())?))
    }
    async fn market_list(&self, request: tonic::Request<MarketListRequest>) -> Result<tonic::Response<MarketListResponse>, tonic::Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        Ok(Response::new(stub.market_list(request.into_inner())?))
    }
    async fn market_summary(
        &self,
        request: tonic::Request<MarketSummaryRequest>,
    ) -> Result<tonic::Response<MarketSummaryResponse>, tonic::Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        Ok(Response::new(stub.market_summary(request.into_inner())?))
    }

    async fn balance_update(&self, request: Request<BalanceUpdateRequest>) -> Result<Response<BalanceUpdateResponse>, Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        Ok(Response::new(stub.update_balance(true, request.into_inner())?))
    }

    async fn order_put(&self, request: Request<OrderPutRequest>) -> Result<Response<OrderInfo>, Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        Ok(Response::new(stub.order_put(true, request.into_inner())?))
    }

    async fn order_cancel(&self, request: tonic::Request<OrderCancelRequest>) -> Result<tonic::Response<OrderInfo>, tonic::Status> {
        let stub = unsafe { G_STUB.as_mut().unwrap() };
        Ok(Response::new(stub.order_cancel(true, request.into_inner())?))
    }
}
