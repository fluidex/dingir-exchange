use crate::asset;
use crate::asset::BalanceManager;
use crate::controller::Controller;
use crate::database;
use crate::models;
use crate::types::SimpleResult;
use crate::utils;
use crate::utils::FTimestamp;
use crate::{config, storage};
use models::{tablenames, BalanceSlice, BalanceSliceInsert, OperationLog, OrderSlice, SliceHistory};

use crate::sqlxextend::*;
use sqlx::migrate::Migrator;
use sqlx::Connection;

use crate::market::Order;
use std::convert::TryFrom;
use std::time::{Duration, Instant};

use crate::types;
use types::{ConnectionType, DbType};

//migration
pub static MIGRATOR: Migrator = sqlx::migrate!(); // defaults to "./migrations"

#[cfg(sqlxverf)]
fn sqlverf_get_last_slice() -> impl std::any::Any {
    sqlx::query!("select * from slice_history order by id desc limit 1")
}

#[test]
fn utest_get_last_slice() {
    assert_eq!(
        format!("select * from {} order by id desc limit 1", tablenames::SLICEHISTORY),
        "select * from slice_history order by id desc limit 1"
    );
}

pub async fn get_last_slice(conn: &mut ConnectionType) -> Option<SliceHistory> {
    let query = format!("select * from {} order by id desc limit 1", tablenames::SLICEHISTORY);

    sqlx::query_as(&query).fetch_optional(conn).await.unwrap()
    /*    match sqlx::query_as(&query).fetch_optional(conn).await {
        Ok(s) => Some(s),
        Err(sqlx::Error::RowNotFound) => None,
        Err(e) => panic!(e),
    }*/
}

#[cfg(sqlxverf)]
fn sqlverf_load_slice_from_db() -> impl std::any::Any {
    let last_balance_id = 0;
    let slice_id: i64 = 1;
    let order_id: i64 = 0;
    (
        sqlx::query!(
            "select * from balance_slice where slice_id = $1 and id > $2 order by id asc limit 1000",
            slice_id,
            last_balance_id
        ),
        sqlx::query!(
            "select * from order_slice where slice_id = $1 and id > $2 order by id asc limit 1000",
            slice_id,
            order_id
        ),
    )
}

#[test]
fn utest_load_slice_from_db() {
    assert_eq!(
        format!(
            "select * from {} where slice_id = $1 and id > $2 order by id asc limit {}",
            tablenames::BALANCESLICE,
            database::QUERY_LIMIT
        ),
        "select * from balance_slice where slice_id = $1 and id > $2 order by id asc limit 1000"
    );

    assert_eq!(
        format!(
            "select * from {} where slice_id = $1 and id > $2 order by id asc limit {}",
            tablenames::ORDERSLICE,
            database::QUERY_LIMIT
        ),
        "select * from order_slice where slice_id = $1 and id > $2 order by id asc limit 1000"
    );
}

pub async fn load_slice_from_db(conn: &mut ConnectionType, slice_id: i64, controller: &mut Controller) {
    // load balance
    let mut last_balance_id = 0;
    let balance_query = format!(
        "select * from {} where slice_id = $1 and id > $2 order by id asc limit {}",
        tablenames::BALANCESLICE,
        database::QUERY_LIMIT
    );

    loop {
        // least order_id is 1
        let balances: Vec<BalanceSlice> = sqlx::query_as(&balance_query)
            .bind(slice_id)
            .bind(last_balance_id)
            .fetch_all(&mut *conn)
            .await
            .unwrap();

        for balance in &balances {
            let balance_type = asset::BalanceType::try_from(balance.t).unwrap();
            let amount = balance.balance;
            controller
                .balance_manager
                .set(balance.user_id as u32, balance_type, &balance.asset, &amount);
        }
        if let Some(slice_balance) = balances.last() {
            last_balance_id = slice_balance.id;
        }
        if balances.len() as i64 != database::QUERY_LIMIT {
            break;
        }
    }
    // load orders
    let mut order_id: i64 = 0;
    let order_query = format!(
        "select * from {} where slice_id = $1 and id > $2 order by id asc limit {}",
        tablenames::ORDERSLICE,
        database::QUERY_LIMIT
    );
    loop {
        // least order_id is 1
        let orders: Vec<OrderSlice> = sqlx::query_as(&order_query)
            .bind(slice_id)
            .bind(order_id)
            .fetch_all(&mut *conn)
            .await
            .unwrap();
        for order in &orders {
            let market = controller.markets.get_mut(&order.market).unwrap();
            let order = Order {
                id: order.id as u64,
                type_: order.order_type,
                side: order.order_side,
                create_time: FTimestamp::from(&order.create_time).0,
                update_time: FTimestamp::from(&order.update_time).0,
                market: market.name.into(),
                user: order.user_id as u32,
                price: order.price,
                amount: order.amount,
                taker_fee: order.taker_fee,
                maker_fee: order.maker_fee,
                remain: order.remain,
                frozen: order.frozen,
                finished_base: order.finished_base,
                finished_quote: order.finished_quote,
                finished_fee: order.finished_fee,
            };
            market.insert_order(order);
        }
        if let Some(last_order) = orders.last() {
            order_id = last_order.id;
        }
        if orders.len() as i64 != database::QUERY_LIMIT {
            break;
        }
    }
}

