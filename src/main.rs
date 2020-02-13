#![allow(dead_code)]
//#![allow(unused_imports)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_and_return)]
#![allow(clippy::too_many_arguments)]

#[macro_use]
extern crate diesel;
use diesel::mysql::MysqlConnection;
use diesel::prelude::*;

#[macro_use]
extern crate diesel_migrations;

mod config;
mod models;
mod schema;
mod types;
mod asset;
mod controller;
mod database;
mod dto;
mod history;
mod market;
mod message;
mod persist;
mod sequencer;
mod server;
mod utils;

use controller::Controller;
use server::{GrpcHandler, MatchengineServer};

embed_migrations!();

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    //simple_logger::init().unwrap();
    let mut rt: tokio::runtime::Runtime = tokio::runtime::Builder::new()
        .enable_all()
        .basic_scheduler()
        .build()
        .expect("build runtime");
    rt.block_on(grpc_run()).unwrap();
}

async fn grpc_run() -> Result<(), Box<dyn std::error::Error>> {
    let mut conf = config_rs::Config::new();
    let config_file = dotenv::var("CONFIG_FILE")?;
    conf.merge(config_rs::File::with_name(&config_file)).unwrap();
    let settings: config::Settings = conf.try_into().unwrap();
    println!("Settings: {:?}", settings);

    let conn = MysqlConnection::establish(&settings.db_log)?;
    embedded_migrations::run_with_output(&conn, &mut std::io::stdout())?;

    let mut grpc_stub = Controller::new(settings);
    persist::init_from_db(&conn, &mut grpc_stub);
    unsafe {
        controller::G_STUB = Some(&mut grpc_stub);
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
