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
use std::sync::Mutex;
use std::convert::TryFrom;

use dingir_exchange::restapi;

use restapi::manage::market;
use restapi::personal_history::my_orders;
use restapi::public_history::{order_trades, recent_trades};
use restapi::state::{AppCache, AppState};
use restapi::tradingview::{chart_config, history, symbols, ticker, unix_timestamp};
use restapi::types::UserInfo;

async fn ping(_req: HttpRequest, _data: web::Data<AppState>) -> impl Responder {
    "pong"
}

async fn get_user(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    let user_id = req.match_info().get("id_or_addr").unwrap();
    if user_id.starts_with("0x") {
        let mut user_map = data.user_addr_map.lock().unwrap();
        if !user_map.contains_key(user_id) {
            let count = user_map.len();
            user_map.insert(user_id.to_string(), UserInfo { user_id: count as i64 });
        }
        let user_info = *user_map.get(user_id).unwrap();
        web::Json(user_info)
    } else {
        unimplemented!()
    }
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

    let config : restapi::config::Settings = 
        restapi_cfg.and_then(|v| v.try_into().ok()).unwrap_or_else(Default::default);

    let manage_channel = if let Some(ep_str) = &config.manage_endpoint {
        log::info!("Connect to manage channel {}", ep_str);
        Some(tonic::transport::Endpoint::try_from(ep_str.clone()).ok()
        .unwrap().connect().await.unwrap())
    }else{
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
                        .route("/symbols", web::get().to(symbols))
                        .route("/history", web::get().to(history)),
                )
                .service(
                    if user_map.manage_channel.is_some() {
                        web::scope("/manage")
                            .service(web::scope("/market")
                                .route("/reload", web::post().to(market::reload))
                                .route("/addpair", web::post().to(market::add_pair))
                                .route("/assets", web::post().to(market::add_assets)))
                    }else {
                        web::scope("/manage")
                        .service(web::resource("/").to(||
                            HttpResponse::Forbidden()
                            .body(String::from("No manage endpoint"))))
                    }                   
                ),
        )
    });

    let server = match workers {
        Some(wr) => server.workers(wr),
        None => server,
    };
    server.bind(("0.0.0.0", 50053))?.run().await
}
