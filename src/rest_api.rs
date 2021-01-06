use actix_web::{web, App, HttpRequest, HttpServer, Responder};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Serialize, Copy, Clone)]
struct UserInfo {
    user_id: i64,
}

struct AppState {
    user_addr_map: Mutex<HashMap<String, UserInfo>>,
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
    let user_map = web::Data::new(AppState {
        user_addr_map: Mutex::new(HashMap::new()),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(user_map.clone())
            .route("/ping", web::get().to(ping))
            .route("/user/{id_or_addr}", web::get().to(get_user))
    })
    .bind(("0.0.0.0", 50053))?
    .run()
    .await
}
