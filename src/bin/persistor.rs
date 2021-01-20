#![allow(dead_code)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_and_return)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::single_char_pattern)]

use dingir_exchange::{config, types, database, message, models};
use database::{DatabaseWriter, DatabaseWriterConfig};
use types::ConnectionType;

use rdkafka::consumer::{StreamConsumer, DefaultConsumerContext, ConsumerContext, stream_consumer};

//use sqlx::Connection;
struct AppliedConsumer<C : ConsumerContext + 'static = DefaultConsumerContext> (stream_consumer::StreamConsumer<C>);

impl<C: ConsumerContext + 'static> message::consumer::RdConsumerExt for AppliedConsumer<C>
{
    type CTXType = stream_consumer::StreamConsumerContext<C>;
    type SelfType = stream_consumer::StreamConsumer<C>;
    fn to_self(&self) -> &Self::SelfType{&self.0}
}

use sqlx::migrate::Migrator;
use sqlx::Connection;
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations/ts"); 

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let mut conf = config_rs::Config::new();
    let config_file = dotenv::var("CONFIG_FILE").unwrap();
    conf.merge(config_rs::File::with_name(&config_file)).unwrap();
    let settings: config::Settings = conf.try_into().unwrap();
    log::debug!("Settings: {:?}", settings);    

    let mut rt: tokio::runtime::Runtime = tokio::runtime::Builder::new()
        .enable_all()
        .threaded_scheduler()
        .build()
        .expect("build runtime");

    let consumer : StreamConsumer = rdkafka::config::ClientConfig::new()
        .set("bootstrap.servers", &settings.brokers)
        .set("group.id", "kline_data_fetcher")
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .create().unwrap();
    let consumer = AppliedConsumer(consumer);

    rt.block_on(async move {

        MIGRATOR.run(&mut ConnectionType::connect(&settings.db_history).await.unwrap()).await.ok();

        let persistor : DatabaseWriter<models::TradeRecord> = 
            DatabaseWriter::new(&DatabaseWriterConfig {
                database_url: settings.db_history.clone(),
                run_daemon: true,
                inner_buffer_size: 8192,
            }).unwrap();
        
        loop {

            let cr_main = message::consumer::SimpleConsumer::new(&consumer)        
            .add_topic(message::TRADES_TOPIC, 
                message::persist::MsgDataPersistor::<models::TradeRecord, types::Trade>{
                    writer: &persistor,
                    phantom: std::marker::PhantomData,
                })
            .unwrap();

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    log::info!("Ctrl-c received, shutting down");
                    break;
                },
    
                err = cr_main.run_stream(|cr|cr.start()) => {
                    log::error!("Kafka consumer error: {}", err);
                }
            }
        }

        tokio::task::spawn_blocking(move || persistor.finish()).await.unwrap().unwrap();
    })

}
