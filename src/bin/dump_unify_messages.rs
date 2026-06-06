#![allow(dead_code)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_and_return)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::single_char_pattern)]

use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};

use dingir_exchange::{config, message};
use message::consumer::{SimpleConsumer, SimpleHandler};

use rdkafka::consumer::StreamConsumer;
use rdkafka::message::{BorrowedMessage, Message};

fn get_msg_tag_from_topic(t: &str) -> Option<&'static str> {
    Some(match t {
        "deposits" => "DepositMessage",
        "internaltransfer" => "TransferMessage",
        "orders" => "OrderMessage",
        "registeruser" => "UserMessage",
        "trades" => "TradeMessage",
        "withdraws" => "WithdrawMessage",
        _ => {
            println!("skip msg of type {}", t);
            return None;
        }
    })
}

struct MessageWriter {
    out_file: Mutex<File>,
}

fn main() {
    dotenv::dotenv().ok();
    let _guard = dingir_exchange::utils::tracing::setup();

    let settings = config::Settings::new();
    log::debug!("Settings: {:?}", settings);

    let rt: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    let writer = Arc::new(MessageWriter {
        out_file: Mutex::new(File::create("unify_msgs_output.txt").unwrap()),
    });

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
            let writer = writer.clone();
            let cr_main = SimpleConsumer::new(consumer.as_ref())
                .add_topic(
                    message::UNIFY_TOPIC,
                    SimpleHandler::new(move |msg: &BorrowedMessage<'_>| {
                        let mut file = writer.out_file.lock().unwrap();
                        let msg_key = std::str::from_utf8(msg.key().unwrap()).unwrap();
                        if let Some(msgtype) = get_msg_tag_from_topic(msg_key) {
                            let payloadmsg = std::str::from_utf8(msg.payload().unwrap()).unwrap();
                            file.write_fmt(format_args!("{{\"type\":\"{}\",\"value\":{}}}\n", msgtype, payloadmsg))
                                .unwrap();
                        }
                    }),
                )
                .unwrap();

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    log::info!("Ctrl-c received, shutting down");
                    break;
                },

                err = cr_main.run_stream() => {
                    log::error!("Kafka consumer error: {}", err);
                }
            }
        }
    })
}
