import Kafka from "kafkajs";

export class KafkaConsumer {
  async Init(verbose = false) {
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
    await consumer.subscribe({ topic: "balances", fromBeginning });
    await consumer.subscribe({ topic: "trades", fromBeginning });
    await consumer.subscribe({ topic: "orders", fromBeginning });
    this.balances = [];
    this.trades = [];
    this.orders = [];
    return consumer.run({
      eachMessage: async ({ topic, partition, message }) => {
        if (this.verbose) {
          console.log("New message:", {
            topic,
            partition,
            offset: message.offset,
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
