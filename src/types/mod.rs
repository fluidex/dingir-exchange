use num_enum::TryFromPrimitive;
use paperclip::actix::Apiv2Schema;
use serde::{Deserialize, Serialize};

pub type SimpleResult = anyhow::Result<()>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
#[repr(i16)]
pub enum MarketRole {
    MAKER = 1,
    TAKER = 2,
}

// https://stackoverflow.com/questions/4848964/difference-between-text-and-varchar-character-varying
// It seems we don't need varchar(n), text is enough?
// https://github.com/launchbadge/sqlx/issues/237#issuecomment-610696905 must use 'varchar'!!!
// text is more readable than #[repr(i16)] and TryFromPrimitive
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, sqlx::Type, Apiv2Schema)]
#[sqlx(type_name = "varchar")]
#[sqlx(rename_all = "lowercase")]
pub enum OrderSide {
    ASK,
    BID,
}
// TryFromPrimitive
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, sqlx::Type, Apiv2Schema)]
#[sqlx(type_name = "varchar")]
#[sqlx(rename_all = "lowercase")]
pub enum OrderType {
    LIMIT,
    MARKET,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum OrderEventType {
    PUT = 1,
    UPDATE = 2,
    FINISH = 3,
    EXPIRED = 4,
}

//pub type DbType = diesel::mysql::Mysql;
//pub type ConnectionType = diesel::mysql::MysqlConnection;
pub type DbType = sqlx::Postgres;
pub type ConnectionType = sqlx::postgres::PgConnection;
pub type DBErrType = sqlx::Error;
