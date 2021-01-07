import { debugDump } from "./client.mjs";

async function main() {
  //    Dotenv.config()
  try {
    await debugDump();
  } catch (error) {
    console.error("Catched error:", error);
  }
}

main();
