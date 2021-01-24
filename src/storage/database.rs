use std::collections::{hash_map, HashMap, VecDeque};
use std::marker::PhantomData;
use std::time::Instant;
use tokio::sync::mpsc::error::TrySendError;
use tokio::{sync, task};

use anyhow::{anyhow, Result};

use crate::models;
use crate::types;

use crate::sqlxextend;
use crate::sqlxextend::*;

use types::DbType;

pub const QUERY_LIMIT: i64 = 1000;
pub const INSERT_LIMIT: i64 = 5000;

//https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=66bb75f8bb7b55d6bc8bfdb9d97ceb79

//tracing the progress in a single tag
#[derive(Debug)]
struct ProgTracingStack(Vec<(u64, Option<u64>)>);

impl std::ops::Deref for ProgTracingStack {
    type Target = Vec<(u64, Option<u64>)>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ProgTracingStack {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ProgTracingStack {
    fn new() -> Self {
        ProgTracingStack(Vec::new())
    }

    fn push_top(&mut self, n: u64) {
        if n >= self.last().unwrap_or(&(n, None)).0 {
            self.push((n, None));
        } else {
            self.push((n, None));
            self.sort_by(|a, b| a.0.cmp(&b.0));
        }
    }

    fn pop_top(&mut self, n: u64) -> Option<u64> {
        if self.is_empty() {
            return None;
        }
        let mut trace = *self.last().expect("has verified vector is not empty");

        for trnow in self.iter_mut().rev().skip(1) {
            if n > trnow.0 {
                trnow.1 = trace.1.or(Some(n));
                self.pop();
                return None;
            } else {
                std::mem::swap(&mut (*trnow), &mut trace)
            }
        }
        self.pop();
        trace.1.or(Some(trace.0))
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    //the size of stack is limited: the maxium is just the
    //uplimit of concurrent task, so we do not bother to
    //use max/min heap
    fn prepare_progtracking_stack() -> ProgTracingStack {

        let mut ret = ProgTracingStack(Vec::new());

        ret.push_top(1);
        ret.push_top(3);
        ret.push_top(5);
        ret.push_top(4);
        ret.push_top(2);

        ret
    }

    #[test]
    fn test_progtracking_stack_push() {

        let case = prepare_progtracking_stack();

        assert_eq!(case[0].0, 1);
        assert_eq!(case[1].0, 2);
        assert_eq!(case[2].0, 3);
        assert_eq!(case[3].0, 4);
        assert_eq!(case[4].0, 5);
    }

    #[test]
    fn test_progtracking_stack_pop_1() {

        let mut case = prepare_progtracking_stack();

        assert_eq!(case.pop_top(1), Some(1));
        assert_eq!(case.pop_top(4), None);
        assert_eq!(case.pop_top(2), Some(2));
        assert_eq!(case.pop_top(3), Some(4));
        assert_eq!(case.pop_top(5), Some(5));
        assert_eq!(case.is_empty(), true);
    }

    #[test]
    fn test_progtracking_stack_pop_2() {

        let mut case = prepare_progtracking_stack();

        assert_eq!(case.pop_top(4), None);
        assert_eq!(case.pop_top(2), None);
        assert_eq!(case.pop_top(1), Some(2));
        assert_eq!(case.pop_top(5), None);
        assert_eq!(case.pop_top(3), Some(5));
        assert_eq!(case.is_empty(), true);
    }    
}

pub type TaskNotifyFlag = HashMap<i32, u64>;
pub struct TaskNotification(i32, u64);

impl TaskNotification {
    fn add_to(self, target: &mut TaskNotifyFlag) {
        if let Some(old) = target.insert(self.0, self.1) {
            if old > self.1 {
                //resume the old value
                target.insert(self.0, old);
            }
        }
    }
}

struct ProgTracing(HashMap<i32, ProgTracingStack>);

impl std::ops::Deref for ProgTracing {
    type Target = HashMap<i32, ProgTracingStack>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ProgTracing {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ProgTracing {
    fn update_from(&mut self, notify: &TaskNotifyFlag) {
        notify.iter().for_each(|item| {
            let (k, v) = item;
            self.entry(*k).or_insert_with(ProgTracingStack::new).push_top(*v);
        })
    }

    fn finish_from(&mut self, notify: TaskNotifyFlag) -> TaskNotifyFlag {
        notify
            .into_iter()
            .filter_map(|item| {
                let (k, v) = item;
                if let hash_map::Entry::Occupied(mut entry) = self.entry(k) {
                    entry.get_mut().pop_top(v).map(|pop_v| (k, pop_v))
                } else {
                    None
                }
            })
            .collect()
    }
}

struct DatabaseWriterTask<T> {
    data: Vec<T>,
    notify_flag: Option<TaskNotifyFlag>,
    benchmark: Option<(Instant, u32)>,
}

impl<T> DatabaseWriterTask<T> {
    fn new() -> Self {
        DatabaseWriterTask::<T> {
            data: Vec::new(),
            notify_flag: None,
            benchmark: None,
        }
    }

    fn is_limited(&self) -> bool {
        self.data.len() >= INSERT_LIMIT as usize
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn add_data(&mut self, dt: T, notify: Option<TaskNotification>) {
        self.data.push(dt);
        if let Some(notify_v) = notify {
            self.notify_flag = self.notify_flag.take().or_else(|| Some(TaskNotifyFlag::new())).map(move |mut val| {
                notify_v.add_to(&mut val);
                val
            });
        }
    }

    fn apply_benchmark(mut self) -> Self {
        self.benchmark = Some((Instant::now(), self.data.len() as u32));
        self
    }
}

enum WriterMsg<T> {
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
    async fn execute(mut self, mut conn: sqlx::pool::PoolConnection<DbType>, ret: sync::mpsc::Sender<WriterMsg<U>>) {
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
        } else {
            log::debug!("minitask for table {} has normally exit", U::table_name());
        }
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseWriterStatus {
    pub pending_count: usize,
    pub spawning_tasks: i32,
}

impl DatabaseWriterStatus {
    fn new() -> Self {
        DatabaseWriterStatus {
            pending_count: 0,
            spawning_tasks: 0,
        }
    }
}

pub struct DatabaseWriterEntryImpl<'a, U: std::clone::Clone + Send>(&'a mut sync::mpsc::Sender<WriterMsg<U>>);

impl<U> DatabaseWriterEntryImpl<'_, U>
where
    U: std::clone::Clone + Send,
{
    pub fn append(self, item: U) -> Result<(), U> {
        self.append_with_notify(item, None)
    }

    pub fn append_with_notify(self, item: U, notify: Option<TaskNotification>) -> Result<(), U> {
        // must not block
        //log::debug!("append item done {:?}", item);
        self.0.try_send(WriterMsg::Data(item, notify)).map_err(|e| {
            if let WriterMsg::Data(u, _) = match e {
                TrySendError::Full(m) => m,
                TrySendError::Closed(m) => m,
            } {
                return u;
            }
            panic!("unexpected msg");
        })
    }
}

pub struct DatabaseWriterEntry<U: std::clone::Clone + Send>(sync::mpsc::Sender<WriterMsg<U>>);

impl<U> DatabaseWriterEntry<U>
where
    U: std::clone::Clone + Send,
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
    complete_notify: sync::watch::Receiver<TaskNotifyFlag>,

    config: DatabaseWriterConfig,
    status_send: Option<sync::watch::Sender<DatabaseWriterStatus>>,
    complete_send: Option<sync::watch::Sender<TaskNotifyFlag>>,

    _phantom: PhantomData<TableTarget>,
}

#[derive(Clone, Debug)]
pub struct DatabaseWriterConfig {
    pub apply_benchmark: bool,
    pub spawn_limit: i32,
    pub capability_limit: usize,
}

impl<U> DatabaseWriter<U>
where
    U: std::clone::Clone + Send,
{
    pub fn new(config: &DatabaseWriterConfig) -> DatabaseWriter<U> {
        let (s_tx, s_rx) = sync::watch::channel(DatabaseWriterStatus::new());
        let (cp_tx, cp_rx) = sync::watch::channel(TaskNotifyFlag::new());

        DatabaseWriter::<U> {
            scheduler: None,
            sender: None,
            config: config.clone(),
            status: s_rx,
            complete_notify: cp_rx,
            status_send: Some(s_tx),
            complete_send: Some(cp_tx),
            _phantom: PhantomData,
        }
    }

    pub fn get_entry(&self) -> Option<DatabaseWriterEntry<U>> {
        self.sender.as_ref().map(|sd| DatabaseWriterEntry(sd.clone()))
    }

    pub fn listen_notify(&self) -> sync::watch::Receiver<TaskNotifyFlag> {
        self.complete_notify.clone()
    }

    pub fn append(&mut self, item: U) -> Result<(), U> {
        self.append_with_notify(item, None)
    }

    pub fn append_with_notify(&mut self, item: U, notify: Option<TaskNotification>) -> Result<(), U> {
        // must not block
        //log::debug!("append item done {:?}", item);
        match &mut self.sender {
            Some(sd) => DatabaseWriterEntryImpl(sd).append_with_notify(item, notify),
            None => Err(item),
        }
    }

    //we consider no block for writer anymore
    pub fn is_block(&self) -> bool {
        self.sender.is_none() || ((self.config.capability_limit as f64 * 0.9) as usize) < self.status().pending_count
    }

    pub fn status(&self) -> DatabaseWriterStatus {
        self.status.borrow().clone()
    }

    pub async fn finish(self) -> types::SimpleResult {
        match self.sender {
            Some(sd) => {
                sd.send(WriterMsg::Exit(true))
                    .await
                    .map_err(|e| anyhow!("Send exit notify fail: {}", e))?;
                self.scheduler
                    .unwrap()
                    .await
                    .map_err(|e| anyhow!("Wait scheuler exit fail: {}", e))?;
                Ok(())
            }
            None => Err(anyhow!("Not inited")),
        }
    }

    //TOD: what is it?
    pub fn reset(&mut self) {}
}

struct DatabaseWriterScheduleCtx<T> {
    ctrl_chn: sync::mpsc::Receiver<WriterMsg<T>>,
    ctrl_notify: sync::mpsc::Sender<WriterMsg<T>>,
    pool: sqlx::Pool<DbType>,
    complete_notify: sync::watch::Sender<TaskNotifyFlag>,
    status_notify: sync::watch::Sender<DatabaseWriterStatus>,

    config: DatabaseWriterConfig,
}

impl<U> DatabaseWriterScheduleCtx<U>
where
    U: Send + std::marker::Sync + std::fmt::Debug + std::clone::Clone,
    U: 'static + TableSchemas,
    U: for<'r> SqlxAction<'r, sqlxextend::InsertTable, DbType>,
{
    async fn schedule(mut self) {
        let mut next_task_stack: VecDeque<DatabaseWriterTask<U>> = VecDeque::new();
        let mut error_task_stack: VecDeque<DatabaseWriterTask<U>> = VecDeque::new();
        let mut notify_tracing = ProgTracing(HashMap::new());
        let mut status_tracing = DatabaseWriterStatus::new();
        let mut grace_down = false;

        loop {
            self.status_notify.send(status_tracing.clone()).ok();

            tokio::select! {
                Ok(conn) = self.pool.acquire(), if !error_task_stack.is_empty() => {
                    tokio::spawn(error_task_stack.pop_back().unwrap().execute(conn, self.ctrl_notify.clone()));
                }
                Ok(conn) = self.pool.acquire(), if (
                        !next_task_stack.is_empty() &&
                        status_tracing.spawning_tasks < self.config.spawn_limit
                )   => {
                    status_tracing.spawning_tasks += 1;
                    let mut task = next_task_stack.pop_back().unwrap();
                    if self.config.apply_benchmark {
                        task = task.apply_benchmark();
                    }
                    status_tracing.pending_count -= task.data.len();
                    if let Some(notifies) = task.notify_flag.as_ref(){
                        notify_tracing.update_from(notifies);
                    }
                    tokio::spawn(task.execute(conn, self.ctrl_notify.clone()));
                }
                Some(msg) = self.ctrl_chn.recv() => {
                    match msg {
                        WriterMsg::Data(data, notify) => {
                            if next_task_stack.is_empty() || next_task_stack.front().unwrap().is_limited(){
                                next_task_stack.push_front(DatabaseWriterTask::new());
                            }
                            next_task_stack.front_mut().unwrap().add_data(data, notify);
                            status_tracing.pending_count += 1;
                        },
                        WriterMsg::Done(mut ctx) => {
                            status_tracing.spawning_tasks -= 1;
                            if let Some(notifies) = ctx.notify_flag.take() {
                                self.complete_notify.send(notify_tracing.finish_from(notifies)).ok();
                            }
                            if grace_down && status_tracing.spawning_tasks == 0 {break;}
                        },
                        WriterMsg::Fail(err, ctx) => {
                            log::error!("exec sql:  fail: {}. retry", err);
                            error_task_stack.push_front(ctx);
                        },
                        WriterMsg::Exit(grace) => {
                            grace_down = true;
                            if !grace || status_tracing.spawning_tasks == 0 {
                                break;
                            }
                        },
                    }
                }
            }
        }

        if !next_task_stack.is_empty() || !error_task_stack.is_empty() {
            log::error!("Data for {} has lost because of non-grace exit", U::table_name());
        }

        log::info!("db scheduler thread for {}  \texit", U::table_name());
    }
}

//by designation message never pile up in channel so we just set
//a reasonable capalicaity.
//Not use unbounded_channel: in case we mess things up, it may be
//difficult to find it has eaten up memory. Instead, we wish
//die fast if code do not work as expected
const CHANNEL_LIMIT: usize = 1000;

impl<U> DatabaseWriter<U>
where
    U: Send + std::marker::Sync + std::fmt::Debug + std::clone::Clone,
    U: 'static + TableSchemas,
    U: for<'r> SqlxAction<'r, sqlxextend::InsertTable, DbType>,
{
    pub fn start_schedule(mut self, pool: &'_ sqlx::Pool<DbType>) -> Result<Self> {
        let (chn_tx, chn_rx) = sync::mpsc::channel(CHANNEL_LIMIT);
        self.sender = Some(chn_tx.clone());

        let ctx = DatabaseWriterScheduleCtx::<U> {
            ctrl_chn: chn_rx,
            ctrl_notify: chn_tx,
            status_notify: self.status_send.take().unwrap(),
            complete_notify: self.complete_send.take().unwrap(),
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
