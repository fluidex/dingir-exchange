import * as Kafka from "kafkajs";

export class KafkaConsumer {
  verbose: boolean;
  consumer: any;
  messages: Map<string, Array<any>>;
  async Init(verbose = false, topics = ["balances", "trades", "orders", "unifyevents"]) {
    this.verbose = verbose;
    const brokers = process.env.KAFKA_BROKERS;
    const kafka = new Kafka.Kafka({
      brokers: (brokers || "127.0.0.1:9092").split(","),
      logLevel: Kafka.logLevel.WARN,
    });
    const consumer = kafka.consumer({ groupId: "test-group" });
    this.consumer = consumer;
    await consumer.connect();
    const fromBeginning = false;
    this.messages = new Map();
    for (const topic of topics) {
      this.messages.set(topic, []);
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
            value: message.value.toString(),
          });
        }
        this.messages.get(topic).push(message.value.toString());
      },
    });
  }
  Reset() {
    this.messages = new Map();
  }
  GetAllMessages(): Map<string, Array<any>> {
    return this.messages;
  }
  async Stop() {
    await this.consumer.disconnect();
  }
}
