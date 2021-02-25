use tonic::{self, Request, Response, Status};

//use rust_decimal::Decimal;
pub use crate::dto::*;

//use crate::me_history::HistoryWriter;
use crate::controller::Controller;
use std::fmt::Debug;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};

type StubType = Arc<RwLock<Controller>>;

type ControllerAction = Box<dyn FnOnce(StubType) -> Pin<Box<dyn futures::Future<Output = ()> + Send>> + Send>;

pub struct GrpcHandler {
    stub: StubType,
    task_dispacther: mpsc::Sender<ControllerAction>,
    set_close: Option<oneshot::Sender<()>>,
}

struct ControllerDispatch<OT>(ControllerAction, oneshot::Receiver<OT>);

impl<OT: 'static + Debug + Send> ControllerDispatch<OT> {
    fn new<T>(f: T) -> Self
    where
        T: for<'c> FnOnce(&'c mut Controller) -> Pin<Box<dyn futures::Future<Output = OT> + Send + 'c>>,
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();

        ControllerDispatch(
            Box::new(
                move |ctrl: StubType| -> Pin<Box<dyn futures::Future<Output = ()> + Send + 'static>> {
                    Box::pin(async move {
                        let mut wg = ctrl.write().await;
                        if let Err(t) = tx.send(f(&mut wg).await) {
                            log::error!("Controller action can not be return: {:?}", t);
                        }
                    })
                },
            ),
            rx,
        )
    }
}

fn map_dispatch_err<T: 'static>(_: mpsc::error::SendError<T>) -> tonic::Status {
    tonic::Status::unknown("Server temporary unavaliable")
}

type ControllerRet<OT> = Result<OT, tonic::Status>;
type ServerRet<OT> = Result<Response<OT>, tonic::Status>;

fn map_dispatch_ret<OT: 'static>(recv_ret: Result<ControllerRet<OT>, oneshot::error::RecvError>) -> ServerRet<OT> {
    match recv_ret {
        Ok(ret) => ret.map(Response::new),
        Err(_) => Err(Status::unknown("Dispatch ret unreach")),
    }
}

pub struct ServerLeave(mpsc::Sender<ControllerAction>, oneshot::Sender<()>);

impl ServerLeave {
    pub async fn leave(self) {
        self.1.send(()).unwrap();
        self.0.closed().await;
    }
}

impl GrpcHandler {
    pub fn new(stub: Controller) -> Self {
        let mut persist_interval = tokio::time::interval(std::time::Duration::from_secs(stub.settings.persist_interval as u64));

        let stub = Arc::new(RwLock::new(stub));
        //we always wait so the size of channel is no matter
        let (tx, mut rx) = mpsc::channel(16);
        let (tx_close, mut rx_close) = oneshot::channel();

        let stub_for_dispatch = stub.clone();

        let ret = GrpcHandler {
            task_dispacther: tx,
            set_close: Some(tx_close),
            stub,
        };

        tokio::spawn(async move {
            persist_interval.tick().await; //skip first tick
            loop {
                tokio::select! {
                    may_task = rx.recv() => {
                        let task = may_task.expect("Server scheduler has unexpected exit");
                        task(stub_for_dispatch.clone()).await;
                    }
                    _ = persist_interval.tick() => {
                        let stub_rd = stub_for_dispatch.read().await;
                        log::info!("Start a persisting task");
                        unsafe {
                            crate::persist::fork_and_make_slice(&*stub_rd);
                        }
                    }
                    _ = &mut rx_close => {
                        log::info!("Server scheduler is notified to close");
                        rx.close();
                        break;
                    }
                }
            }

            //drain unhandled task
            while let Some(task) = rx.recv().await {
                task(stub_for_dispatch.clone()).await;
            }

            log::warn!("Server scheduler has exited");
        });

        ret
    }

    pub fn on_leave(&mut self) -> ServerLeave {
        ServerLeave(
            self.task_dispacther.clone(),
            self.set_close.take().expect("Do not call twice with on_leave"),
        )
    }
}

#[tonic::async_trait]
impl Matchengine for GrpcHandler {
    async fn asset_list(&self, request: Request<AssetListRequest>) -> Result<Response<AssetListResponse>, Status> {
        let stub = self.stub.read().await;
        Ok(Response::new(stub.asset_list(request.into_inner())?))
    }

    async fn balance_query(&self, request: Request<BalanceQueryRequest>) -> Result<Response<BalanceQueryResponse>, Status> {
        let stub = self.stub.read().await;
        Ok(Response::new(stub.balance_query(request.into_inner())?))
    }