#[cfg(sqlxverf)]
fn sqlverf_load_operation_log_from_db() -> impl std::any::Any {
    let operation_log_start_id: i64 = 0;
    sqlx::query!(
        "select * from operation_log where id > $1 order by id asc limit 1000",
        operation_log_start_id
    )
}

#[test]
fn utest_load_operation_log_from_db() {
    assert_eq!(
        format!(
            "select * from {} where id > $1 order by id asc limit {}",
            tablenames::OPERATIONLOG,
            database::QUERY_LIMIT
        ),
        "select * from operation_log where id > $1 order by id asc limit 1000"
    );
}

pub async fn load_operation_log_from_db(conn: &mut ConnectionType, operation_log_start_id: u64, controller: &mut Controller) {
    // LOAD operation_log
    let mut operation_log_start_id = operation_log_start_id as i64; // exclusive
    let query = format!(
        "select * from {} where id > $1 order by id asc limit {}",
        tablenames::OPERATIONLOG,
        database::QUERY_LIMIT
    );

    loop {
        let operation_logs: Vec<OperationLog> = sqlx::query_as(&query)
            .bind(operation_log_start_id)
            .fetch_all(&mut *conn)
            .await
            .unwrap();

        if operation_logs.is_empty() {
            break;
        }
        operation_log_start_id = operation_logs.last().unwrap().id;
        for log in operation_logs {
            log::info!("replay {} {}", &log.method, &log.params);
            controller.replay(&log.method, &log.params).unwrap();
        }
    }
    controller.sequencer.set_operation_log_id(operation_log_start_id as u64);
    log::info!("set operation_log_id to {}", operation_log_start_id);
}

pub use storage::config::MarketConfigs;

pub async fn init_config_from_db(conn: &mut ConnectionType, config: &mut config::Settings) -> anyhow::Result<MarketConfigs> {
    let mut market_cfg = MarketConfigs::new();

    //replace configs data with which loadedfrom db
    config.assets = market_cfg.load_asset_from_db(&mut *conn).await?;
    config.markets = market_cfg.load_market_from_db(&mut *conn).await?;
    Ok(market_cfg)
}

pub async fn init_from_db(conn: &mut ConnectionType, controller: &mut Controller) -> anyhow::Result<()> {
    let last_slice = get_last_slice(conn).await;
    let mut end_operation_log_id = 0;
    if let Some(slice) = last_slice {
        log::debug!("last slice {:?}", slice);
        load_slice_from_db(conn, slice.time, controller).await;
        end_operation_log_id = slice.end_operation_log_id;
        controller.sequencer.set_order_id(slice.end_order_id as u64);
        controller.sequencer.set_trade_id(slice.end_trade_id as u64);
        log::info!("set order_id and trade_id to {} {}", slice.end_order_id, slice.end_trade_id);
    }
    load_operation_log_from_db(conn, end_operation_log_id as u64, controller).await;
    Ok(())
}

const DUMPING_SET_LIMIT: usize = 10000;

fn collect_n<T: std::iter::Iterator>(iter: &mut T, n: usize, mut record: Vec<T::Item>) -> Vec<T::Item> {
    if record.len() >= n {
        return record;
    }

    for i in iter {
        record.push(i);
        if record.len() >= n {
            return record;
        }
    }

    record
}

