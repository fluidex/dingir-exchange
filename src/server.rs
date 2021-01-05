use tonic::{self, Request, Response, Status};

//use rust_decimal::Decimal;
pub use crate::dto::*;

//use crate::me_history::HistoryWriter;

use crate::controller;
use crate::controller::G_STUB;

pub struct GrpcHandler {}

macro_rules! get_stub {
    () => {
        unsafe { G_STUB.as_mut().unwrap() }
    };
}

#[tonic::async_trait]
impl Matchengine for GrpcHandler {
    async fn asset_list(&self, request: Request<AssetListRequest>) -> Result<Response<AssetListResponse>, Status> {
        let stub = get_stub!();
        Ok(Response::new(stub.asset_list(request.into_inner())?))
    }

    async fn balance_query(&self, request: Request<BalanceQueryRequest>) -> Result<Response<BalanceQueryResponse>, Status> {
        let stub = get_stub!();
        Ok(Response::new(stub.balance_query(request.into_inner())?))
    }

    async fn order_query(&self, request: tonic::Request<OrderQueryRequest>) -> Result<tonic::Response<OrderQueryResponse>, tonic::Status> {
        let stub = get_stub!();
        Ok(Response::new(stub.order_query(request.into_inner())?))
    }
    //async fn order_book(&self, _request: tonic::Request<OrderBookRequest>) -> Result<tonic::Response<OrderBookResponse>, tonic::Status> {
    //    unimplemented!()
    //}
    async fn order_book_depth(
        &self,
        request: tonic::Request<OrderBookDepthRequest>,
    ) -> Result<tonic::Response<OrderBookDepthResponse>, tonic::Status> {
        let stub = get_stub!();
        Ok(Response::new(stub.order_book_depth(request.into_inner())?))
    }
    async fn order_detail(&self, request: tonic::Request<OrderDetailRequest>) -> Result<tonic::Response<OrderInfo>, tonic::Status> {
        let stub = get_stub!();
        Ok(Response::new(stub.order_detail(request.into_inner())?))
    }
    async fn market_list(&self, request: tonic::Request<MarketListRequest>) -> Result<tonic::Response<MarketListResponse>, tonic::Status> {
        let stub = get_stub!();
        Ok(Response::new(stub.market_list(request.into_inner())?))
    }
    async fn market_summary(
        &self,
        request: tonic::Request<MarketSummaryRequest>,
    ) -> Result<tonic::Response<MarketSummaryResponse>, tonic::Status> {
        let stub = get_stub!();
        Ok(Response::new(stub.market_summary(request.into_inner())?))
    }

    async fn balance_update(&self, request: Request<BalanceUpdateRequest>) -> Result<Response<BalanceUpdateResponse>, Status> {
        let stub = get_stub!();
        Ok(Response::new(stub.update_balance(true, request.into_inner())?))
    }

    async fn order_put(&self, request: Request<OrderPutRequest>) -> Result<Response<OrderInfo>, Status> {
        let stub = get_stub!();
        Ok(Response::new(stub.order_put(true, request.into_inner())?))
    }

    async fn order_cancel(&self, request: tonic::Request<OrderCancelRequest>) -> Result<tonic::Response<OrderInfo>, tonic::Status> {
        let stub = get_stub!();
        Ok(Response::new(stub.order_cancel(true, request.into_inner())?))
    }
    // This is the only blocking call of the server
    #[cfg(debug_assertions)]
    async fn debug_dump(&self, request: Request<DebugDumpRequest>) -> Result<Response<DebugDumpResponse>, Status> {
        let stub = get_stub!();
        match stub.stw_notifier.replace(None) {
            Some(chn) => {
                let f = Box::pin(stub.debug_dump(request.into_inner()));
                let fs: Box<dyn controller::DebugRunner<DebugDumpResponse>> = Box::new(f);
                chn.send(controller::DebugRunTask::Dump(fs))
                    .map_err(|_| Status::unknown("Can not send the task out"))?;
                Ok(Response::new(DebugDumpResponse {}))
            }
            _ => Err(Status::unknown("No channel for Stop the world, may be occupied?")),
        }
    }

    #[cfg(debug_assertions)]
    async fn debug_reset(&self, request: Request<DebugResetRequest>) -> Result<Response<DebugResetResponse>, Status> {
        let stub = get_stub!();
        match stub.stw_notifier.replace(None) {
            Some(chn) => {
                let f = Box::pin(stub.debug_reset(request.into_inner()));
                let fs: Box<dyn controller::DebugRunner<DebugResetResponse>> = Box::new(f);
                chn.send(controller::DebugRunTask::Reset(fs))
                    .map_err(|_| Status::unknown("Can not send the task out"))?;
                Ok(Response::new(DebugResetResponse {}))
            }
            _ => Err(Status::unknown("No channel for Stop the world, may be occupied?")),
        }
    }

    #[cfg(debug_assertions)]
    async fn debug_reload(&self, request: Request<DebugReloadRequest>) -> Result<Response<DebugReloadResponse>, Status> {
        let stub = get_stub!();
        match stub.stw_notifier.replace(None) {
            Some(chn) => {
                let f = Box::pin(stub.debug_reload(request.into_inner()));
                let fs: Box<dyn controller::DebugRunner<DebugReloadResponse>> = Box::new(f);
                chn.send(controller::DebugRunTask::Reload(fs))
                    .map_err(|_| Status::unknown("Can not send the task out"))?;
                Ok(Response::new(DebugReloadResponse {}))
            }
            _ => Err(Status::unknown("No channel for Stop the world, may be occupied?")),
        }
    }

    #[cfg(not(debug_assertions))]
    async fn debug_dump(&self, request: Request<DebugDumpRequest>) -> Result<Response<DebugDumpResponse>, Status> {
        Err(Status::unknown("Not avaliable in release build"))
    }

    #[cfg(not(debug_assertions))]
    async fn debug_reset(&self, request: Request<DebugResetRequest>) -> Result<Response<DebugResetResponse>, Status> {
        Err(Status::unknown("Not avaliable in release build"))
    }

    #[cfg(not(debug_assertions))]
    async fn debug_reload(&self, request: Request<DebugReloadRequest>) -> Result<Response<DebugReloadResponse>, Status> {
        Err(Status::unknown("Not avaliable in release build"))
    }
}
