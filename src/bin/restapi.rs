#![allow(dead_code)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_and_return)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::single_char_pattern)]
#![allow(clippy::await_holding_refcell_ref)] // FIXME

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use sqlx::postgres::Postgres;
use sqlx::Pool;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::Mutex;

use dingir_exchange::restapi;

use restapi::manage::market;
use restapi::personal_history::my_orders;
use restapi::public_history::{order_trades, recent_trades};
use restapi::state::{AppCache, AppState};
use restapi::tradingview::{chart_config, history, search_symbols, symbols, ticker, unix_timestamp};
use restapi::user::get_user;

async fn ping(_req: HttpRequest, _data: web::Data<AppState>) -> impl Responder {
    "pong"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    let mut conf = config_rs::Config::new();
    let config_file = dotenv::var("CONFIG_FILE").unwrap();
    conf.merge(config_rs::File::with_name(&config_file)).unwrap();

    let restapi_cfg: Option<config_rs::Value> = conf.get("restapi").ok();

    let dburl = conf.get_str("db_history").unwrap();
    log::debug!("Prepared db connection: {}", &dburl);

    let config: restapi::config::Settings = restapi_cfg.and_then(|v| v.try_into().ok()).unwrap_or_else(Default::default);

    let manage_channel = if let Some(ep_str) = &config.manage_endpoint {
        log::info!("Connect to manage channel {}", ep_str);
        Some(
            tonic::transport::Endpoint::try_from(ep_str.clone())
                .ok()
                .unwrap()
                .connect()
                .await
                .unwrap(),
        )
    } else {
        None
    };

    let user_map = web::Data::new(AppState {
        user_addr_map: Mutex::new(HashMap::new()),
        manage_channel,
        db: Pool::<Postgres>::connect(&dburl).await.unwrap(),
        config,
    });

    let workers = user_map.config.workers;

    let server = HttpServer::new(move || {
        App::new().app_data(user_map.clone()).app_data(AppCache::new()).service(
            web::scope("/restapi")
                .route("/ping", web::get().to(ping))
                .route("/user/{id_or_addr}", web::get().to(get_user))
                .route("/recenttrades/{market}", web::get().to(recent_trades))
                .route("/ordertrades/{market}/{order_id}", web::get().to(order_trades))
                .route("/closedorders/{market}/{user_id}", web::get().to(my_orders))
                .route("/ticker_{ticker_inv}/{market}", web::get().to(ticker))
                .service(
                    web::scope("/tradingview")
                        .route("/time", web::get().to(unix_timestamp))
                        .route("/config", web::get().to(chart_config))
                        .route("/search", web::get().to(search_symbols))
                        .route("/symbols", web::get().to(symbols))
                        .route("/history", web::get().to(history)),
                )
                .service(if user_map.manage_channel.is_some() {
                    web::scope("/manage").service(
                        web::scope("/market")
                            .route("/reload", web::post().to(market::reload))
                            .route("/tradepairs", web::post().to(market::add_pair))
                            .route("/assets", web::post().to(market::add_assets)),
                    )
                } else {
                    web::scope("/manage")
                        .service(web::resource("/").to(|| HttpResponse::Forbidden().body(String::from("No manage endpoint"))))
                }),
        )
    });

    let server = match workers {
        Some(wr) => server.workers(wr),
        None => server,
    };
    server.bind(("0.0.0.0", 50053))?.run().await
}
