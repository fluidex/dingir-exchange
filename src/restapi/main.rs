#![allow(dead_code)]
//#![allow(unused_imports)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_and_return)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::single_char_pattern)]
#![allow(clippy::await_holding_refcell_ref)] // FIXME

use actix_web::{web, App, HttpRequest, HttpServer, Responder};
use std::collections::HashMap;
use std::sync::Mutex;
use sqlx::Pool;
use sqlx::postgres::Postgres;

mod errors;
mod mock;
mod tradingview;
mod types;
use tradingview::{chart_config, history, symbols, unix_timestamp};
use types::UserInfo;

pub(crate) struct AppState {
    user_addr_map: Mutex<HashMap<String, UserInfo>>,
    db: sqlx::pool::Pool<Postgres>,
}

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

    let dburl = conf.get_str("db_history").unwrap();

    let user_map = web::Data::new(AppState {
        user_addr_map: Mutex::new(HashMap::new()),
        db: Pool::<Postgres>::connect(&dburl).await.unwrap(),
    });

    log::debug!("Prepared db connection: {}", &dburl);

    HttpServer::new(move || {
        App::new().app_data(user_map.clone()).service(
            web::scope("/restapi")
                .route("/ping", web::get().to(ping))
                .route("/user/{id_or_addr}", web::get().to(get_user))
                .service(
                    web::scope("/tradingview")
                        .route("/time", web::get().to(unix_timestamp))
                        .route("/config", web::get().to(chart_config))
                        .route("/symbols", web::get().to(symbols))
                        .route("/history", web::get().to(history)),
                ),
        )
    })
    .bind(("0.0.0.0", 50053))?
    .run()
    .await
}
