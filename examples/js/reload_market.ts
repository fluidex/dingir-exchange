import { reloadMarkets } from "./client";

async function main() {
  //    Dotenv.config()
  try {
    await reloadMarkets();
  } catch (error) {
    console.error("Catched error:", error);
  }
}

main();
