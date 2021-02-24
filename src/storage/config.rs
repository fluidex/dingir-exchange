use crate::config;
use super::models::{DbType, TimestampDbType, tablenames, AssetDesc, MarketDesc};
use anyhow::Result;


impl From<AssetDesc> for config::Asset {
    fn from(origin: AssetDesc) -> Self {
        config::Asset {
            name: origin.asset_name,
            prec_show: origin.precision_show as u32,
            prec_save: origin.precision_stor as u32,
        }
    }
}

impl From<MarketDesc> for config::Market
{
    fn from(origin: MarketDesc) -> Self
    {
        let market_name = origin.market_name.unwrap_or(
            origin.base_asset.clone() + "_" + &origin.quote_asset
        );

        config::Market {
            base: config::MarketUnit {
                name: origin.base_asset,
                prec: origin.precision_base as u32,
            },
            quote: config::MarketUnit {
                name: origin.quote_asset,
                prec: origin.precision_quote as u32,
            }, 
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

#[cfg(sqlxverf)]
fn sqlverf_loadasset_from_db() -> impl std::any::Any{
    let t = TimestampDbType::from_timestamp(0, 0);
    sqlx::query_as!(
        AssetDesc,
        "select asset_name, precision_stor, precision_show, create_time from asset where create_time > $1",
        t
    )
}

#[cfg(sqlxverf)]
fn sqlverf_loadmarket_from_db() -> impl std::any::Any{
    let t = TimestampDbType::from_timestamp(0, 0);
    sqlx::query_as!(
        MarketDesc,
        "select id, create_time, base_asset, quote_asset, 
        precision_base, precision_quote, precision_fee,
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

    pub async fn init_asset_from_db<'c, 'e, T>(&'c mut self, db_conn: T) -> Result<Vec<config::Asset>>
    where T: sqlx::Executor<'e, Database=DbType>
    {
        self.assets_load_time = TimestampDbType::from_timestamp(0, 0);
        self.load_asset_from_db(db_conn).await
    }

    pub async fn init_market_from_db<'c, 'e, T>(&'c mut self, db_conn: T) -> Result<Vec<config::Market>>
    where T: sqlx::Executor<'e, Database=DbType> + Send
    {
        self.market_load_time = TimestampDbType::from_timestamp(0, 0);
        self.load_market_from_db(db_conn).await
    }

    //this load market config from database, instead of loading them from the config
    //file
    pub async fn load_asset_from_db<'c, 'e, T>(&'c mut self, db_conn: T) -> Result<Vec<config::Asset>>
    where T: sqlx::Executor<'e, Database=DbType> + Send
    {
        let query = format!("select asset_name, precision_stor, 
        precision_show, create_time from {} where create_time > $1",
        tablenames::ASSET);

        let mut ret : Vec<config::Asset> = Vec::new();
        let mut rows = sqlx::query_as::<_, AssetDesc>(&query).bind(self.market_load_time).fetch(db_conn);

        while let Some(item) = rows.try_next().await? {

            self.assets_load_time = item.create_time.and_then(|t| {
                if self.assets_load_time < t {
                    Some(t)
                }else{
                    None
                }
            }).unwrap_or(self.assets_load_time);
            ret.push(item.into());
        }

        log::info!("Load {} assets and update load time to {}", ret.len(), self.assets_load_time);
        Ok(ret)
    }

    pub async fn load_market_from_db<'c, 'e, T>(&'c mut self, db_conn: T) -> Result<Vec<config::Market>>
    where T: sqlx::Executor<'e, Database=DbType>
    {

        let query = format!("select id, create_time, base_asset, quote_asset, 
        precision_base, precision_quote, precision_fee,
        min_amount, market_name from {} where create_time > $1",
        tablenames::MARKET);

        let mut ret : Vec<config::Market> = Vec::new();
        let mut rows = sqlx::query_as::<_, MarketDesc>(&query).bind(self.market_load_time).fetch(db_conn);

        while let Some(item) = rows.try_next().await? {

            self.market_load_time = item.create_time.and_then(|t| {
                if self.market_load_time < t {
                    Some(t)
                }else{
                    None
                }
            }).unwrap_or(self.market_load_time);
            ret.push(item.into());
        }

        log::info!("Load {} market and update load time to {}", ret.len(), self.market_load_time);
        Ok(ret)
    }

}

pub async fn persist_asset_to_db<'c, 'e, T>(db_conn: T, asset: &config::Asset) -> Result<()>
where T: sqlx::Executor<'e, Database=DbType>
{
    sqlx::query(
        &format!("insert into {} (asset_name, precision_stor, precision_show) values ($1, $2, $3)", tablenames::ASSET)
    ).bind(&asset.name)
    .bind(asset.prec_save as i16)
    .bind(asset.prec_show as i16)
    .execute(db_conn).await?;

    Ok(())
}

pub async fn persist_market_to_db<'c, 'e, T>(db_conn: T, market: &config::Market) -> Result<()>
where T: sqlx::Executor<'e, Database=DbType>
{
    sqlx::query(
        &format!("insert into {} (abase_asset, quote_asset, precision_base, 
            precision_quote, precision_fee, min_amount, market_name) 
            values ($1, $2, $3, $4, $5, $6, $7)", tablenames::MARKET)
    ).bind(&market.base.name)
    .bind(&market.quote.name)
    .bind(market.base.prec as i16)
    .bind(market.quote.prec as i16)
    .bind(market.fee_prec as i16)
    .bind(market.min_amount)
    .bind(&market.name)
    .execute(db_conn).await?;

    Ok(())
}