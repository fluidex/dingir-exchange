#![allow(dead_code)]
#![allow(clippy::single_component_path_imports)]
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
mod database;
mod grpc_server;
mod history;
mod market;
mod message;
mod utils;

use grpc_server::{GrpcHandler, GrpcStub, MatchengineServer};
use schema::operlog_example;

embed_migrations!();

fn main() {
    env_logger::init();
    dotenv::dotenv().ok();
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

    let connection = MysqlConnection::establish(&settings.db_log)?;
    embedded_migrations::run_with_output(&connection, &mut std::io::stdout())?;

    let mut grpc_stub = GrpcStub::new(settings);

    // LOAD operlog
    let batch_limit: i64 = 1000;
    let mut operlog_start_id: u64 = 0; // exclusive
    loop {
        let operlogs: Vec<models::Operlog> = operlog_example::dsl::operlog_example
            .filter(operlog_example::id.gt(operlog_start_id))
            .order(operlog_example::id.asc())
            .limit(batch_limit)
            .load::<models::Operlog>(&connection)
            .unwrap();
        if operlogs.is_empty() {
            break;
        }
        operlog_start_id = operlogs.last().unwrap().id;
        for log in operlogs {
            println!("replay {} {}", &log.method, &log.params);
            grpc_stub.replay(&log.method, serde_json::from_str(&log.params).unwrap()).unwrap();
        }
    }
    grpc_stub.sequencer.borrow_mut().set_operlog_id(operlog_start_id);

    unsafe {
        grpc_server::G_STUB = Some(&mut grpc_stub);
    }

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
