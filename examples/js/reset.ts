import { debugReset } from "./client";

async function main() {
  //    Dotenv.config()
  try {
    await debugReset();
  } catch (error) {
    console.error("Catched error:", error);
  }
}

main();
