use crate::asset;
use crate::asset::BalanceManager;
use crate::controller::{Controller, G_STUB};
use crate::database;
use crate::market;
use crate::models::{BalanceSlice, NewBalanceSlice, OperationLog, OrderSlice, SliceHistory};
use crate::schema;
use crate::types::SimpleResult;
use crate::utils;
use crate::utils::{decimal_b2r, decimal_r2b, timestamp_to_chrono};
//use cre

use diesel::dsl::count_star;
use diesel::mysql::MysqlConnection;
use diesel::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

use crate::market::Order;
use std::convert::TryFrom;
//use core::panicking::panic;
use std::panic;

pub fn get_last_slice(conn: &MysqlConnection) -> Option<SliceHistory> {
    let slices: Vec<SliceHistory> = schema::slice_history::dsl::slice_history
        .order(schema::slice_history::id.desc())
        .limit(1)
        .load::<SliceHistory>(conn)
        .unwrap();
    slices.get(0).cloned()
}

pub fn load_slice_from_db(conn: &MysqlConnection, slice_id: i64, controller: &mut Controller) {
    // load balance
    let mut last_balance_id = 0;
    loop {
        // least order_id is 1
        let balances: Vec<BalanceSlice> = schema::balance_slice::dsl::balance_slice
            .filter(schema::balance_slice::slice_id.eq(slice_id))
            .filter(schema::balance_slice::id.gt(last_balance_id))
            .order(schema::balance_slice::id.asc())
            .limit(database::QUERY_LIMIT)
            .load::<BalanceSlice>(conn)
            .unwrap();
        for balance in &balances {
            let balance_type = asset::BalanceType::try_from(balance.t).unwrap();
            let amount = decimal_b2r(&balance.balance);
            controller
                .balance_manager
                .borrow_mut()
                .set(balance.user_id, balance_type, &balance.asset, &amount);
        }
        if let Some(slice_balance) = balances.last() {
            last_balance_id = slice_balance.id;
        }
        if balances.len() as i64 != database::QUERY_LIMIT {
            break;
        }
    }
    // load orders
    let mut order_id: u64 = 0;
    loop {
        // least order_id is 1
        let orders: Vec<OrderSlice> = schema::order_slice::dsl::order_slice
            .filter(schema::order_slice::slice_id.eq(slice_id))
            .filter(schema::order_slice::id.gt(order_id))
            .order(schema::order_slice::id.asc())
            .limit(database::QUERY_LIMIT)
            .load::<OrderSlice>(conn)
            .unwrap();
        for order in &orders {
            let market = controller.markets.get_mut(&order.market).unwrap();
            let order_rc = Rc::new(RefCell::new(Order {
                id: order.id,
                type_: market::OrderType::try_from(order.t).unwrap(),
                side: market::OrderSide::try_from(order.side).unwrap(),
                create_time: order.create_time.timestamp_millis() as f64,
                update_time: order.update_time.timestamp_millis() as f64,
                market: market.name,
                user: order.user_id,
                price: decimal_b2r(&order.price),
                amount: decimal_b2r(&order.amount),
                taker_fee: decimal_b2r(&order.taker_fee),
                maker_fee: decimal_b2r(&order.maker_fee),
                left: decimal_b2r(&order.left),
                freeze: decimal_b2r(&order.freeze),
                finished_base: decimal_b2r(&order.finished_base),
                finished_quote: decimal_b2r(&order.finished_quote),
                finished_fee: decimal_b2r(&order.finished_fee),
            }));
            market.insert_order(order_rc);
        }
        if let Some(last_order) = orders.last() {
            order_id = last_order.id;
        }
        if orders.len() as i64 != database::QUERY_LIMIT {
            break;
        }
    }
}

