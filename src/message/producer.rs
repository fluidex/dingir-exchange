use anyhow::Result;
use crossbeam_channel::{RecvTimeoutError, TryRecvError};
use fluidex_common::rdkafka::client::ClientContext;
use fluidex_common::rdkafka::config::ClientConfig;
use fluidex_common::rdkafka::error::{KafkaError, RDKafkaErrorCode};
use fluidex_common::rdkafka::producer::{BaseProducer, BaseRecord, DeliveryResult, Producer, ProducerContext};
use fluidex_common::rdkafka::util::{IntoOpaque, Timeout};
use std::time::Duration;

pub type SimpleDeliverResult = Result<(), KafkaError>;

pub trait MessageScheme: Default + Sync + Send {
    type DeliverOpaque: IntoOpaque;
    type K: Into<String>;
    type V: Into<String>;

    fn settings() -> Vec<(Self::K, Self::V)> {
        vec![]
    }
    fn is_full(&self) -> bool;
    fn on_message(&mut self, title_tip: &'static str, message: String);
    fn pop_up(&mut self) -> Option<BaseRecord<'_, str, str, Self::DeliverOpaque>>;
    fn commit(&mut self, isfailed: Option<Self::DeliverOpaque>);
    fn deliver_commit(&mut self, result: SimpleDeliverResult, opaque: Self::DeliverOpaque);
}

pub struct RdProducerContext<T: MessageScheme> {
    //we use unboound channel to simulate a continuation(?)
    delivery_record: crossbeam_channel::Sender<(SimpleDeliverResult, T::DeliverOpaque)>,
    delivery_record_get: crossbeam_channel::Receiver<(SimpleDeliverResult, T::DeliverOpaque)>,
    //_phantom : std::marker::PhantomData<T>,
}

impl<T: MessageScheme> Default for RdProducerContext<T> {
    fn default() -> Self {
        let (s, r) = crossbeam_channel::unbounded();

        Self {
            delivery_record: s,
            delivery_record_get: r,
        }
    }
}

impl<T: MessageScheme> ClientContext for RdProducerContext<T> {}
impl<T: MessageScheme> ProducerContext for RdProducerContext<T> {
    type DeliveryOpaque = T::DeliverOpaque;
    fn delivery(&self, result: &DeliveryResult, opaque: Self::DeliveryOpaque) {
        self.delivery_record
            .send((
                match result.as_ref() {
                    Err((err, _)) => Err(err.clone()),
                    Ok(_) => Ok(()),
                },
                opaque,
            ))
            .ok();
    }
}

//provide a running kafka producer instance which keep sending message under the full-ordering scheme
//it simply block the Sender side of crossbeam_channel when the deliver queue is full, and quit
//only when the sender side is closed
impl<T: MessageScheme> RdProducerContext<T> {
    pub fn new_producer(self, brokers: &str) -> Result<BaseProducer<Self>> {
        let mut config = ClientConfig::new();
        config.set("bootstrap.servers", brokers);
        T::settings().into_iter().for_each(|item| {
            let (k, v) = item;
            config.set(k, v);
        });

        let producer = config.create_with_context(self)?;
        Ok(producer)
    }

