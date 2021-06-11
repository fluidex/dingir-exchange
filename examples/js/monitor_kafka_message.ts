import { KafkaConsumer } from "./kafka_client";
import * as Dotenv from "dotenv";

async function main() {
  Dotenv.config();
  const consumer = new KafkaConsumer().Init(true);
  await consumer;
}

main().catch(console.error);
