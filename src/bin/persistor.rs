#![allow(dead_code)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_and_return)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::single_char_pattern)]

use database::{DatabaseWriter, DatabaseWriterConfig};
use dingir_exchange::{config, database, message, models, types};
use types::{DbType};

use rdkafka::consumer::{StreamConsumer};

use message::persist::{MIGRATOR, TopicConfig};

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

    let consumer: StreamConsumer = rdkafka::config::ClientConfig::new()
        .set("bootstrap.servers", &settings.brokers)
        .set("group.id", &settings.consumer_group)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .create()
        .unwrap();

    rt.block_on(async move {
        let pool = sqlx::Pool::<DbType>::connect(&settings.db_history).await.unwrap();

        MIGRATOR
            .run(&pool)
            .await
            .ok();

        let persistor: DatabaseWriter<models::TradeRecord> = DatabaseWriter::new(&DatabaseWriterConfig {
            spawn_limit: 4,
            apply_benchmark: true,
            capability_limit: 8192,
        })
        .start_schedule(&pool)
        .unwrap();

        let trade_cfg = TopicConfig::<message::Trade>::new(message::TRADES_TOPIC)
            .persist_to(&persistor)
            .persist_to(&persistor);

        loop {
            let cr_main = message::consumer::SimpleConsumer::new(&consumer)
                .add_topic_config(&trade_cfg)
//                .add_topic(message::TRADES_TOPIC, MsgDataPersistor::new(&persistor).handle_message::<message::Trade>())
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

        persistor.finish().await.unwrap();
    })
}