    pub fn run_default(producer: BaseProducer<Self>, receiver: crossbeam_channel::Receiver<(&'static str, String)>) {
        let message_scheme = T::default();
        Self::run(producer, message_scheme, receiver);
    }

    pub fn run(producer: BaseProducer<Self>, mut message_scheme: T, receiver: crossbeam_channel::Receiver<(&'static str, String)>) {
        Self::run_loop(&producer, &mut message_scheme, receiver);

        //flush producer before exit
        while let Some(msg) = message_scheme.pop_up() {
            let send_ret = match producer.send(msg) {
                Ok(_) => None,
                Err((KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull), rec)) => {
                    //when queue is full, simply made some polling and retry
                    producer.poll(Duration::from_millis(100));
                    Some(rec.delivery_opaque)
                }
                Err((err, _)) => {
                    log::error!("kafka encounter error when shutdown: {}", err);
                    //TODO: so what should we do? try handling / waiting or just quit?
                    return;
                }
            };
            message_scheme.commit(send_ret);
        }

        producer.flush(Timeout::Never);
        log::info!("kafka producer running terminated");
    }

    fn run_loop(producer: &BaseProducer<Self>, message_scheme: &mut T, receiver: crossbeam_channel::Receiver<(&'static str, String)>) {
        let timeout_interval = Duration::from_millis(100);
        let delivery_report = &producer.context().delivery_record_get;
        // last_poll == 0 means msg canot be sent out
        let mut last_poll: i32 = 0;
        let mut producer_queue_full = false;

        loop {
            let mut is_idle = true;

            //current implement in mod.rs lead to arbitrary dropping of messages
            //in the flush() method, I try to fix it here ...
            //basically, it should be enough to make use of the ability of
            //crossbeam_channel to achieve effectly managing on buffer status,
            //so we can just stop receiving when the queue has fulled

            //first, always keep absorbing messages
            let scheme_full = message_scheme.is_full();
            if !scheme_full {
                let recv_ret = if last_poll == 0 {
                    receiver.try_recv()
                } else {
                    receiver.recv_timeout(timeout_interval).map_err(|err| match err {
                        RecvTimeoutError::Timeout => TryRecvError::Empty,
                        RecvTimeoutError::Disconnected => TryRecvError::Disconnected,
                    })
                };
                match recv_ret {
                    Ok((topic, message)) => {
                        is_idle &= false;
                        message_scheme.on_message(topic, message);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => {
                        log::info!("kafka producer disconnected");
                        return;
                    }
                };
            }
            //then try send out some messages...
            let pop_msg = if !producer_queue_full { message_scheme.pop_up() } else { None };
            if let Some(msg) = pop_msg {
                let send_ret = match producer.send(msg) {
                    Ok(_) => None,
                    Err((KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull), rec)) => {
                        //flag is clear when we had polled something
                        producer_queue_full = true;
                        log::warn!("kafka sender buffer is full");
                        Some(rec.delivery_opaque)
                    }
                    Err((err, rec)) => {
                        log::info!("kafka producer encounter error {}", err);
                        Some(rec.delivery_opaque)
                    }
                };
                message_scheme.commit(send_ret);
                is_idle &= false;
            }
            //finally, always poll
            let poll_dur = if scheme_full && last_poll == 0 {
                timeout_interval
            } else {
                Duration::from_millis(0)
            };
            last_poll = producer.poll(poll_dur);
            producer_queue_full = producer_queue_full && last_poll == 0;
            while let Ok((result, opaque)) = delivery_report.try_recv() {
                message_scheme.deliver_commit(result, opaque);
            }

            if is_idle {
                // never ever dead loop...
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}

pub const BALANCES_TOPIC: &str = "balances";
pub const DEPOSITS_TOPIC: &str = "deposits";
pub const INTERNALTX_TOPIC: &str = "internaltransfer";
pub const ORDERS_TOPIC: &str = "orders";
pub const TRADES_TOPIC: &str = "trades";
pub const UNIFY_TOPIC: &str = "unifyevents";
pub const USER_TOPIC: &str = "registeruser";
pub const WITHDRAWS_TOPIC: &str = "withdraws";

use std::collections::LinkedList;

#[derive(Default)]
pub struct SimpleMessageScheme {
    balances_list: LinkedList<String>,
    internaltxs_list: LinkedList<String>,
    orders_list: LinkedList<String>,
    trades_list: LinkedList<String>,
    users_list: LinkedList<String>,
    last_poped: Option<(&'static str, String)>,
}

impl MessageScheme for SimpleMessageScheme {
    type DeliverOpaque = ();
    type K = &'static str;
    type V = &'static str;

    fn settings() -> Vec<(Self::K, Self::V)> {
        vec![("queue.buffering.max.ms", "1")]
    }
    fn is_full(&self) -> bool {
        self.balances_list.len() >= 100
            || self.internaltxs_list.len() >= 100
            || self.orders_list.len() >= 100
            || self.trades_list.len() >= 100
            || self.users_list.len() >= 100
    }

    fn on_message(&mut self, title_tip: &'static str, message: String) {
        let list = match title_tip {
            BALANCES_TOPIC => &mut self.balances_list,
            INTERNALTX_TOPIC => &mut self.internaltxs_list,
            ORDERS_TOPIC => &mut self.orders_list,
            TRADES_TOPIC => &mut self.trades_list,
            USER_TOPIC => &mut self.users_list,
            _ => return,
        };

        list.push_back(message);
    }

    fn pop_up(&mut self) -> Option<BaseRecord<'_, str, str, Self::DeliverOpaque>> {
        //we select the list with most size (so message stream is never ordering)
        let mut len = self.balances_list.len();
        let mut list = &mut self.balances_list;
        let mut topic_name = BALANCES_TOPIC;

        let mut candi_list = [
            &mut self.internaltxs_list,
            &mut self.orders_list,
            &mut self.trades_list,
            &mut self.users_list,
        ];
        let iters = [INTERNALTX_TOPIC, ORDERS_TOPIC, TRADES_TOPIC, USER_TOPIC]
            .iter()
            .zip(&mut candi_list);

        for i in iters.into_iter() {
            let (tp_name, l) = i;
            if l.len() > len {
                len = l.len();
                list = *l;
                topic_name = tp_name;
            }
        }

        self.last_poped = list.pop_front().map(|str| (topic_name, str));

        self.last_poped.as_ref().map(|poped_ret| {
            let (topic_name, str) = poped_ret;
            BaseRecord::to(topic_name).key("").payload(AsRef::as_ref(str))
        })
    }

    fn commit(&mut self, isfailed: Option<Self::DeliverOpaque>) {
        if isfailed.is_some() {
            //push the poped message back
            let (topic_name, str) = self.last_poped.take().unwrap();
            self.on_message(topic_name, str);
        }
    }
    fn deliver_commit(&mut self, result: SimpleDeliverResult, _opaque: Self::DeliverOpaque) {
        if let Err(e) = result {
            log::error!("kafka send err: {}, MESSAGE LOST", e);
        }
    }
}

#[derive(Default)]
pub struct FullOrderMessageScheme {
    ordered_list: LinkedList<(&'static str, String)>,
    //two counters is used to assigned and verify for delivery
    deliver_cnt: u64,
    commited_cnt: u64,
}

impl MessageScheme for FullOrderMessageScheme {
    type DeliverOpaque = Box<u64>;
    type K = &'static str;
    type V = &'static str;

    fn settings() -> Vec<(Self::K, Self::V)> {
        //with these semantics the message written into kafka should be
        //strictly ordering as input
        vec![
            ("enable.idempotence", "true"),
            ("max.in.flight.requests.per.connection", "1"),
            //message being tried to send never timeout in ~24days and until 2^31 retries
            //if it stil failed the underlying connection must be investigated
            ("delivery.timeout.ms", "2147483647"),
        ]
    }
    fn is_full(&self) -> bool {
        self.ordered_list.len() >= 100
    }

    fn on_message(&mut self, title_tip: &'static str, message: String) {
        match title_tip {
            DEPOSITS_TOPIC | INTERNALTX_TOPIC | ORDERS_TOPIC | TRADES_TOPIC | USER_TOPIC | WITHDRAWS_TOPIC => {
                self.ordered_list.push_back((title_tip, message))
            }
            _ => {}
        };
    }

    fn pop_up(&mut self) -> Option<BaseRecord<'_, str, str, Self::DeliverOpaque>> {
        if self.ordered_list.is_empty() {
            return None;
        }
        let (title_tip, message) = self.ordered_list.front().unwrap();
        Some(
            BaseRecord::with_opaque_to(UNIFY_TOPIC, Box::new(self.deliver_cnt))
                .key(*title_tip)
                .payload(AsRef::as_ref(message)),
        )
    }

    fn commit(&mut self, isfailed: Option<Self::DeliverOpaque>) {
        if isfailed.is_none() {
            self.ordered_list.pop_front();
            self.deliver_cnt += 1;
        } else {
            //sanity check
            assert!(*isfailed.unwrap() == self.deliver_cnt);
        }
    }
    fn deliver_commit(&mut self, result: SimpleDeliverResult, opaque: Self::DeliverOpaque) {
        //sanity check: verify we are keeping order
        assert!(*opaque == self.commited_cnt);
        self.commited_cnt += 1;
        log::debug!("kafka unify messenger has confirm deliver till {}", self.commited_cnt);

        if let Err(e) = result {
            //TODO: should we panic ?
            log::error!("kafka send err: {}, MESSAGE LOST", e);
        }
    }
}
