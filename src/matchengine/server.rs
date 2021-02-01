use tonic::{self, Request, Response, Status};

//use rust_decimal::Decimal;
pub use crate::dto::*;

//use crate::me_history::HistoryWriter;
use crate::controller::G_RT;
use crate::controller::G_STUB;

pub struct GrpcHandler {}

macro_rules! get_stub {
    () => {
        unsafe { G_STUB.as_mut().unwrap() }
    };
}

fn run_blocking_the_world_task<F, G>(f: G) -> Result<(), Status>
where
    G: FnOnce() -> F + Send + 'static, //We need additional wrapping to send the using of controller into another thread
    F: std::future::Future<Output = Result<(), Status>> + 'static,
{
    println!("We start a handling with block-the-world (grpc) mode");
    //let handle = get_stub!().rt.clone();

    let thr_handle = std::thread::spawn(move || -> Result<(), Status> {
        unsafe {
            (*G_RT).block_on(f())
            //            just for verification
            //            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    });

    //simply block the thread in a crude way ...
    let ret = thr_handle.join();
    println!("Block-the-world task done, continue running");
    ret.unwrap()
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
    async fn order_cancel_all(
        &self,
        request: tonic::Request<OrderCancelAllRequest>,
    ) -> Result<tonic::Response<OrderCancelAllResponse>, tonic::Status> {
        let stub = get_stub!();
        Ok(Response::new(stub.order_cancel_all(true, request.into_inner())?))
    }

    // This is the only blocking call of the server
    #[cfg(debug_assertions)]
    async fn debug_dump(&self, request: Request<DebugDumpRequest>) -> Result<Response<DebugDumpResponse>, Status> {
        run_blocking_the_world_task(|| async {
            let stub = get_stub!();
            stub.debug_dump(request.into_inner()).await.map(|_| ())
        })
        .map(|_| Response::new(DebugDumpResponse {}))

        // match stub.stw_notifier.replace(None) {
        //     Some(chn) => {
        //         let f = Box::pin(stub.debug_dump(request.into_inner()));
        //         let fs: Box<dyn controller::DebugRunner<DebugDumpResponse>> = Box::new(f);
        //         chn.send(controller::DebugRunTask::Dump(fs))
        //             .map_err(|_| Status::unknown("Can not send the task out"))?;
        //         Ok(Response::new(DebugDumpResponse {}))
        //     }
        //     _ => Err(Status::unknown("No channel for Stop the world, may be occupied?")),
        // }
    }

    #[cfg(debug_assertions)]
    async fn debug_reset(&self, request: Request<DebugResetRequest>) -> Result<Response<DebugResetResponse>, Status> {
        run_blocking_the_world_task(|| async {
            let stub = get_stub!();
            stub.debug_reset(request.into_inner()).await.map(|_| ())
        })
        .map(|_| Response::new(DebugResetResponse {}))

        // match stub.stw_notifier.replace(None) {
        //     Some(chn) => {
        //         let f = Box::pin(stub.debug_reset(request.into_inner()));
        //         let fs: Box<dyn controller::DebugRunner<DebugResetResponse>> = Box::new(f);
        //         chn.send(controller::DebugRunTask::Reset(fs))
        //             .map_err(|_| Status::unknown("Can not send the task out"))?;
        //         Ok(Response::new(DebugResetResponse {}))
        //     }
        //     _ => Err(Status::unknown("No channel for Stop the world, may be occupied?")),
        // }
    }

    #[cfg(debug_assertions)]
    async fn debug_reload(&self, request: Request<DebugReloadRequest>) -> Result<Response<DebugReloadResponse>, Status> {
        run_blocking_the_world_task(|| async {
            let stub = get_stub!();
            stub.debug_reload(request.into_inner()).await.map(|_| ())
        })
        .map(|_| Response::new(DebugReloadResponse {}))
        // match stub.stw_notifier.replace(None) {
        //     Some(chn) => {
        //         let f = Box::pin(stub.debug_reload(request.into_inner()));
        //         let fs: Box<dyn controller::DebugRunner<DebugReloadResponse>> = Box::new(f);
        //         chn.send(controller::DebugRunTask::Reload(fs))
        //             .map_err(|_| Status::unknown("Can not send the task out"))?;
        //         Ok(Response::new(DebugReloadResponse {}))
        //     }
        //     _ => Err(Status::unknown("No channel for Stop the world, may be occupied?")),
        // }
    }

    #[cfg(not(debug_assertions))]
    async fn debug_dump(&self, request: Request<DebugDumpRequest>) -> Result<Response<DebugDumpResponse>, Status> {
        println!("Warning: Not avaliable in release build");
        Ok(Response::new(DebugDumpResponse {}))
    }

    #[cfg(not(debug_assertions))]
    async fn debug_reset(&self, request: Request<DebugResetRequest>) -> Result<Response<DebugResetResponse>, Status> {
        println!("Warning: Not avaliable in release build");
        Ok(Response::new(DebugResetResponse {}))
    }

    #[cfg(not(debug_assertions))]
    async fn debug_reload(&self, request: Request<DebugReloadRequest>) -> Result<Response<DebugReloadResponse>, Status> {
        println!("Warning: Not avaliable in release build");
        Ok(Response::new(DebugReloadResponse {}))
    }
}
