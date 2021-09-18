import { defaultClient as client } from "../client";

async function main() {
  //    Dotenv.config()
  try {
    await client.debugReset();
  } catch (error) {
    console.error("Catched error:", error);
  }
}

main();
