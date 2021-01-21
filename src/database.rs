use std::marker::PhantomData;
use std::collections::{HashMap, VecDeque};
use std::time::{Instant};
use tokio::{task, sync};
use tokio::sync::mpsc::error::TrySendError;

use anyhow::{anyhow, Result};

use crate::models;
use crate::types;

use crate::sqlxextend;
use crate::sqlxextend::*;

use types::DbType;

pub const QUERY_LIMIT: i64 = 1000;
pub const INSERT_LIMIT: i64 = 5000;

//https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=66bb75f8bb7b55d6bc8bfdb9d97ceb79

#[derive(Debug)]
struct ProgTracingStack (Vec<(u64, Option<u64>)>);

impl ProgTracingStack 
{
    fn push(&mut self, n : u64)
    {
        if n >= self.0.last().unwrap_or(&(n, None)).0 {
            self.0.push((n, None));
        }else {
            self.0.push((n, None));
            self.0.sort_by(|a, b| a.0.cmp(&b.0));
        }
    }

    fn is_empty(&self) -> bool {self.0.is_empty()}

    fn pop(&mut self, n : u64) -> Option<u64> 
    {
        if self.is_empty(){
            return None;
        }
        let mut trace = *self.0.last().expect("has verified vector is not empty");

        let mut iter = self.0.iter_mut().rev().skip(1);
        while let Some(trnow) = iter.next() {
            if n > trnow.0 {
                trnow.1 = trace.1.or(Some(n));
                self.0.pop();
                return None;
            }else {
                let tmp = trnow.clone();
                *trnow = trace;
                trace = tmp;                
            }
        };
        self.0.pop();
        return trace.1.or(Some(trace.0));
    }
}

pub type TaskNotifyFlag = HashMap<i32, u64>;
pub struct TaskNotification (i32, u64);

impl TaskNotification {
    fn add_to(self, target : &mut TaskNotifyFlag) {
        if let Some(old) = target.insert(self.0, self.1) {
            if old > self.1 {
                //resume the old value
                target.insert(self.0, old);
            }
        }
    }
}

struct DatabaseWriterTask<T> {
    data: Vec<T>,
    notify_flag: Option<TaskNotifyFlag>,
    benchmark: Option<(Instant, u32)>,
}

impl<T> DatabaseWriterTask<T> {
    fn new() -> Self {
        DatabaseWriterTask::<T>
        {
            data: Vec::new(),
            notify_flag: None,
            benchmark: None,
        }
    }

    fn is_limited(&self) -> bool {self.data.len() >= INSERT_LIMIT as usize}

    fn is_empty(&self) -> bool{self.data.is_empty()}

    fn add_data(&mut self, dt: T, notify: Option<TaskNotification>){
        self.data.push(dt);
        if let Some(notify_v) = notify {
            self.notify_flag = self.notify_flag
            .take()
            .or(Some(TaskNotifyFlag::new()))
            .map(move |mut val| {
                notify_v.add_to(&mut val);
                val
            });
        }
    }

    fn apply_benchmark(mut self) -> Self{
        self.benchmark = Some((Instant::now(), self.data.len() as u32));
        self
    }
}

enum WriterMsg<T>
{
    Data(T, Option<TaskNotification>),
    Done(DatabaseWriterTask<T>),
    Fail(sqlx::Error, DatabaseWriterTask<T>),
    Exit(bool),
}

