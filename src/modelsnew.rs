#![allow(unused_imports)]
#![allow(clippy::single_component_path_imports)]

pub type DecimalDbType = rust_decimal::Decimal;
pub type TimestampDbType = sqlx::types::chrono::NaiveDateTime;

pub const BALANCEHISTORY : &str = "balance_history";
pub const ORDERHISTORY : &str = "order_history";
pub const TRADEHISTORY : &str = "trade_history";
pub const OPERATIONLOG : &str = "operation_log";

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct BalanceHistory {
    //for renaming, add #[sqlx(rename = "<row name>")] in corresponding
    //field (not like diesel imply within the derive macro)
    //pub id: i64,
    pub time: TimestampDbType,
    pub user_id: i32,
    pub asset: String,
    pub business: String,
    pub change: DecimalDbType,
    pub balance: DecimalDbType,
    // TODO: change it to jsonb
    pub detail: String,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct OrderHistory {
    pub id: i64,
    pub create_time: TimestampDbType,
    pub finish_time: TimestampDbType,
    pub user_id: i32,
    pub market: String,
    // Type enum: MARKET or LIMIT
    pub t: i16,
    pub side: i16,
    pub price: DecimalDbType,
    pub amount: DecimalDbType,
    pub taker_fee: DecimalDbType,
    pub maker_fee: DecimalDbType,
    pub finished_base: DecimalDbType,
    pub finished_quote: DecimalDbType,
    pub finished_fee: DecimalDbType,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct TradeHistory {
    pub time: TimestampDbType,
    pub user_id: i32,
    pub market: String,
    pub trade_id: i64,
    pub order_id: i64,
    pub counter_order_id: i64,
    pub side: i16,
    pub role: i16,
    pub price: DecimalDbType,
    pub amount: DecimalDbType,
    pub quote_amount: DecimalDbType,
    pub fee: DecimalDbType,
    pub counter_order_fee: DecimalDbType,
}

// Can the following struct be auto generated in diesel?
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct OperationLog {
    pub id: i64,
    pub time: TimestampDbType,
    pub method: String,
    // TODO: change it to jsonb
    pub params: String,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct BalanceSlice {
    pub id: i32,
    pub slice_id: i64, // Unix timestamp
    pub user_id: i32,
    pub asset: String,
    pub t: i16, // Enum: AVAILABLE or FREEZE
    pub balance: DecimalDbType,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct NewBalanceSlice {
    //pub id: i32,
    pub slice_id: i64, // Unix timestamp
    pub user_id: i32,
    pub asset: String,
    pub t: i16, // Enum: AVAILABLE or FREEZE
    pub balance: DecimalDbType,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct OrderSlice {
    pub id: i64,
    pub slice_id: i64,
    // Type enum: MARKET or LIMIT
    pub t: i16,
    pub side: i16,
    pub create_time: TimestampDbType,
    pub update_time: TimestampDbType,
    pub user_id: i32,
    pub market: String,
    //pub source: String,
    pub price: DecimalDbType,
    pub amount: DecimalDbType,
    pub taker_fee: DecimalDbType,
    pub maker_fee: DecimalDbType,
    pub remain: DecimalDbType,
    pub frozen: DecimalDbType,
    pub finished_base: DecimalDbType,
    pub finished_quote: DecimalDbType,
    pub finished_fee: DecimalDbType,
}

// xx_id here means the last persisted entry id
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct SliceHistory {
    pub id: i32,
    pub time: i64,
    pub end_operation_log_id: i64,
    pub end_order_id: i64,
    pub end_trade_id: i64,
}


/* 
    Not like diesel, we still need more code for insert action here 
    May be we could use macro to save these works
*/
use crate::typesnew as types;
use crate::sqlxextend;
use types::DbType;

/* --------------------- models::BalanceHistory -----------------------------*/
impl sqlxextend::TableSchemas for BalanceHistory
{
    fn table_name() -> &'static str {BALANCEHISTORY}
    const ARGN: i32 = 7;
    fn default_argsn() -> Vec<i32>{ vec![1] }
}

impl sqlxextend::BindQueryArg<'_, DbType> for BalanceHistory
{
    fn bind_args<'g, 'q : 'g>(&'q self, arg : &mut impl sqlx::Arguments<'g, Database = DbType>)
    {
        arg.add(self.time);
        arg.add(self.user_id);
        arg.add(&self.asset);
        arg.add(&self.business);
        arg.add(&self.change);
        arg.add(&self.balance);
        arg.add(&self.detail);
    }
}

impl sqlxextend::SqlxAction<'_, sqlxextend::InsertTable, DbType> for BalanceHistory{}

/* --------------------- models::TradeHistory -----------------------------*/
impl sqlxextend::TableSchemas for TradeHistory
{
    fn table_name() -> &'static str {TRADEHISTORY}
    const ARGN: i32 = 13;
    fn default_argsn() -> Vec<i32>{ vec![1] }
}

impl sqlxextend::BindQueryArg<'_, DbType> for TradeHistory
{
    fn bind_args<'g, 'q : 'g>(&'q self, arg : &mut impl sqlx::Arguments<'g, Database = DbType>)
    {
        arg.add(self.time);
        arg.add(self.user_id);
        arg.add(&self.market);
        arg.add(self.trade_id);
        arg.add(self.order_id);
        arg.add(self.counter_order_id);
        arg.add(self.side);        
        arg.add(self.role);        
        arg.add(&self.price);   
        arg.add(&self.amount);   
        arg.add(&self.quote_amount);   
        arg.add(&self.fee);   
        arg.add(&self.counter_order_fee);   
    }
}

impl sqlxextend::SqlxAction<'_, sqlxextend::InsertTable, DbType> for TradeHistory{}

/* --------------------- models::OrderHistory -----------------------------*/
impl sqlxextend::TableSchemas for OrderHistory
{
    fn table_name() -> &'static str {ORDERHISTORY}
    const ARGN: i32 = 14;
    //fn default_argsn() -> Vec<i32>{ vec![1] }
}

impl sqlxextend::BindQueryArg<'_, DbType> for OrderHistory
{
    fn bind_args<'g, 'q : 'g>(&'q self, arg : &mut impl sqlx::Arguments<'g, Database = DbType>)
    {
        arg.add(self.id);
        arg.add(self.create_time);
        arg.add(self.finish_time);
        arg.add(self.user_id);
        arg.add(&self.market);
        arg.add(self.t);
        arg.add(self.side);             
        arg.add(&self.price);   
        arg.add(&self.amount);   
        arg.add(&self.taker_fee);   
        arg.add(&self.maker_fee);   
        arg.add(&self.finished_base);   
        arg.add(&self.finished_quote);   
        arg.add(&self.finished_fee);           
    }
}

impl sqlxextend::SqlxAction<'_, sqlxextend::InsertTable, DbType> for OrderHistory{}

impl sqlxextend::TableSchemas for OperationLog
{
    const ARGN: i32 = 4;
    fn table_name() -> &'static str {OPERATIONLOG}
}

impl sqlxextend::BindQueryArg<'_, DbType> for OperationLog
{
    fn bind_args<'g, 'q : 'g>(&'q self, arg : &mut impl sqlx::Arguments<'g, Database = DbType>)
    {
        arg.add(self.id);
        arg.add(self.time);
        arg.add(&self.method);
        arg.add(&self.params);
    }
}

impl sqlxextend::SqlxAction<'_, sqlxextend::InsertTable, DbType> for OperationLog{}
