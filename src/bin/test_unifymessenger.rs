#![allow(dead_code)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_and_return)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::single_char_pattern)]

use std::fs::File;
use std::io::Write;
use std::sync::Mutex;

use dingir_exchange::{config, message};
use message::consumer::{Simple, SimpleConsumer, SimpleMessageHandler};

use rdkafka::consumer::StreamConsumer;
use rdkafka::message::{BorrowedMessage, Message};

struct MessageWriter {
    out_file: Mutex<File>,
}

impl SimpleMessageHandler for &MessageWriter {
    fn on_message(&self, msg: &BorrowedMessage<'_>) {
        let mut file = self.out_file.lock().unwrap();

        let msgtype = match std::str::from_utf8(msg.key().unwrap()).unwrap() {
            "orders" => "OrderMessage",
            "trades" => "TradeMessage",
            "balances" => "BalanceMessage",
            _ => unreachable!(),
        };

        let payloadmsg = std::str::from_utf8(msg.payload().unwrap()).unwrap();
        file.write_fmt(format_args!("{{\"type\":\"{}\",\"value\":{}}}\n", msgtype, payloadmsg))
            .unwrap();
    }
}

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let mut conf = config_rs::Config::new();
    let config_file = dotenv::var("CONFIG_FILE").unwrap();
    conf.merge(config_rs::File::with_name(&config_file)).unwrap();
    let settings: config::Settings = conf.try_into().unwrap();
    log::debug!("Settings: {:?}", settings);

    let rt: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    let writer = MessageWriter {
        out_file: Mutex::new(File::create("output.txt").unwrap()),
    };

    rt.block_on(async move {
        let consumer: StreamConsumer = rdkafka::config::ClientConfig::new()
            .set("bootstrap.servers", &settings.brokers)
            .set("group.id", &settings.consumer_group)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .create()
            .unwrap();

        let consumer = std::sync::Arc::new(consumer);

        loop {
            let cr_main = SimpleConsumer::new(consumer.as_ref())
                .add_topic(message::UNIFY_TOPIC, Simple::from(&writer))
                .unwrap();

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    log::info!("Ctrl-c received, shutting down");
                    break;
                },

                err = cr_main.run_stream(|cr|cr.stream()) => {
                    log::error!("Kafka consumer error: {}", err);
                }
            }
        }
    })
}
