#![allow(dead_code)]
//#![allow(unused_imports)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_and_return)]
#![allow(clippy::too_many_arguments)]

mod types;

mod asset;
mod config;
mod controller;
mod database;
mod dto;
mod history;
mod market;
mod message;
mod sqlxextend;
mod models;
mod persist;
//mod schema;
mod sequencer;
mod server;
mod utils;

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
        .basic_scheduler()
        .build()
        .expect("build runtime");
    let mut stub = rt.block_on(prepare()).expect("Init state error");
    rt.block_on(grpc_run(stub)).unwrap();
}

async fn prepare() -> anyhow::Result<Controller>
{
    let mut conf = config_rs::Config::new();
    let config_file = dotenv::var("CONFIG_FILE")?;
    conf.merge(config_rs::File::with_name(&config_file)).unwrap();
    let settings: config::Settings = conf.try_into().unwrap();
    println!("Settings: {:?}", settings);

    let mut conn = ConnectionType::connect(&settings.db_log).await?;
    //TODO: add migrations
    //embedded_migrations::run_with_output(&conn, &mut std::io::stdout())?;
    let mut grpc_stub = Controller::new(settings);
    persist::init_from_db(&mut conn, &mut grpc_stub).await?;
    Ok(grpc_stub)
}

async fn grpc_run(mut grpc_stub: Controller) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        controller::G_STUB = &mut grpc_stub;
    }
    persist::init_persist_timer();

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