async fn dump_records<Q, T>(mut iter: T, n: usize, conn: &mut ConnectionType) -> anyhow::Result<usize>
where
    Q: Clone + TableSchemas,
    Q: for<'r> SqlxAction<'r, InsertTable, DbType>,
    T: std::iter::Iterator<Item = Q>,
{
    let mut records: Vec<Q> = collect_n(&mut iter, n, Vec::new());
    let mut inserted_count: usize = 0;
    while !records.is_empty() {
        inserted_count += records.len();
        records = match InsertTableBatch::sql_query_fine(records.as_slice(), &mut *conn).await {
            Err((resident, e)) => {
                log::error!(
                    "dump balance encounter error {}, {} record left, wouldwait and retry",
                    e,
                    resident.len()
                );
                //substract failed part
                inserted_count -= resident.len();
                //for each error we wait 1s

                tokio::time::sleep(Duration::from_secs(1)).await;
                resident
            }
            Ok(_) => Vec::new(),
        };
    }
    Ok(inserted_count)
}

pub async fn dump_balance(conn: &mut ConnectionType, slice_id: i64, balance_manager: &BalanceManager) -> SimpleResult {
    let records_iter = balance_manager.balances.iter().map(|item| {
        let (k, v) = item;
        BalanceSliceInsert {
            slice_id,
            user_id: k.user_id as i32,
            asset: k.asset.clone(),
            t: k.balance_type as i16,
            balance: *v,
        }
    });

    let insert_count = dump_records(records_iter, DUMPING_SET_LIMIT, conn).await?;
    log::debug!("persist {} balances done", insert_count);

    Ok(())
}

pub async fn dump_orders(conn: &mut ConnectionType, slice_id: i64, controller: &Controller) -> SimpleResult {
    let records_iter = controller
        .markets
        .values()
        .flat_map(|market| market.orders.values())
        .map(|order_rc| {
            let order = order_rc.borrow();
            OrderSlice {
                id: order.id as i64,
                slice_id,
                order_type: order.type_,
                order_side: order.side,
                create_time: FTimestamp(order.create_time).into(),
                update_time: FTimestamp(order.update_time).into(),
                user_id: order.user as i32,
                market: order.market.to_string(),
                price: order.price,
                amount: order.amount,
                taker_fee: order.taker_fee,
                maker_fee: order.maker_fee,
                remain: order.remain,
                frozen: order.frozen,
                finished_base: order.finished_base,
                finished_quote: order.finished_quote,
                finished_fee: order.finished_fee,
            }
        });

    let insert_count = dump_records(records_iter, DUMPING_SET_LIMIT, conn).await?;
    log::debug!("persist {} orders done", insert_count);
    Ok(())
}

pub async fn update_slice_history(conn: &mut ConnectionType, slice_id: i64, controller: &Controller) -> SimpleResult {
    let sequencer = &controller.sequencer;
    let slice_history = SliceHistory {
        time: slice_id,
        end_operation_log_id: sequencer.get_operation_log_id() as i64,
        end_order_id: sequencer.get_order_id() as i64,
        end_trade_id: sequencer.get_trade_id() as i64,
    };

    slice_history.sql_query(conn).await?;
    Ok(())
}

pub async fn dump_to_db(conn: &mut ConnectionType, slice_id: i64, controller: &Controller) -> SimpleResult {
    log::info!("persisting orders and balances to db");
    dump_orders(conn, slice_id, controller).await?;
    dump_balance(conn, slice_id, &controller.balance_manager).await?;
    update_slice_history(conn, slice_id, controller).await?;
    Ok(())
}

#[cfg(sqlxverf)]
fn sqlverf_delete_slice() -> impl std::any::Any {
    let slice_id: i64 = 0;
    sqlx::query!("delete from balance_slice where slice_id = $1", slice_id)
}

#[test]
fn utest_delete_slice() {
    assert_eq!(
        format!("delete from {} where slice_id = $1", tablenames::BALANCESLICE),
        "delete from balance_slice where slice_id = $1"
    );
}

const SLICE_KEEP_TIME: i64 = 30; //3 * 24 * 3600;

pub async fn delete_slice(conn: &mut ConnectionType, slice_id: i64) -> SimpleResult {
    sqlx::query(&format!("delete from {} where slice_id = $1", tablenames::BALANCESLICE))
        .bind(slice_id)
        .execute(&mut *conn)
        .await?;
    sqlx::query(&format!("delete from {} where slice_id = $1", tablenames::ORDERSLICE))
        .bind(slice_id)
        .execute(&mut *conn)
        .await?;
    sqlx::query(&format!("delete from {} where time = $1", tablenames::SLICEHISTORY))
        .bind(slice_id)
        .execute(&mut *conn)
        .await?;

    Ok(())
}

