//deposit a lot to engine, so we would not encounter "balance not enough" failure

import { depositAssets } from "../exchange_helper";

async function main() {
  //if I really had so much money ....
  await depositAssets({ USDT: "10000000.0", ETH: "50000.0" }, 3);
  await depositAssets({ USDT: "10000.0", ETH: "50.0" }, 11);
}

main().catch(console.log);
