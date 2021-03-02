import { reloadMarkets } from "./client.mjs";

async function main() {
  //    Dotenv.config()
  try {
    await reloadMarkets();
  } catch (error) {
    console.error("Catched error:", error);
  }
}

main();
