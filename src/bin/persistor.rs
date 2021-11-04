#![allow(dead_code)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_and_return)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::single_char_pattern)]

use database::{DatabaseWriter, DatabaseWriterConfig};
use dingir_exchange::{config, database, message, models, types};
use fluidex_common::non_blocking_tracing;
use std::pin::Pin;
use types::DbType;

use fluidex_common::rdkafka::consumer::StreamConsumer;

use message::persist::{self, TopicConfig};

fn main() {
    dotenv::dotenv().ok();
    let _guard = non_blocking_tracing::setup();

    let settings = config::Settings::new();
    log::debug!("Settings: {:?}", settings);

    let rt: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async move {
        let consumer: StreamConsumer = fluidex_common::rdkafka::config::ClientConfig::new()
            .set("bootstrap.servers", &settings.brokers)
            .set("group.id", &settings.consumer_group)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create()
            .unwrap();

        let consumer = std::sync::Arc::new(consumer);

        let pool = sqlx::Pool::<DbType>::connect(&settings.db_history).await.unwrap();

        // migrate using `dingir_exchange::persist::MIGRATOR` with '/migrations' for db_history (state_changes and kline)
        dingir_exchange::persist::MIGRATOR.run(&pool).await.ok();
        // migrate using `message::persist::MIGRATOR` with '/migrations/ts' for kline additionally
        message::persist::MIGRATOR.run(&pool).await.ok();

        let write_config = DatabaseWriterConfig {
            spawn_limit: 4,
            apply_benchmark: true,
            capability_limit: 8192,
        };

        let persistor_kline: DatabaseWriter<models::MarketTrade> = DatabaseWriter::new(&write_config).start_schedule(&pool).unwrap();

        //following is equal to writers in history.rs
        let persistor_trade: DatabaseWriter<models::UserTrade> = DatabaseWriter::new(&write_config).start_schedule(&pool).unwrap();

        let persistor_order: DatabaseWriter<models::OrderHistory> = DatabaseWriter::new(&write_config).start_schedule(&pool).unwrap();

        let persistor_balance: DatabaseWriter<models::BalanceHistory> = DatabaseWriter::new(&write_config).start_schedule(&pool).unwrap();

        let persistor_transfer: DatabaseWriter<models::InternalTx> = DatabaseWriter::new(&write_config).start_schedule(&pool).unwrap();

        let persistor_user: DatabaseWriter<models::AccountDesc> = DatabaseWriter::new(&write_config).start_schedule(&pool).unwrap();

        let trade_cfg = TopicConfig::<message::Trade>::new(message::TRADES_TOPIC)
            .persist_to(&persistor_kline)
            .persist_to(&persistor_trade)
            .with_tr::<persist::AskTrade>()
            .persist_to(&persistor_trade)
            .with_tr::<persist::BidTrade>();

        let order_cfg = TopicConfig::<message::OrderMessage>::new(message::ORDERS_TOPIC)
            .persist_to(&persistor_order)
            .with_tr::<persist::ClosedOrder>();

        let balance_cfg = TopicConfig::<message::BalanceMessage>::new(message::BALANCES_TOPIC).persist_to(&persistor_balance);

        let internaltx_cfg = TopicConfig::<message::TransferMessage>::new(message::INTERNALTX_TOPIC).persist_to(&persistor_transfer);

        let user_cfg = TopicConfig::<message::UserMessage>::new(message::USER_TOPIC).persist_to(&persistor_user);

        let auto_commit = vec![
            trade_cfg.auto_commit_start(consumer.clone()),
            order_cfg.auto_commit_start(consumer.clone()),
            balance_cfg.auto_commit_start(consumer.clone()),
            internaltx_cfg.auto_commit_start(consumer.clone()),
            user_cfg.auto_commit_start(consumer.clone()),
        ];
        let consumer = consumer.as_ref();

        loop {
            let cr_main = message::consumer::SimpleConsumer::new(consumer)
                .add_topic_config(&trade_cfg).unwrap()
                .add_topic_config(&order_cfg).unwrap()
                .add_topic_config(&balance_cfg).unwrap()
                .add_topic_config(&internaltx_cfg).unwrap()
                .add_topic_config(&user_cfg).unwrap()
//                .add_topic(message::TRADES_TOPIC, MsgDataPersistor::new(&persistor).handle_message::<message::Trade>())
                ;

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

        tokio::try_join!(
            persistor_kline.finish(),
            persistor_trade.finish(),
            persistor_order.finish(),
            persistor_balance.finish(),
            persistor_transfer.finish(),
            persistor_user.finish(),
        )
        .expect("all persistor should success finish");
        let final_commits: Vec<Pin<Box<dyn std::future::Future<Output = ()> + Send>>> = auto_commit
            .into_iter()
            .map(|ac| -> Pin<Box<dyn std::future::Future<Output = ()> + Send>> { Box::pin(ac.final_commit(consumer)) })
            .collect();
        futures::future::join_all(final_commits).await;
        //auto_commit.final_commit(consumer).await;
    })
}
