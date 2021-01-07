import { KafkaConsumer } from "./kafka_client.mjs";
import Dotenv from "dotenv";

async function main() {
  Dotenv.config();
  const consumer = new KafkaConsumer(true).Init();
  await consumer;
}

main().catch(console.error);