    async fn order_query(&self, request: tonic::Request<OrderQueryRequest>) -> Result<tonic::Response<OrderQueryResponse>, tonic::Status> {
        let stub = self.stub.read().await;
        Ok(Response::new(stub.order_query(request.into_inner())?))
    }
    //async fn order_book(&self, _request: tonic::Request<OrderBookRequest>) -> Result<tonic::Response<OrderBookResponse>, tonic::Status> {
    //    unimplemented!()
    //}
    async fn order_book_depth(
        &self,
        request: tonic::Request<OrderBookDepthRequest>,
    ) -> Result<tonic::Response<OrderBookDepthResponse>, tonic::Status> {
        let stub = self.stub.read().await;
        Ok(Response::new(stub.order_book_depth(request.into_inner())?))
    }
    async fn order_detail(&self, request: tonic::Request<OrderDetailRequest>) -> Result<tonic::Response<OrderInfo>, tonic::Status> {
        let stub = self.stub.read().await;
        Ok(Response::new(stub.order_detail(request.into_inner())?))
    }
    async fn market_list(&self, request: tonic::Request<MarketListRequest>) -> Result<tonic::Response<MarketListResponse>, tonic::Status> {
        let stub = self.stub.read().await;
        Ok(Response::new(stub.market_list(request.into_inner())?))
    }
    async fn market_summary(
        &self,
        request: tonic::Request<MarketSummaryRequest>,
    ) -> Result<tonic::Response<MarketSummaryResponse>, tonic::Status> {
        let stub = self.stub.read().await;
        Ok(Response::new(stub.market_summary(request.into_inner())?))
    }

    /*---------------------------- following are "written ops" ---------------------------------*/
    async fn balance_update(&self, request: Request<BalanceUpdateRequest>) -> Result<Response<BalanceUpdateResponse>, Status> {
        let ControllerDispatch(act, rt) =
            ControllerDispatch::new(move |ctrl: &mut Controller| Box::pin(async move { ctrl.update_balance(true, request.into_inner()) }));

        self.task_dispacther.send(act).await.map_err(map_dispatch_err)?;
        map_dispatch_ret(rt.await)
    }

    async fn order_put(&self, request: Request<OrderPutRequest>) -> Result<Response<OrderInfo>, Status> {
        let ControllerDispatch(act, rt) =
            ControllerDispatch::new(move |ctrl: &mut Controller| Box::pin(async move { ctrl.order_put(true, request.into_inner()) }));

        self.task_dispacther.send(act).await.map_err(map_dispatch_err)?;
        map_dispatch_ret(rt.await)
    }

    async fn order_cancel(&self, request: tonic::Request<OrderCancelRequest>) -> Result<tonic::Response<OrderInfo>, tonic::Status> {
        let ControllerDispatch(act, rt) =
            ControllerDispatch::new(move |ctrl: &mut Controller| Box::pin(async move { ctrl.order_cancel(true, request.into_inner()) }));

        self.task_dispacther.send(act).await.map_err(map_dispatch_err)?;
        map_dispatch_ret(rt.await)
    }
    async fn order_cancel_all(
        &self,
        request: tonic::Request<OrderCancelAllRequest>,
    ) -> Result<tonic::Response<OrderCancelAllResponse>, tonic::Status> {
        let ControllerDispatch(act, rt) = ControllerDispatch::new(move |ctrl: &mut Controller| {
            Box::pin(async move { ctrl.order_cancel_all(true, request.into_inner()) })
        });

        self.task_dispacther.send(act).await.map_err(map_dispatch_err)?;
        map_dispatch_ret(rt.await)
    }

    async fn reload_markets(&self, request: Request<ReloadMarketsRequest>) -> Result<Response<SimpleSuccessResponse>, Status> {

        //there should be no need to queue the opeartion
        let mut stub = self.stub.write().await;      

        stub.market_reload(request.into_inner().from_scratch).await?;

        Ok(Response::new(SimpleSuccessResponse {}))
    }

    // This is the only blocking call of the server
    #[cfg(debug_assertions)]
    async fn debug_dump(&self, request: Request<DebugDumpRequest>) -> Result<Response<DebugDumpResponse>, Status> {
        let ControllerDispatch(act, rt) =
            ControllerDispatch::new(move |ctrl: &mut Controller| Box::pin(ctrl.debug_dump(request.into_inner())));

        self.task_dispacther.send(act).await.map_err(map_dispatch_err)?;
        map_dispatch_ret(rt.await)
    }

    #[cfg(debug_assertions)]
    async fn debug_reset(&self, request: Request<DebugResetRequest>) -> Result<Response<DebugResetResponse>, Status> {
        let ControllerDispatch(act, rt) =
            ControllerDispatch::new(move |ctrl: &mut Controller| Box::pin(ctrl.debug_reset(request.into_inner())));

        self.task_dispacther.send(act).await.map_err(map_dispatch_err)?;
        map_dispatch_ret(rt.await)
    }

    #[cfg(debug_assertions)]
    async fn debug_reload(&self, request: Request<DebugReloadRequest>) -> Result<Response<DebugReloadResponse>, Status> {
        let ControllerDispatch(act, rt) =
            ControllerDispatch::new(move |ctrl: &mut Controller| Box::pin(ctrl.debug_reload(request.into_inner())));

        self.task_dispacther.send(act).await.map_err(map_dispatch_err)?;
        map_dispatch_ret(rt.await)
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
