import { debugDump } from "./client";

async function main() {
  //    Dotenv.config()
  try {
    await debugDump();
  } catch (error) {
    console.error("Catched error:", error);
  }
}

main();
