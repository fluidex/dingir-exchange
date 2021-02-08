//deposit a lot to engine, so we would not encounter "balance not enough" failure

import { depositAssets } from "./util.mjs";

async function main() {
  //if I really had so much money ....
  await depositAssets({ USDT: "10000000.0", ETH: "50000.0" });
}

main().catch(console.log);
