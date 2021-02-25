
use actix_web::{web, http, Responder};
//use web::Json;
use futures::future::OptionFuture;

use crate::matchengine::server::matchengine::{self, matchengine_client::MatchengineClient};
use crate::storage;
use super::{types, state};

pub mod market {

    use super::*;

    pub async fn add_assets(
        req: web::Form<types::NewAssetReq>,
        app_state: web::Data<state::AppState>
    ) -> impl Responder {
    
        let assets_req = req.into_inner();

        for asset in &assets_req.assets {
            if let Err(e) = storage::config::persist_asset_to_db(&app_state.db, asset, assets_req.force_update).await
            {
                return (e.to_string(), http::StatusCode::INTERNAL_SERVER_ERROR);
            }
            
        }

        (String::from("done"), http::StatusCode::OK)
    }

    pub async fn reload(
        app_state: web::Data<state::AppState>
    ) -> impl Responder {
    
        let mut rpc_cli = MatchengineClient::new(app_state.manage_channel.as_ref().unwrap().clone());

        if let Err(e) = rpc_cli.reload_markets(matchengine::ReloadMarketsRequest{from_scratch: false}).await
        {
            return (e.to_string(), http::StatusCode::INTERNAL_SERVER_ERROR);
        }

        (String::from("done"), http::StatusCode::OK)
    }

    pub async fn add_pair(
        req: web::Form<types::NewTradePairReq>,
        app_state: web::Data<state::AppState>
    ) -> impl Responder {
    
        let trade_pair = req.into_inner();
    
        if let Some(Err(e)) = OptionFuture::from(trade_pair.asset_base.as_ref().map(
            |base_asset| storage::config::persist_asset_to_db(&app_state.db, base_asset, false)
        )).await {
            return (e.to_string(), http::StatusCode::INTERNAL_SERVER_ERROR);
        }

        if let Some(Err(e)) = OptionFuture::from(trade_pair.asset_quote.as_ref().map(
            |base_asset| storage::config::persist_asset_to_db(&app_state.db, base_asset, false)
        )).await {
            return (e.to_string(), http::StatusCode::INTERNAL_SERVER_ERROR);
        }

        if let Err(e) = storage::config::persist_market_to_db(&app_state.db, &trade_pair.market).await
        {
            return (e.to_string(), http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    
        if !trade_pair.not_reload {
            let mut rpc_cli = MatchengineClient::new(app_state.manage_channel.as_ref().unwrap().clone());
            if let Err(e) = rpc_cli.reload_markets(matchengine::ReloadMarketsRequest{from_scratch: false}).await
            {
                return (e.to_string(), http::StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    
        (String::from("done"), http::StatusCode::OK)
    }

}
