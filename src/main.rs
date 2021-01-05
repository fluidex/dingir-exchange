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

use tokio::task::LocalSet;
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
    rt.block_on(main_scheme(stub)).unwrap();
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
    persist::MIGRATOR.run(&mut conn).await?;
    let mut grpc_stub = Controller::new(settings);
    persist::init_from_db(&mut conn, &mut grpc_stub).await?;
    Ok(grpc_stub)
}

#[cfg(debug_assertions)]
async fn main_scheme(mut grpc_stub: Controller) -> Result<(), Box<dyn std::error::Error>>
{
    println!("Now we are under debug single-thread running mode");
    let local = LocalSet::new();
    let (tx_stop, mut rx_stop) = tokio::sync::watch::channel(false);
    let stw_chn = grpc_stub.stw_notifier.clone();

    let mainroute = async move{
        grpc_run(grpc_stub).await.unwrap();
        tx_stop.broadcast(true).unwrap();
    };

    if let Some(true) = rx_stop.recv().await {
        //just dump the initialize value
        panic!("main route should not start yet")
    }

    let mainroute_ret = local.spawn_local(mainroute);

    loop {

        let (tx_stw, rx_stw) = tokio::sync::oneshot::channel::<controller::DebugRunTask>();
        stw_chn.replace(Some(tx_stw));

        let ret = local.run_until(async {
            tokio::select! {
                Some(true) = rx_stop.recv() => None,
                any = rx_stw => Some(any),
            }
        }).await;

        if let Some(f) = ret {
            println!("We have Stop-the-world notify, handling it");

            let local_stw = LocalSet::new();

            match f.unwrap(){
                controller::DebugRunTask::Dump(fu) => {local_stw.spawn_local(fu);},
                controller::DebugRunTask::Reset(fu) => {local_stw.spawn_local(fu);},
                controller::DebugRunTask::Reload(fu) => {local_stw.spawn_local(fu);},
            }

            local_stw.await;

            println!("Stop-the-world task done, continue running");
        }else {
            break;
        }
        
    }

    mainroute_ret.await?;
    Ok(())
}

#[cfg(not(debug_assertions))]
async fn main_scheme(mut grpc_stub: Controller) -> Result<(), Box<dyn std::error::Error>>{grpc_run(grpc_stub).await}

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
