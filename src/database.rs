use std::marker::PhantomData;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossbeam_channel::RecvTimeoutError;
use sqlx::prelude::*;

use crate::models;
use crate::types;

use crate::sqlxextend;
use crate::sqlxextend::*;

use types::ConnectionType;
use types::DbType;

pub const QUERY_LIMIT: i64 = 1000;
pub const INSERT_LIMIT: i64 = 1000;

pub struct DatabaseWriterStatus {
    pub pending_count: usize,
}
pub struct DatabaseWriter<TableTarget, U = TableTarget>
where
    TableTarget: for<'r> SqlxAction<'r, sqlxextend::InsertTable, DbType>,
    TableTarget: From<U>,
    U: std::clone::Clone + Send,
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
    U: std::clone::Clone + Send,
{
    pub conn_str: String,
    pub channel_receiver: crossbeam_channel::Receiver<U>,
    pub timer_interval: Duration,
    pub entry_limit: usize,
}

impl<U> DatabaseWriter<U, U>
where
    U: Send + std::marker::Sync + std::fmt::Debug + std::clone::Clone,
    U: 'static,
    U: for<'r> SqlxAction<'r, sqlxextend::InsertTable, DbType>,
{
    pub fn new(config: &DatabaseWriterConfig) -> Result<DatabaseWriter<U, U>> {
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
        let mut rt: tokio::runtime::Runtime = tokio::runtime::Builder::new()
            .enable_all()
            .basic_scheduler()
            .build()
            .expect("build runtime for workerthread");

        let mut conn = rt.block_on(ConnectionType::connect(config.conn_str.as_ref())).unwrap();
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
                //print the insert sql statement
                println!("{}", sqlxextend::InsertTable::sql_statement::<U>());
                for u in entries.drain(0..) {
                    loop {
                        match rt.block_on(u.sql_query(&mut conn)) {
                            Ok(_) => {
                                break;
                            }
                            Err(sqlx::Error::Database(dberr)) => {
                                if let Some(code) = dberr.code() {
                                    if code == "23505" {
                                        println!("Warning, exec sql duplicated entry, break");
                                        break;
                                    }
                                }
                                println!("exec sql: db fail: {}. retry.", dberr.message());
                            }
                            Err(e) => {
                                println!("exec sql:  fail: {}. retry.", e.to_string());
                                std::thread::sleep(std::time::Duration::from_secs(1));
                            }
                        }
                    }
                }
            }
        }

        drop(conn);
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
    pub fn reset(&mut self) {}
}

/*
pub fn check_sql_conn(conn_str: &str) -> SimpleResult {
    match ConnectionType::connect(conn_str) {
        Ok(_) => Ok(()),
        Err(e) => simple_err!("invalid conn {} {}", conn_str, e),
    }
}
*/

pub type OperationLogSender = DatabaseWriter<models::OperationLog>;
