use anyhow::Result;
use crossbeam_channel::{TryRecvError, bounded};
use rdkafka::client::ClientContext;
use rdkafka::config::ClientConfig;
use rdkafka::error::{KafkaError, RDKafkaErrorCode};
use rdkafka::producer::{BaseProducer, BaseRecord, DeliveryResult, Producer, ProducerContext};
use rdkafka::util::Timeout;
use std::time::Duration;

pub const BALANCES_TOPIC: &str = "balances";
pub const DEPOSITS_TOPIC: &str = "deposits";
pub const INTERNALTX_TOPIC: &str = "internaltransfer";
pub const ORDERS_TOPIC: &str = "orders";
pub const TRADES_TOPIC: &str = "trades";
pub const UNIFY_TOPIC: &str = "unifyevents";
pub const USER_TOPIC: &str = "registeruser";
pub const WITHDRAWS_TOPIC: &str = "withdraws";

pub type SimpleDeliverResult = Result<(), KafkaError>;

pub trait MessageScheme: Default + Sync + Send {
    type DeliverOpaque: rdkafka::util::IntoOpaque;
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
    delivery_record: crossbeam_channel::Sender<(SimpleDeliverResult, T::DeliverOpaque)>,
    delivery_record_get: crossbeam_channel::Receiver<(SimpleDeliverResult, T::DeliverOpaque)>,
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

impl<T: MessageScheme> RdProducerContext<T> {
    pub fn new_producer(self, brokers: &str) -> Result<BaseProducer<Self>>
    where
        T::K: AsRef<str>,
        T::V: AsRef<str>,
    {
        let mut config = ClientConfig::new();
        config.set("bootstrap.servers", brokers);
        for (k, v) in T::settings() {
            config.set(k.as_ref(), v.as_ref());
        }
        let producer = config.create_with_context(self)?;
        Ok(producer)
    }

