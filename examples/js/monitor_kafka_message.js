import { KafkaConsumer } from "./kafka_client.mjs";

async function main() {
  const consumer = new KafkaConsumer(true).Init();
  await consumer;
}

main().catch(console.error);
