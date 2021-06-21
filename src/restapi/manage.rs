use actix_web::{http, web, Responder};
//use web::Json;
use futures::future::OptionFuture;

use super::{state, types};
use crate::matchengine::server::matchengine::{self, matchengine_client::MatchengineClient};
use crate::storage;

pub mod market {

    use super::*;

    async fn do_reload(app_state: &state::AppState) -> (String, http::StatusCode) {
        let mut rpc_cli = MatchengineClient::new(app_state.manage_channel.as_ref().unwrap().clone());

        if let Err(e) = rpc_cli
            .reload_markets(matchengine::ReloadMarketsRequest { from_scratch: false })
            .await
        {
            return (e.to_string(), http::StatusCode::INTERNAL_SERVER_ERROR);
        }

        (String::from("done"), http::StatusCode::OK)
    }

    pub async fn add_assets(req: web::Json<types::NewAssetReq>, app_state: web::Data<state::AppState>) -> impl Responder {
        let assets_req = req.into_inner();

        for asset in &assets_req.assets {
            log::debug!("Add asset {:?}", asset);
            if let Err(e) = storage::config::persist_asset_to_db(&app_state.db, asset, false).await {
                return (e.to_string(), http::StatusCode::INTERNAL_SERVER_ERROR);
            }
        }

        if !assets_req.not_reload {
            do_reload(&app_state.into_inner()).await
        } else {
            (String::from("done"), http::StatusCode::OK)
        }
    }

    pub async fn reload(app_state: web::Data<state::AppState>) -> impl Responder {
        do_reload(&app_state.into_inner()).await
    }

    pub async fn add_pair(req: web::Json<types::NewTradePairReq>, app_state: web::Data<state::AppState>) -> impl Responder {
        let trade_pair = req.into_inner();

        if let Some(asset) = trade_pair.asset_base.as_ref() {
            if asset.id != trade_pair.market.base {
                return (String::from("Base asset not match"), http::StatusCode::BAD_REQUEST);
            }
        }

        if let Some(asset) = trade_pair.asset_quote.as_ref() {
            if asset.id != trade_pair.market.quote {
                return (String::from("Quote asset not match"), http::StatusCode::BAD_REQUEST);
            }
        }

        if let Some(Err(e)) = OptionFuture::from(
            trade_pair
                .asset_base
                .as_ref()
                .map(|base_asset| storage::config::persist_asset_to_db(&app_state.db, base_asset, false)),
        )
        .await
        {
            return (e.to_string(), http::StatusCode::INTERNAL_SERVER_ERROR);
        }

        if let Some(Err(e)) = OptionFuture::from(
            trade_pair
                .asset_quote
                .as_ref()
                .map(|quote_asset| storage::config::persist_asset_to_db(&app_state.db, quote_asset, false)),
        )
        .await
        {
            return (e.to_string(), http::StatusCode::INTERNAL_SERVER_ERROR);
        }

        if let Err(e) = storage::config::persist_market_to_db(&app_state.db, &trade_pair.market).await {
            return (e.to_string(), http::StatusCode::INTERNAL_SERVER_ERROR);
        }

        if !trade_pair.not_reload {
            do_reload(&app_state.into_inner()).await
        } else {
            (String::from("done"), http::StatusCode::OK)
        }
    }
}