pub fn load_operation_log_from_db(conn: &MysqlConnection, operation_log_start_id: u64, controller: &mut Controller) {
    // LOAD operation_log
    let mut operation_log_start_id: u64 = operation_log_start_id; // exclusive
    loop {
        let operation_logs: Vec<OperationLog> = schema::operation_log::dsl::operation_log
            .filter(schema::operation_log::id.gt(operation_log_start_id))
            .order(schema::operation_log::id.asc())
            .limit(database::QUERY_LIMIT)
            .load::<OperationLog>(conn)
            .unwrap();
        if operation_logs.is_empty() {
            break;
        }
        operation_log_start_id = operation_logs.last().unwrap().id;
        for log in operation_logs {
            println!("replay {} {}", &log.method, &log.params);
            controller.replay(&log.method, &log.params).unwrap();
        }
    }
    controller.sequencer.borrow_mut().set_operation_log_id(operation_log_start_id);
    log::info!("set operation_log_id to {}", operation_log_start_id);
}

pub fn init_from_db(conn: &MysqlConnection, controller: &mut Controller) {
    let last_slice = get_last_slice(conn);
    let mut end_operation_log_id = 0;
    if let Some(slice) = last_slice {
        load_slice_from_db(conn, slice.time, controller);
        end_operation_log_id = slice.end_operation_log_id;
        controller.sequencer.borrow_mut().set_order_id(slice.end_order_id);
        controller.sequencer.borrow_mut().set_trade_id(slice.end_trade_id);
        log::info!("set order_id and trade_id to {} {}", slice.end_order_id, slice.end_trade_id);
    }
    load_operation_log_from_db(conn, end_operation_log_id, controller);
}

pub fn dump_balance(conn: &MysqlConnection, slice_id: i64, balance_manager: &BalanceManager) -> SimpleResult {
    let mut records = Vec::new();
    let mut insert_count: usize = 0;
    for (k, v) in &balance_manager.balances {
        let record = NewBalanceSlice {
            slice_id,
            user_id: k.user_id,
            asset: k.asset.clone(),
            t: k.balance_type as u8,
            balance: decimal_r2b(v),
        };
        records.push(record);
        if records.len() as i64 >= database::INSERT_LIMIT {
            insert_count += records.len();
            diesel::insert_into(schema::balance_slice::table).values(&records).execute(conn)?;

            records.clear();
        }
    }
    if !records.is_empty() {
        insert_count += records.len();
        diesel::insert_into(schema::balance_slice::table).values(&records).execute(conn)?;
    }

    log::debug!("persist {} balances done", insert_count);
    Ok(())
}

pub fn dump_orders(conn: &MysqlConnection, slice_id: i64, controller: &Controller) -> SimpleResult {
    let mut count: usize = 0;
    let mut records = Vec::new();
    for market in controller.markets.values() {
        for order_rc in market.orders.values() {
            let order = *order_rc.borrow_mut();
            let record = OrderSlice {
                id: order.id,
                slice_id,
                t: order.type_ as u8,
                side: order.side as u8,
                create_time: timestamp_to_chrono(order.create_time),
                update_time: timestamp_to_chrono(order.update_time),
                user_id: order.user,
                market: order.market.to_string(),
                price: decimal_r2b(&order.price),
                amount: decimal_r2b(&order.amount),
                taker_fee: decimal_r2b(&order.taker_fee),
                maker_fee: decimal_r2b(&order.maker_fee),
                left: decimal_r2b(&order.left),
                freeze: decimal_r2b(&order.freeze),
                finished_base: decimal_r2b(&order.finished_base),
                finished_quote: decimal_r2b(&order.finished_quote),
                finished_fee: decimal_r2b(&order.finished_fee),
            };
            log::debug!("inserting order {:?}", record);
            records.push(record);
            if records.len() as i64 >= database::INSERT_LIMIT {
                count += records.len();
                diesel::insert_into(schema::order_slice::table).values(&records).execute(conn)?;

                records.clear();
            }
        }
    }

    if !records.is_empty() {
        count += records.len();
        diesel::insert_into(schema::order_slice::table).values(&records).execute(conn)?;
    }

    log::debug!("persist {} orders done", count);

    Ok(())
}

