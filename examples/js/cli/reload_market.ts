import { defaultClient as client } from "../client";

async function main() {
  //    Dotenv.config()
  try {
    await client.reloadMarkets();
  } catch (error) {
    console.error("Caught error:", error);
  }
}

main();
