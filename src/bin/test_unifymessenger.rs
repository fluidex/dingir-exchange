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
            "registeruser" => "UserMessage",
            _ => unreachable!(),
        };

        let payloadmsg = std::str::from_utf8(msg.payload().unwrap()).unwrap();
        file.write_fmt(format_args!("{{\"type\":\"{}\",\"value\":{}}}\n", msgtype, payloadmsg))
            .unwrap();
    }
}

fn main() {
    dotenv::dotenv().ok();

    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(non_blocking)
        .init();

    let settings = config::Settings::new();
    log::debug!("Settings: {:?}", settings);

    let rt: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    let writer = MessageWriter {
        out_file: Mutex::new(File::create("unify_msgs_output.txt").unwrap()),
    };

    rt.block_on(async move {
        let consumer: StreamConsumer = rdkafka::config::ClientConfig::new()
            .set("bootstrap.servers", &settings.brokers)
            .set("group.id", "unify_msg_dumper")
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
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