impl<U> DatabaseWriterTask<U> 
where 
    U: Send + std::marker::Sync + std::fmt::Debug + std::clone::Clone,
    U: 'static + TableSchemas,
    U: for<'r> SqlxAction<'r, sqlxextend::InsertTable, DbType>,
{
    async fn execute(mut self, mut conn : sqlx::pool::PoolConnection<DbType>, mut ret : sync::mpsc::Sender<WriterMsg<U>>)
    {
        let entries = &self.data;

        log::debug!(
            "{} (by batch for {} entries)",
            <InsertTable as CommonSQLQuery<U, sqlx::Postgres>>::sql_statement(),
            entries.len()
        );
        let ret = match InsertTableBatch::sql_query_fine(entries.as_slice(), &mut conn).await {
            Ok(_) => {
                if let Some((now, len)) = self.benchmark {
                    log::debug!(
                        "insert {} items into {} takes {}",
                        len,
                        U::table_name(),
                        now.elapsed().as_secs_f32()
                    );                    
                }
                ret.send(WriterMsg::Done(self)).await
            }
            Err((resident, e)) => {
                self.data = resident;
                ret.send(WriterMsg::Fail(e, self)).await
            }      
        };

        if ret.is_err() {
            log::error!("channel has closed, data lost");
        }else{
            log::debug!("minitask for table {} has normally exit", U::table_name());
        }
        
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseWriterStatus {
    pub pending_count: usize,
}

impl DatabaseWriterStatus {
    fn new() -> Self {
        DatabaseWriterStatus{
            pending_count: 0,
        }
    }

    fn set_pending_count(mut self, v : usize) -> Self {
        self.pending_count = v;
        self
    }
}

pub struct DatabaseWriterEntryImpl<'a, U : std::clone::Clone + Send> (&'a mut sync::mpsc::Sender<WriterMsg<U>>);

impl<U> DatabaseWriterEntryImpl<'_, U>
where U: std::clone::Clone + Send,
{
    pub fn append(self, item: U) -> Result<(), U>{
        self.append_with_notify(item, None)
    }

    pub fn append_with_notify(self, item: U, notify : Option<TaskNotification>) -> Result<(), U>{
        // must not block
        //log::debug!("append item done {:?}", item);
        self.0.try_send(WriterMsg::Data(item, notify))
            .map_err(|e| {
                if let WriterMsg::Data(u, _) = match e {TrySendError::Full(m) => m, TrySendError::Closed(m) => m,} {
                    return u;
                }
                panic!("unexpected msg");
            })
    }

}

pub struct DatabaseWriterEntry<U : std::clone::Clone + Send> (sync::mpsc::Sender<WriterMsg<U>>);

impl<U> DatabaseWriterEntry<U>
where U: std::clone::Clone + Send,
{
    pub fn gen(&mut self) -> DatabaseWriterEntryImpl<'_, U> {
        DatabaseWriterEntryImpl(&mut self.0)
    }
}

pub struct DatabaseWriter<TableTarget, U = TableTarget>
where
    TableTarget: From<U>,
    U: std::clone::Clone + Send,
{
    scheduler: Option<task::JoinHandle<()>>,
    sender: Option<sync::mpsc::Sender<WriterMsg<U>>>,
    
    status: sync::watch::Receiver<DatabaseWriterStatus>,

    config: DatabaseWriterConfig,
    status_send: Option<sync::watch::Sender<DatabaseWriterStatus>>,

    _phantom: PhantomData<TableTarget>,
}

#[derive(Clone, Debug)]
pub struct DatabaseWriterConfig {
    pub apply_benchmark: bool,
    pub spawn_limit: i32,
    pub channel_limit: usize,
}

impl<U> DatabaseWriter<U>
where
    U: std::clone::Clone + Send,
{
    pub fn new(config: &DatabaseWriterConfig) -> DatabaseWriter<U> {

        let (s_tx, s_rx) = sync::watch::channel(DatabaseWriterStatus::new());

        DatabaseWriter::<U> {
            scheduler: None,
            sender: None,
            config: config.clone(),
            status: s_rx,
            status_send: Some(s_tx),
            _phantom: PhantomData,
        }
    }

    pub fn get_entry(&self) -> Option<DatabaseWriterEntry<U>> {
        self.sender.as_ref().map(|sd| DatabaseWriterEntry(sd.clone()))
    }

    pub fn append(&mut self, item: U) -> Result<(), U>{
        self.append_with_notify(item, None)
    }

    pub fn append_with_notify(&mut self, item: U, notify : Option<TaskNotification>) -> Result<(), U>{
        // must not block
        //log::debug!("append item done {:?}", item);
        match &mut self.sender {
            Some(sd) => DatabaseWriterEntryImpl(sd).append_with_notify(item, notify),
            None => Err(item),
        }
    }

    //we consider no block for writer anymore
    pub fn is_block(&self) -> bool {

        match &self.sender {
            Some(_) => false,
            None => true,
        }
    }

    pub fn status(&self) -> DatabaseWriterStatus {
        self.status.borrow().clone()
    }

    pub async fn finish(self) -> types::SimpleResult {
        match self.sender {
            Some(mut sd) => {
                sd.send(WriterMsg::Exit(true)).await
                    .map_err(|e| anyhow!("Send exit notify fail: {}", e))?;
                self.scheduler.unwrap().await
                    .map_err(|e| anyhow!("Wait scheuler exit fail: {}", e))?;
                Ok(())
            },
            None => Err(anyhow!("Not inited")),
        }
    }

    //TOD: what is it?
    pub fn reset(&mut self) {}    
}

