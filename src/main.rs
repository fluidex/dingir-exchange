#![allow(dead_code)]
//#![allow(unused_imports)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_and_return)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::await_holding_refcell_ref)] // FIXME

mod types;

mod asset;
mod config;
mod controller;
mod database;
mod dto;
mod history;
mod kline;
mod market;
mod message;
mod models;
mod persist;
mod sqlxextend;
//mod schema;
mod sequencer;
mod server;
mod utils;

// TODO: refactor create & workspace & folders
mod restapi {
    mod errors;
    mod mock;
    mod tradingview;
    mod types;
}

use controller::Controller;
use server::{GrpcHandler, MatchengineServer};

use sqlx::Connection;
use types::ConnectionType;

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    //simple_logger::init().unwrap();
    let mut rt: tokio::runtime::Runtime = tokio::runtime::Builder::new()
        .enable_all()
        .threaded_scheduler()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        let stub = prepare().await.expect("Init state error");
        stub.prepare_stub();

        let rpc_thread = std::thread::spawn(move || {
            let mut aux_rt: tokio::runtime::Runtime = tokio::runtime::Builder::new()
                .enable_all()
                .basic_scheduler()
                .build()
                .expect("build auxiliary runtime");

            println!("start grpc under single-thread runtime");
            aux_rt.block_on(grpc_run()).unwrap()
        });

        tokio::runtime::Handle::current()
            .spawn_blocking(|| rpc_thread.join())
            .await
            .unwrap()
    })
    .unwrap();

    Controller::release_stub();
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

async fn grpc_run() -> Result<(), Box<dyn std::error::Error>> {
    persist::init_persist_timer();
    // grpc_stub.kline_manager.run();

    let addr = "0.0.0.0:50051".parse().unwrap();
    let grpc = GrpcHandler {};
    println!("Starting gprc service");

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("Ctrl-c received, shutting down");
        tx.send(()).ok();
    });

    tonic::transport::Server::builder()
        .add_service(MatchengineServer::new(grpc))
        .serve_with_shutdown(addr, async {
            rx.await.ok();
        })
        .await?;

    println!("Shutted down");
    Ok(())
}
