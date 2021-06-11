import Kafka from "kafkajs";

export class KafkaConsumer {
  async Init(
    verbose = false,
    topics = ["balances", "trades", "orders", "unifyevents"]
  ) {
    this.verbose = verbose;
    const brokers = process.env.KAFKA_BROKERS;
    const kafka = new Kafka.Kafka({
      brokers: (brokers || "127.0.0.1:9092").split(","),
      logLevel: Kafka.logLevel.WARN
    });
    const consumer = kafka.consumer({ groupId: "test-group" });
    this.consumer = consumer;
    await consumer.connect();
    const fromBeginning = false;
    for (const topic of topics) {
      this[topic] = [];
      await consumer.subscribe({ topic, fromBeginning });
    }
    return consumer.run({
      eachMessage: async ({ topic, partition, message }) => {
        if (this.verbose) {
          console.log("New message:", {
            topic,
            partition,
            offset: message.offset,
            key: message.key.toString(),
            value: message.value.toString()
          });
        }
        this[topic].push(message.value.toString());
      }
    });
  }
  Reset() {
    this.orders = [];
    this.balances = [];
    this.trades = [];
  }
  GetAllMessages() {
    return {
      orders: this.orders,
      balances: this.balances,
      trades: this.trades
    };
  }
  async Stop() {
    await this.consumer.disconnect();
  }
}
