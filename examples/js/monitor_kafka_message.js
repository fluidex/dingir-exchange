import Kafka from "kafkajs";
console.log(Kafka);
const kafka = new Kafka.Kafka({
  brokers: ["127.0.0.1:9092"]
});
const consumer = kafka.consumer({ groupId: "test-group" });
const run = async () => {
  // Consuming
  await consumer.connect();
  await consumer.subscribe({ topic: "balances", fromBeginning: true });
  await consumer.subscribe({ topic: "trades", fromBeginning: true });
  await consumer.subscribe({ topic: "orders", fromBeginning: true });

  await consumer.run({
    eachMessage: async ({ topic, partition, message }) => {
      console.log("New message:", {
        topic,
        partition,
        offset: message.offset,
        value: message.value.toString()
      });
    }
  });
};

run().catch(console.error);
