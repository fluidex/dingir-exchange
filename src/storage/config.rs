use super::models::{tablenames, AssetDesc, DbType, MarketDesc, TimestampDbType};
use crate::config;
use anyhow::Result;

impl From<AssetDesc> for config::Asset {
    fn from(origin: AssetDesc) -> Self {
        config::Asset {
            id: origin.id,
            symbol: origin.symbol,
            name: origin.name,
            chain_id: origin.chain_id,
            token_address: origin.token_address,
            rollup_token_id: origin.rollup_token_id,
            prec_show: origin.precision_show as u32,
            prec_save: origin.precision_stor as u32,
            logo_uri: origin.logo_uri,
        }
    }
}

impl From<MarketDesc> for config::Market {
    fn from(origin: MarketDesc) -> Self {
        let market_name = origin.market_name.unwrap_or(origin.base_asset.clone() + "_" + &origin.quote_asset);

        config::Market {
            base: origin.base_asset,
            quote: origin.quote_asset,
            price_prec: origin.precision_price as u32,
            amount_prec: origin.precision_amount as u32,
            fee_prec: origin.precision_fee as u32,
            name: market_name,
            min_amount: origin.min_amount,
        }
    }
}

pub struct MarketConfigs {
    assets_load_time: TimestampDbType,
    market_load_time: TimestampDbType,
}

// TODO: fix this
#[cfg(sqlxverf)]
fn sqlverf_loadasset_from_db() -> impl std::any::Any {
    let t = TimestampDbType::from_timestamp(0, 0);
    sqlx::query_as!(
        AssetDesc,
        "select asset_name, precision_stor, precision_show, create_time from asset where create_time > $1",
        t
    )
}

impl Default for MarketConfigs {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: fix this
#[cfg(sqlxverf)]
fn sqlverf_loadmarket_from_db() -> impl std::any::Any {
    let t = TimestampDbType::from_timestamp(0, 0);
    sqlx::query_as!(
        MarketDesc,
        "select id, create_time, base_asset, quote_asset, 
        precision_amount, precision_price, precision_fee,
        min_amount, market_name from market where create_time > $1",
        t
    )
}

use futures::TryStreamExt;

impl MarketConfigs {
    pub fn new() -> Self {
        MarketConfigs {
            assets_load_time: TimestampDbType::from_timestamp(0, 0),
            market_load_time: TimestampDbType::from_timestamp(0, 0),
        }
    }

    pub fn reset_load_time(&mut self) {
        self.assets_load_time = TimestampDbType::from_timestamp(0, 0);
        self.market_load_time = TimestampDbType::from_timestamp(0, 0);
    }

    //this load market config from database, instead of loading them from the config
    //file
    pub async fn load_asset_from_db<'c, 'e, T>(&'c mut self, db_conn: T) -> Result<Vec<config::Asset>>
    where
        T: sqlx::Executor<'e, Database = DbType> + Send,
    {
        let query = format!(
            "select id, symbol, name, chain_id, token_address, rollup_token_id, precision_stor, precision_show,
            logo_uri, create_time from {} where create_time > $1",
            tablenames::ASSET
        );

        let mut ret: Vec<config::Asset> = Vec::new();
        let mut rows = sqlx::query_as::<_, AssetDesc>(&query).bind(self.market_load_time).fetch(db_conn);

        while let Some(item) = rows.try_next().await? {
            self.assets_load_time = item
                .create_time
                .and_then(|t| if self.assets_load_time < t { Some(t) } else { None })
                .unwrap_or(self.assets_load_time);
            ret.push(item.into());
        }

        log::info!("Load {} assets and update load time to {}", ret.len(), self.assets_load_time);
        Ok(ret)
    }

    pub async fn load_market_from_db<'c, 'e, T>(&'c mut self, db_conn: T) -> Result<Vec<config::Market>>
    where
        T: sqlx::Executor<'e, Database = DbType>,
    {
        let query = format!(
            "select id, create_time, base_asset, quote_asset, 
        precision_amount, precision_price, precision_fee,
        min_amount, market_name from {} where create_time > $1",
            tablenames::MARKET
        );

        let mut ret: Vec<config::Market> = Vec::new();
        let mut rows = sqlx::query_as::<_, MarketDesc>(&query).bind(self.market_load_time).fetch(db_conn);

        while let Some(item) = rows.try_next().await? {
            self.market_load_time = item
                .create_time
                .and_then(|t| if self.market_load_time < t { Some(t) } else { None })
                .unwrap_or(self.market_load_time);
            ret.push(item.into());
        }

        log::info!("Load {} market and update load time to {}", ret.len(), self.market_load_time);
        Ok(ret)
    }
}

// TODO: fix this
#[cfg(sqlxverf)]
fn sqlverf_persist_asset_to_db() -> impl std::any::Any {
    let asset = config::Asset {
        name: String::from("test"),
        prec_save: 0,
        prec_show: 0,
    };

    sqlx::query!(
        "insert into asset (asset_name, precision_stor, precision_show) values ($1, $2, $3) 
        on conflict (asset_name) do update set precision_stor=EXCLUDED.precision_stor, precision_show=EXCLUDED.precision_show",
        &asset.name,
        asset.prec_save as i16,
        asset.prec_show as i16
    )
}

// TODO: chain_id & logo_uri
pub async fn persist_asset_to_db<'c, 'e, T>(db_conn: T, asset: &config::Asset, force: bool) -> Result<()>
where
    T: sqlx::Executor<'e, Database = DbType>,
{
    let query_template = if force {
        format!(
            "insert into {} (id, symbol, name, token_address, rollup_token_id, precision_stor, precision_show) values ($1, $2, $3, $4, $5, $6) 
        on conflict do update set precision_stor=EXCLUDED.precision_stor, precision_show=EXCLUDED.precision_show",
            tablenames::ASSET
        )
    } else {
        format!(
            "insert into {} (id, symbol, name, token_address, rollup_token_id, precision_stor, precision_show) values ($1, $2, $3, $4, $5, $6) on conflict do nothing",
            tablenames::ASSET
        )
    };

    sqlx::query(&query_template)
        .bind(&asset.id)
        .bind(&asset.symbol)
        .bind(&asset.name)
        .bind(&asset.token_address)
        .bind(&asset.rollup_token_id)
        .bind(asset.prec_save as i16)
        .bind(asset.prec_show as i16)
        .execute(db_conn)
        .await?;

    Ok(())
}

pub async fn persist_market_to_db<'c, 'e, T>(db_conn: T, market: &config::Market) -> Result<()>
where
    T: sqlx::Executor<'e, Database = DbType>,
{
    sqlx::query(&format!(
        "insert into {} (base_asset, quote_asset, 
            precision_amount, precision_price, precision_fee, 
            min_amount, market_name) 
            values ($1, $2, $3, $4, $5, $6, $7)",
        tablenames::MARKET
    ))
    .bind(&market.base)
    .bind(&market.quote)
    .bind(market.amount_prec as i16)
    .bind(market.price_prec as i16)
    .bind(market.fee_prec as i16)
    .bind(market.min_amount)
    .bind(&market.name)
    .execute(db_conn)
    .await?;

    Ok(())
}
