#![allow(dead_code)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_and_return)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::single_char_pattern)]

use database::{DatabaseWriter, DatabaseWriterConfig};
use dingir_exchange::{config, database, message, models, types};
use types::DbType;

use rdkafka::consumer::StreamConsumer;

use message::persist::{self, TopicHandlerBuilder, MIGRATOR};

fn main() {
    dotenv::dotenv().ok();
    let _guard = dingir_exchange::utils::tracing::setup();

    let settings = config::Settings::new();
    log::debug!("Settings: {:?}", settings);

    let rt: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async move {
        let consumer: StreamConsumer = rdkafka::config::ClientConfig::new()
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

        MIGRATOR.run(&pool).await.ok();

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

        let (trade_cfg, trade_commit) = TopicHandlerBuilder::<message::Trade>::new(message::TRADES_TOPIC)
            .persist_to(&persistor_kline)
            .persist_transformed::<models::UserTrade, persist::AskTrade>(&persistor_trade)
            .persist_transformed::<models::UserTrade, persist::BidTrade>(&persistor_trade)
            .build();

        let (order_cfg, order_commit) = TopicHandlerBuilder::<message::OrderMessage>::new(message::ORDERS_TOPIC)
            .persist_transformed::<models::OrderHistory, persist::ClosedOrder>(&persistor_order)
            .build();

        let (balance_cfg, balance_commit) = TopicHandlerBuilder::<message::BalanceMessage>::new(message::BALANCES_TOPIC)
            .persist_to(&persistor_balance)
            .build();

        let (internaltx_cfg, internaltx_commit) = TopicHandlerBuilder::<message::TransferMessage>::new(message::INTERNALTX_TOPIC)
            .persist_to(&persistor_transfer)
            .build();

        let (user_cfg, user_commit) = TopicHandlerBuilder::<message::UserMessage>::new(message::USER_TOPIC)
            .persist_to(&persistor_user)
            .build();

        let auto_commit = vec![
            trade_commit.auto_commit_start(consumer.clone()),
            order_commit.auto_commit_start(consumer.clone()),
            balance_commit.auto_commit_start(consumer.clone()),
            internaltx_commit.auto_commit_start(consumer.clone()),
            user_commit.auto_commit_start(consumer.clone()),
        ];
        let consumer = consumer.as_ref();

        loop {
            let cr_main = message::consumer::SimpleConsumer::new(consumer)
                .add_topic_config(&trade_cfg).unwrap()
                .add_topic_config(&order_cfg).unwrap()
                .add_topic_config(&balance_cfg).unwrap()
                .add_topic_config(&internaltx_cfg).unwrap()
                .add_topic_config(&user_cfg).unwrap();

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    log::info!("Ctrl-c received, shutting down");
                    for ac in auto_commit {
                        ac.interrut_and_commit(consumer).await;
                    }
                    break;
                },

                err = cr_main.run_stream() => {
                    log::error!("Kafka consumer error: {}", err);
                }
            }
        }
    })
}
