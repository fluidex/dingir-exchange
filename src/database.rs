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
    pub inner_buffer_size: usize,
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

impl<U, T> DatabaseWriter<T, U>
where
    T: From<U>,
    U: std::clone::Clone + Send,
{
    pub fn append(&self, item: U) {
        // must not block
        //log::debug!("append item done {:?}", item);
        self.sender.try_send(item).unwrap();
    }
}

impl<U> DatabaseWriter<U, U>
where
    U: Send + std::marker::Sync + std::fmt::Debug + std::clone::Clone,
    U: 'static + TableSchemas,
    U: for<'r> SqlxAction<'r, sqlxextend::InsertTable, DbType>,
{
    pub fn new(config: &DatabaseWriterConfig) -> Result<DatabaseWriter<U, U>> {
        // FIXME reconnect? escape?
        // test connection
        //me_util::check_sql_conn(&config.database_url);

        let (sender, receiver) = crossbeam_channel::bounded::<U>(config.inner_buffer_size);

        let thread_config: ThreadConfig<U> = ThreadConfig {
            conn_str: config.database_url.clone(),
            channel_receiver: receiver,
            entry_limit: 1024,
            timer_interval: std::time::Duration::from_millis(100),
        };

        let mut writer = DatabaseWriter {
            thread_num: 4,
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

    pub fn run(idx: usize, config: ThreadConfig<U>) {
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
                let timeout = deadline.checked_duration_since(Instant::now());
                if timeout.is_none() {
                    break;
                }
                match config.channel_receiver.recv_timeout(timeout.unwrap()) {
                    Ok(entry) => {
                        //log::debug!("db writer {} get item, now queue len {}", U::table_name(), config.channel_receiver.len());
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
                        log::info!("db writer thread {} for {}  \texit", idx, U::table_name());
                        running = false;
                        break;
                    }
                }
            }

            if !entries.is_empty() {
                //print the insert sql statement
                log::debug!(
                    "{} (by batch for {} entries)",
                    <InsertTable as CommonSQLQuery<U, sqlx::Postgres>>::sql_statement(),
                    entries.len()
                );
                let insert_start = Instant::now();
                loop {
                    match rt.block_on(InsertTableBatch::sql_query_fine(entries.as_slice(), &mut conn)) {
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
                log::debug!(
                    "insert {} items into {} takes {}",
                    entries.len(),
                    U::table_name(),
                    insert_start.elapsed().as_secs_f32()
                );
            }
        }

        drop(conn);
    }

    pub fn start_thread(&mut self) {
        let mut threads = Vec::new();
        let thread_num = self.thread_num;
        let thread_config = self.thread_config.clone();
        // thread_num is 1 now
        for idx in 0..thread_num {
            let config = thread_config.clone();
            let thread_handle: std::thread::JoinHandle<()> = std::thread::spawn(move || {
                log::info!("db writer config: {:?}", config.conn_str);
                Self::run(idx, config);
            });
            threads.push(thread_handle);
        }

        self.threads = threads
    }

    pub fn is_block(&self) -> bool {
        let l = self.sender.len();
        let full = l >= (self.sender.capacity().unwrap() as f64 * 0.9) as usize;
        if l > 20 {
            log::debug!("db queue size {} for {}", self.sender.len(), U::table_name());
        }
        if full {
            log::warn!("db queue is full for {}", U::table_name());
        }
        full
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
        Err(e) => Err(anyhow!("invalid conn {} {}", conn_str, e)),
    }
}
*/

pub type OperationLogSender = DatabaseWriter<models::OperationLog>;
