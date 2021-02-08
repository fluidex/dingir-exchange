#![allow(dead_code)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_and_return)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::single_char_pattern)]
#![allow(clippy::await_holding_refcell_ref)] // FIXME

use dingir_exchange::config;
use dingir_exchange::controller::Controller;
use dingir_exchange::persist;
use dingir_exchange::server::{GrpcHandler, MatchengineServer};
//use dingir_exchange::sqlxextend;

use dingir_exchange::types::ConnectionType;
use sqlx::Connection;

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let rt: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        let stub = prepare().await.expect("Init state error");
        grpc_run(stub).await
    })
    .unwrap();
}

async fn prepare() -> anyhow::Result<Controller> {
    let mut conf = config_rs::Config::new();
    let config_file = dotenv::var("CONFIG_FILE")?;
    conf.merge(config_rs::File::with_name(&config_file)).unwrap();
    let settings: config::Settings = conf.try_into().unwrap();
    println!("Settings: {:?}", settings);

    let mut conn = ConnectionType::connect(&settings.db_log).await?;
    persist::MIGRATOR.run(&mut conn).await?;
    let mut grpc_stub = Controller::new(settings);
    persist::init_from_db(&mut conn, &mut grpc_stub).await?;
    Ok(grpc_stub)
}

async fn grpc_run(stub: Controller) -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse().unwrap();
    let mut grpc = GrpcHandler::new(stub);
    log::info!("Starting gprc service");

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let on_leave = grpc.on_leave();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        log::info!("Ctrl-c received, shutting down");
        tx.send(()).ok();
    });

    tonic::transport::Server::builder()
        .add_service(MatchengineServer::new(grpc))
        .serve_with_shutdown(addr, async {
            rx.await.ok();
        })
        .await?;

    log::info!("Shutted down, wait for final clear");
    on_leave.leave().await;
    log::info!("Shutted down");
    Ok(())
}
