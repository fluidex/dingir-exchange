use std::marker::PhantomData;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use diesel::associations::HasTable;
use diesel::insertable::InsertValues;
use diesel::mysql::{Mysql, MysqlConnection};
use diesel::prelude::*;
use diesel::query_builder::{QueryFragment, UndecoratedInsertRecord, ValuesClause};
use diesel::result::DatabaseErrorKind::UniqueViolation;
use diesel::result::Error::DatabaseError;
use diesel::Insertable;

use anyhow::Result;
use crossbeam_channel::RecvTimeoutError;

use crate::models;
use crate::types;
use crate::types::SimpleResult;

pub const QUERY_LIMIT: i64 = 1000;
pub const INSERT_LIMIT: i64 = 1000;

pub struct DatabaseWriterStatus {
    pub pending_count: usize,
}
pub struct DatabaseWriter<TableTarget, U>
where
    U: std::clone::Clone,
{
    pub sender: crossbeam_channel::Sender<U>,
    pub thread_num: usize,
    pub threads: Vec<JoinHandle<()>>,
    pub thread_config: ThreadConfig<U>,
    phantom: PhantomData<TableTarget>,
}

pub struct DatabaseWriterConfig {
    pub database_url: String,
    pub run_daemon: bool,
}

#[derive(std::clone::Clone)]
pub struct ThreadConfig<U>
where
    U: std::clone::Clone,
{
    pub conn_str: String,
    pub channel_receiver: crossbeam_channel::Receiver<U>,
    pub timer_interval: Duration,
    pub entry_limit: usize,
}

impl<TableType, TableTarget, U, Inner> DatabaseWriter<TableType, U>
where
    TableType: HasTable<Table = TableTarget>,
    TableTarget: 'static,
    TableTarget: diesel::Table,
    TableTarget: std::marker::Send + std::fmt::Debug,
    <TableTarget as QuerySource>::FromClause: QueryFragment<Mysql>,
    U: 'static,
    U: std::marker::Send + std::fmt::Debug + std::clone::Clone,
    U: Insertable<TableTarget, Values = ValuesClause<Inner, TableTarget>>,
    U: UndecoratedInsertRecord<<TableType as HasTable>::Table>,
    Inner: QueryFragment<Mysql> + InsertValues<TableTarget, Mysql>,
{
    pub fn new(config: &DatabaseWriterConfig) -> Result<DatabaseWriter<TableType, U>> {
        // FIXME reconnect? escape?
        // test connection
        //me_util::check_sql_conn(&config.database_url);

        let (sender, receiver) = crossbeam_channel::bounded::<U>(100_000);

        let thread_config: ThreadConfig<U> = ThreadConfig {
            conn_str: config.database_url.clone(),
            channel_receiver: receiver,
            entry_limit: 1000,
            timer_interval: std::time::Duration::from_millis(100),
        };

        let mut writer = DatabaseWriter {
            thread_num: 1,
            sender,
            threads: Vec::new(),
            thread_config,
            phantom: PhantomData,
        };
        if config.run_daemon {
            writer.start_thread();
        }
        Ok(writer)
    }

    pub fn run(config: ThreadConfig<U>) {
        let conn: MysqlConnection = MysqlConnection::establish(config.conn_str.as_ref()).unwrap();
        let mut running = true;
        while running {
            let mut entries: Vec<U> = Vec::new();
            let mut deadline = Instant::now() + config.timer_interval;

            loop {
                match config.channel_receiver.recv_timeout(deadline.duration_since(Instant::now())) {
                    Ok(entry) => {
                        if entries.is_empty() {
                            // Message should have a worst delivery time
                            deadline = Instant::now() + config.timer_interval;
                        }
                        entries.push(entry);
                        if entries.len() >= config.entry_limit {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        break;
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        println!("sql consumer Disconnected");
                        running = false;
                        break;
                    }
                }
            }

            if !entries.is_empty() {
                Self::insert(entries, &conn);
            }
        }

        drop(conn);
    }

    pub fn insert(entries: Vec<U>, conn: &MysqlConnection) {
        // TODO
        let table_name = TableType::table();
        println!("Insert into {:?}: {} values", table_name, entries.len());
        let query = diesel::insert_into(TableType::table()).values(entries);

        loop {
            match conn.execute_returning_count(&query) {
                Ok(_) => {
                    break;
                }
                Err(DatabaseError(UniqueViolation, _)) => {
                    // it may be caused by master-slave replication?
                    println!("SQL ERR: Dup entry, skip. ");
                    break;
                }
                Err(e) => {
                    println!("exec sql:  fail: {}. retry.", e.to_string());
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        }
    }

    pub fn start_thread(&mut self) {
        let mut threads = Vec::new();
        let thread_num = self.thread_num;
        let thread_config = self.thread_config.clone();
        // thread_num is 1 now
        for _ in 0..thread_num {
            let config = thread_config.clone();
            let thread_handle: std::thread::JoinHandle<()> = std::thread::spawn(move || {
                println!("config: {:?}", config.conn_str);
                Self::run(config);
            });
            threads.push(thread_handle);
        }

        self.threads = threads
    }

    pub fn is_block(&self) -> bool {
        self.sender.is_full()
    }

    pub fn append(&self, item: U) {
        // must not block
        println!("append item done {:?}", item);
        self.sender.try_send(item).unwrap();
    }

    pub fn finish(self) -> types::SimpleResult {
        drop(self.sender);
        for handle in self.threads {
            if let Err(e) = handle.join() {
                println!("join threads err {:?} ", e);
            }
        }
        Ok(())
    }
    pub fn status(&self) -> DatabaseWriterStatus {
        DatabaseWriterStatus {
            pending_count: self.sender.len(),
        }
    }
}

pub fn check_sql_conn(conn_str: &str) -> SimpleResult {
    match MysqlConnection::establish(conn_str) {
        Ok(_) => Ok(()),
        Err(e) => simple_err!("invalid conn {} {}", conn_str, e),
    }
}

pub type OperationLogSender = DatabaseWriter<crate::schema::operation_log::table, models::OperationLog>;