struct DatabaseWriterScheduleCtx<T> {
    ctrl_chn : sync::mpsc::Receiver<WriterMsg<T>>,
    ctrl_notify: sync::mpsc::Sender<WriterMsg<T>>,
    pool: sqlx::Pool<DbType>,
    config: DatabaseWriterConfig,
}

impl<U> DatabaseWriterScheduleCtx<U>
where 
    U: Send + std::marker::Sync + std::fmt::Debug + std::clone::Clone,
    U: 'static + TableSchemas,
    U: for<'r> SqlxAction<'r, sqlxextend::InsertTable, DbType>,
{

    async fn schedule(mut self) {

        let mut next_task_stack : VecDeque<DatabaseWriterTask<U>> = VecDeque::new();
        let mut error_task_stack : VecDeque<DatabaseWriterTask<U>> = VecDeque::new();
        let mut spawn_tasks : i32 = 0;
        let mut grace_down = false;

        loop {
            tokio::select! {
                Ok(conn) = self.pool.acquire(), if !error_task_stack.is_empty() => {
                    tokio::spawn(error_task_stack.pop_back().unwrap().execute(conn, self.ctrl_notify.clone()));
                }
                Some(msg) = self.ctrl_chn.recv() => {
                    match msg {
                        WriterMsg::Data(data, notify) => {
                            if next_task_stack.is_empty() || next_task_stack.front().unwrap().is_limited(){
                                next_task_stack.push_front(DatabaseWriterTask::new());
                            }

                            next_task_stack.front_mut().unwrap().add_data(data, notify);
                            if spawn_tasks < self.config.spawn_limit {
                                if let Some(conn) = self.pool.try_acquire() {
                                    spawn_tasks += 1;
                                    let mut task = next_task_stack.pop_back().unwrap();
                                    if self.config.apply_benchmark {
                                        task = task.apply_benchmark();
                                    }
                                    tokio::spawn(task.execute(conn, self.ctrl_notify.clone()));
                                }
                            }
                        },
                        WriterMsg::Done(ctx) => {
                            spawn_tasks -= 1;
                            if !next_task_stack.is_empty() {
                                if let Some(conn) = self.pool.try_acquire() {
                                    spawn_tasks += 1;                        
                                    let mut task = next_task_stack.pop_back().unwrap();
                                    if self.config.apply_benchmark {
                                        task = task.apply_benchmark();
                                    }
                                    tokio::spawn(task.execute(conn, self.ctrl_notify.clone()));
                                }
                            }
                            if grace_down && spawn_tasks == 0 {break;}
                        },
                        WriterMsg::Fail(err, ctx) => {
                            log::error!("exec sql:  fail: {}. retry", err);
                            error_task_stack.push_front(ctx);
                        },
                        WriterMsg::Exit(grace) => {
                            grace_down = true;
                            if !grace || spawn_tasks == 0 {
                                break;
                            }
                        },
                    }    
                }
            }
        }

        if !next_task_stack.is_empty() || !error_task_stack.is_empty() {
            log::error!("Data has lost because of non-grace exit");
        }

        log::info!("db scheduler thread for {}  \texit", U::table_name());
    }    
}

impl<U> DatabaseWriter<U>
where
    U: Send + std::marker::Sync + std::fmt::Debug + std::clone::Clone,
    U: 'static + TableSchemas,
    U: for<'r> SqlxAction<'r, sqlxextend::InsertTable, DbType>,
{

    pub fn start_schedule(mut self, pool:&'_ sqlx::Pool<DbType>) -> Result<Self>{

        let (chn_tx, chn_rx) = sync::mpsc::channel(self.config.channel_limit);
        self.sender = Some(chn_tx.clone());

        let ctx = DatabaseWriterScheduleCtx::<U>{
            ctrl_chn: chn_rx,
            ctrl_notify: chn_tx,
            pool: pool.clone(),
            config: self.config.clone(),
        };

        //no url output (we do not need)
        log::info!("db writer for {} config: {:?}", U::table_name(), ctx.config);
        self.scheduler = Some(tokio::spawn(ctx.schedule()));

        Ok(self)
    }

}

pub type OperationLogSender = DatabaseWriter<models::OperationLog>;
