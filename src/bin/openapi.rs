use actix_web::{App, HttpServer};
use paperclip::actix::{
    api_v2_operation,
    web::{self, Json},
    Apiv2Schema, OpenApiExt,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Apiv2Schema)]
struct Pet {
    name: String,
    id: Option<i64>,
}

#[api_v2_operation]
async fn echo_pet(body: Json<Pet>) -> Result<Json<Pet>, actix_web::Error> {
    Ok(body)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap_api()
            .service(web::resource("/pets").route(web::post().to(echo_pet)))
            .with_json_spec_at("/api/spec")
            .build()
    })
    .bind("0.0.0.0:50054")?
    .run()
    .await
}