#[cfg(sqlxverf)]
fn sqlverf_clear_slice() -> impl std::any::Any {
    let slice_id: i64 = 0;
    (
        sqlx::query!("select count(*) from slice_history where time > $1", slice_id),
        sqlx::query!("select time from slice_history where time <= $1", slice_id),
    )
}

#[test]
fn utest_clear_slice() {
    assert_eq!(
        format!("select count(*) from {} where time > $1", tablenames::SLICEHISTORY),
        "select count(*) from slice_history where time > $1"
    );
    assert_eq!(
        format!("select time from {} where time <= $1", tablenames::SLICEHISTORY),
        "select time from slice_history where time <= $1"
    );
}

// slice_id: timestamp
pub async fn clear_slice(conn: &mut ConnectionType, slice_id: i64) -> SimpleResult {
    let count: i64 = sqlx::query_scalar(&format!("select count(*) from {} where time > $1", tablenames::SLICEHISTORY))
        .bind(slice_id - SLICE_KEEP_TIME)
        .fetch_one(&mut *conn)
        .await?;
    log::info!("recent slice count: {}", count);
    let slices: Vec<i64> = sqlx::query_scalar(&format!("select time from {} where time <= $1", tablenames::SLICEHISTORY))
        .bind(slice_id - SLICE_KEEP_TIME)
        .fetch_all(&mut *conn)
        .await?;
    for entry_time in slices {
        delete_slice(&mut *conn, entry_time).await?;
    }
    Ok(())
}

pub async fn make_slice(controller: &Controller) -> SimpleResult {
    //let url = "postgres://exchange:exchange_AA9944@127.0.0.1/exchange";
    let url = &controller.settings.db_log;
    let mut conn = ConnectionType::connect(url).await?;
    let slice_id = utils::current_timestamp() as i64;
    let timing = Instant::now();
    dump_to_db(&mut conn, slice_id, controller).await?;
    clear_slice(&mut conn, slice_id).await?;
    log::info!(
        "make slice done, slice_id {}, use {} secs",
        slice_id,
        timing.elapsed().as_secs_f32()
    );

    Ok(())
}

use std::panic;

#[cfg(target_family = "windows")]
pub fn do_forking() -> bool {
    log::error!("windows platform has no fork");
    false
}

#[cfg(not(target_family = "windows"))]
fn do_forking() -> bool {
    unsafe {
        // clean last child process by waitpid
        let wait_res = nix::sys::wait::waitpid(nix::unistd::Pid::from_raw(-1), Some(nix::sys::wait::WaitPidFlag::WNOHANG));
        log::info!("waitpid result: {:#?}", wait_res);
        match nix::unistd::fork() {
            Ok(nix::unistd::ForkResult::Parent { child, .. }) => {
                log::info!("Continuing execution in parent process, new child has pid: {}", child);
                false
            }
            Ok(nix::unistd::ForkResult::Child) => {
                log::info!("fork success");
                true
            }
            //if fork fail? should we panic? this will make the main process exit
            //purpose to do that?
            Err(e) => panic!("Fork failed {}", e),
        }
    }
}

/// # Safety
///
/// Safe by designation
pub unsafe fn fork_and_make_slice(controller: *const Controller) /*-> SimpleResult*/
{
    if !do_forking() {
        return;
    }
    //env_logger::init();

    // Now we are in the child process

    //tokio runtime in current thread would highly possible being ruined after fork
    //so we put our task under new thread, with another tokio runtime

    let controller = controller.as_ref().unwrap();

    let thread_handle = std::thread::spawn(move || {
        let rt: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread()
            .thread_name("matchengine_dumper")
            .enable_all()
            .build()
            .expect("build another runtime for slice-making");

        if let Err(e) = rt.block_on(make_slice(controller)) {
            // TODO: it seems sometimes no stderr/stdout is printed here. check it later
            panic!("panic {:?}", e);
        }
    });

    let exitcode = match thread_handle.join() {
        Err(e) => {
            log::error!("make slice fail: {:?}", e);
            1
        }
        _ => {
            log::info!("make slice done");
            0
        }
    };

    log::logger().flush();

    //die fast
    std::process::exit(exitcode);
}