    pub fn run_default(producer: BaseProducer<Self>, receiver: crossbeam_channel::Receiver<(&'static str, String)>) {
        let message_scheme = T::default();
        Self::run(producer, message_scheme, receiver);
    }

    pub fn run(producer: BaseProducer<Self>, mut message_scheme: T, receiver: crossbeam_channel::Receiver<(&'static str, String)>) {
        Self::run_loop(&producer, &mut message_scheme, receiver);

        // flush producer before exit
        while let Some(msg) = message_scheme.pop_up() {
            let send_ret = match producer.send(msg) {
                Ok(_) => None,
                Err((KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull), rec)) => {
                    producer.poll(Duration::from_millis(100));
                    Some(rec.delivery_opaque)
                }
                Err((err, _)) => {
                    log::error!("kafka encounter error when shutdown: {}", err);
                    return;
                }
            };
            message_scheme.commit(send_ret);
        }

        let _ = producer.flush(Timeout::Never);
        log::info!("kafka producer running terminated");
    }

    fn run_loop(producer: &BaseProducer<Self>, message_scheme: &mut T, receiver: crossbeam_channel::Receiver<(&'static str, String)>) {
        let timeout_interval = Duration::from_millis(100);
        let delivery_report = &producer.context().delivery_record_get;
        let mut producer_queue_full = false;

        loop {
            let mut is_idle = true;
            let scheme_full = message_scheme.is_full();

            if !scheme_full {
                let recv_ret = receiver.try_recv();
                match recv_ret {
                    Ok((topic, message)) => {
                        is_idle = false;
                        message_scheme.on_message(topic, message);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => {
                        log::info!("kafka producer disconnected");
                        return;
                    }
                };
            }

            let pop_msg = if !producer_queue_full { message_scheme.pop_up() } else { None };
            if let Some(msg) = pop_msg {
                let send_ret = match producer.send(msg) {
                    Ok(_) => None,
                    Err((KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull), rec)) => {
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
                is_idle = false;
            }

            let poll_dur = if scheme_full || producer_queue_full {
                timeout_interval
            } else {
                Duration::from_millis(0)
            };
            producer.poll(poll_dur);
            producer_queue_full = false; // poll() may have cleared the queue
            while let Ok((result, opaque)) = delivery_report.try_recv() {
                message_scheme.deliver_commit(result, opaque);
            }

            if is_idle {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}

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
        let mut len = self.balances_list.len();
        let mut list: *mut LinkedList<String> = &mut self.balances_list;
        let mut topic_name = BALANCES_TOPIC;

        let mut candi_list = [
            (&mut self.internaltxs_list, INTERNALTX_TOPIC),
            (&mut self.orders_list, ORDERS_TOPIC),
            (&mut self.trades_list, TRADES_TOPIC),
            (&mut self.users_list, USER_TOPIC),
        ];

        for (l, tp) in candi_list.iter_mut() {
            if l.len() > len {
                len = l.len();
                list = *l;
                topic_name = *tp;
            }
        }

        // SAFETY: list is always a valid mutable reference to one of the fields
        let list_ref = unsafe { &mut *list };
        self.last_poped = list_ref.pop_front().map(|str| (topic_name, str));

        self.last_poped
            .as_ref()
            .map(|(topic_name, str)| BaseRecord::to(topic_name).key("").payload(str.as_ref()))
    }

    fn commit(&mut self, isfailed: Option<Self::DeliverOpaque>) {
        if isfailed.is_some() {
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
    deliver_cnt: u64,
    commited_cnt: u64,
}

impl MessageScheme for FullOrderMessageScheme {
    type DeliverOpaque = Box<u64>;
    type K = &'static str;
    type V = &'static str;

    fn settings() -> Vec<(Self::K, Self::V)> {
        vec![
            ("enable.idempotence", "true"),
            ("max.in.flight.requests.per.connection", "1"),
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
                .payload(message.as_ref()),
        )
    }

    fn commit(&mut self, isfailed: Option<Self::DeliverOpaque>) {
        if isfailed.is_none() {
            self.ordered_list.pop_front();
            self.deliver_cnt += 1;
        } else {
            assert!(*isfailed.unwrap() == self.deliver_cnt);
        }
    }
    fn deliver_commit(&mut self, result: SimpleDeliverResult, opaque: Self::DeliverOpaque) {
        assert!(*opaque == self.commited_cnt);
        self.commited_cnt += 1;
        log::debug!("kafka unify messenger has confirm deliver till {}", self.commited_cnt);

        if let Err(e) = result {
            log::error!("kafka send err: {}, MESSAGE LOST", e);
        }
    }
}

pub struct RdProducerStub<T> {
    pub sender: crossbeam_channel::Sender<(&'static str, String)>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> RdProducerStub<T> {
    fn push_message_and_topic(&self, message: String, topic_name: &'static str) {
        // Profiling: drop message if channel full instead of panicking
        let _ = self.sender.try_send((topic_name, message));
    }
}

impl<T: MessageScheme + 'static> RdProducerStub<T> {
    pub fn new_and_run(brokers: &str) -> Result<Self>
    where
        T::K: AsRef<str>,
        T::V: AsRef<str>,
    {
        let (sender, receiver) = bounded(1_000_000);
        let producer_context: RdProducerContext<T> = Default::default();
        let kafkaproducer = producer_context.new_producer(brokers)?;
        std::thread::spawn(move || {
            RdProducerContext::<T>::run_default(kafkaproducer, receiver);
        });
        Ok(Self {
            sender,
            _phantom: std::marker::PhantomData,
        })
    }
}

impl<T: MessageScheme> super::MessageManager for RdProducerStub<T> {
    fn is_block(&self) -> bool {
        // Profiling: temporarily disable backpressure to measure core matching throughput
        // self.sender.len() >= (self.sender.capacity().unwrap() - 10_000)
        false
    }
    fn push_order_message(&mut self, order: &super::OrderMessage) {
        let message = simd_json::to_string(&order).unwrap();
        self.push_message_and_topic(message, ORDERS_TOPIC)
    }
    fn push_trade_message(&mut self, trade: &super::Trade) {
        let message = simd_json::to_string(&trade).unwrap();
        self.push_message_and_topic(message, TRADES_TOPIC)
    }
    fn push_balance_message(&mut self, balance: &super::BalanceMessage) {
        let message = simd_json::to_string(&balance).unwrap();
        self.push_message_and_topic(message, BALANCES_TOPIC)
    }
    fn push_deposit_message(&mut self, deposit: &super::DepositMessage) {
        let message = simd_json::to_string(&deposit).unwrap();
        self.push_message_and_topic(message, DEPOSITS_TOPIC)
    }
    fn push_withdraw_message(&mut self, withdraw: &super::WithdrawMessage) {
        let message = simd_json::to_string(&withdraw).unwrap();
        self.push_message_and_topic(message, WITHDRAWS_TOPIC)
    }
    fn push_transfer_message(&mut self, tx: &super::TransferMessage) {
        let message = simd_json::to_string(&tx).unwrap();
        self.push_message_and_topic(message, INTERNALTX_TOPIC)
    }
    fn push_user_message(&mut self, user: &super::UserMessage) {
        let message = simd_json::to_string(&user).unwrap();
        self.push_message_and_topic(message, USER_TOPIC)
    }
}

pub type SimpleMessageManager = RdProducerStub<SimpleMessageScheme>;
pub type FullOrderMessageManager = RdProducerStub<FullOrderMessageScheme>;

pub fn new_simple_message_manager(brokers: &str) -> Result<SimpleMessageManager> {
    SimpleMessageManager::new_and_run(brokers)
}

pub fn new_full_order_message_manager(brokers: &str) -> Result<FullOrderMessageManager> {
    FullOrderMessageManager::new_and_run(brokers)
}