pub fn update_slice_history(conn: &MysqlConnection, slice_id: i64, controller: &Controller) -> SimpleResult {
    let sequencer = controller.sequencer.borrow_mut();
    let slice_history = SliceHistory {
        id: 0,
        time: slice_id,
        end_operation_log_id: sequencer.get_operation_log_id(),
        end_order_id: sequencer.get_order_id(),
        end_trade_id: sequencer.get_trade_id(),
    };
    diesel::insert_into(schema::slice_history::table)
        .values(slice_history)
        .execute(conn)?;
    Ok(())
}

pub fn dump_to_db(conn: &MysqlConnection, slice_id: i64, controller: &Controller) -> SimpleResult {
    log::info!("persisting orders and balances to db");
    dump_orders(conn, slice_id, controller)?;
    dump_balance(conn, slice_id, &controller.balance_manager.borrow())?;
    update_slice_history(conn, slice_id, controller)?;
    Ok(())
}

const SLICE_KEEP_TIME: i64 = 30; //3 * 24 * 3600;

pub fn delete_slice(conn: &MysqlConnection, slice_id: i64) -> SimpleResult {
    diesel::delete(schema::balance_slice::table.filter(schema::balance_slice::dsl::slice_id.eq(slice_id))).execute(conn)?;
    diesel::delete(schema::order_slice::table.filter(schema::order_slice::dsl::slice_id.eq(slice_id))).execute(conn)?;
    diesel::delete(schema::slice_history::table.filter(schema::slice_history::dsl::time.eq(slice_id))).execute(conn)?;
    Ok(())
}

// slice_id: timestamp
pub fn clear_slice(conn: &MysqlConnection, slice_id: i64) -> SimpleResult {
    let count: i64 = schema::slice_history::table
        .filter(schema::slice_history::dsl::time.ge(slice_id - SLICE_KEEP_TIME))
        .select(count_star())
        .first(conn)?;
    log::info!("recent slice count: {}", count);
    let slices: Vec<SliceHistory> = schema::slice_history::table
        .filter(schema::slice_history::dsl::time.lt(slice_id - SLICE_KEEP_TIME))
        .load::<SliceHistory>(conn)?;
    for entry in slices {
        delete_slice(conn, entry.time)?
    }
    Ok(())
}

pub fn make_slice() -> SimpleResult {
    let conn = MysqlConnection::establish("mysql://exchange:exchangeAA9944@@127.0.0.1/exchange")?;
    let controller = unsafe { G_STUB.as_mut().unwrap() };
    let slice_id = utils::current_timestamp() as i64;
    dump_to_db(&conn, slice_id, controller)?;
    clear_slice(&conn, slice_id)?;
    log::info!("make slice done, slice_id {}", slice_id);
    Ok(())
}

pub fn fork_and_make_slice() /*-> SimpleResult*/
{
    match nix::unistd::fork() {
        Ok(nix::unistd::ForkResult::Parent { child, .. }) => {
            println!("Continuing execution in parent process, new child has pid: {}", child);
            //return Ok(());
            return;
        }
        Ok(nix::unistd::ForkResult::Child) => {
            println!("fork success");
        }
        Err(e) => panic!("Fork failed {}", e),
    }
    //env_logger::init();

    // Now we are in the child process
    if let Err(e) = make_slice() {
        log::error!("make slice error {:?}", e);
        std::process::exit(1);
    }
    log::logger().flush();
    println!("make slice done");
    std::process::exit(0);
    // exit the child process
}

/*
pub fn on_timer() {
    let mut now: time_t = time(0 as *mut time_t);
    if now - last_slice_time >= settings.slice_interval as libc::c_long
        && now % settings.slice_interval as libc::c_long <= 5i32 as libc::c_long
    {
        //make_slice(now);
        last_slice_time = now
    };
}
*/

pub fn init_persist_timer() {
    //let duration = std::time::Duration::from_millis(3600 * 1000);

    let duration = std::time::Duration::from_millis(3600 * 1000);
    let mut ticker_dump = tokio::time::interval(duration);
    // use spawn_local here will block the network thread
    tokio::spawn(async move {
        loop {
            ticker_dump.tick().await;
            fork_and_make_slice();
        }
    });
}
