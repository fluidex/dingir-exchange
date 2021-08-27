use actix_web::{App, HttpServer};
use dingir_exchange::openapi::user::get_user;
use dingir_exchange::restapi::state::{AppCache, AppState};
use fluidex_common::non_blocking_tracing;
use paperclip::actix::web::{self};
use paperclip::actix::{api_v2_operation, OpenApiExt};
use sqlx::postgres::Postgres;
use sqlx::Pool;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::Mutex;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let _guard = non_blocking_tracing::setup();

    let db_url = dingir_exchange::config::Settings::new().db_history;
    log::debug!("Prepared DB connection: {}", &db_url);

    let config = dingir_exchange::restapi::config::Settings::default();
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
        db: Pool::<Postgres>::connect(&db_url).await.unwrap(),
        config,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(user_map.clone())
            .app_data(AppCache::new())
            .wrap_api()
            .service(
                web::scope("/openapi")
                    .route("/ping", web::get().to(ping))
                    .route("/user/{l1addr_or_l2pubkey}", web::get().to(get_user)),
            )
            .with_json_spec_at("/api/spec")
            .build()
    })
    .bind("0.0.0.0:50054")?
    .run()
    .await
}

#[api_v2_operation]
async fn ping() -> Result<&'static str, actix_web::Error> {
    Ok("pong")
}
