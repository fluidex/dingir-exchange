import { defaultClient as client } from "./client";

async function main() {
  //    Dotenv.config()
  try {
    await client.debugDump();
  } catch (error) {
    console.error("Catched error:", error);
  }
}

main();
