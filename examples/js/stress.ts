import { market, userId } from "./config"; // dotenv
import { defaultClient as client } from "./client";

import {
  depositAssets,
  printBalance,
  putRandOrder,
  sleep,
  decimalAdd,
  decimalEqual
} from "./util";

async function stressTest({ parallel, interval, repeat }) {
  const tradeCountBefore = (await client.marketSummary(market)).trade_count;
  console.log("cancel", tradeCountBefore, "trades");
  console.log(await client.orderCancelAll(userId, market));
  await depositAssets({ USDT: "10000000", ETH: "10000" }, userId);
  const USDTBefore = await client.balanceQueryByAsset(userId, "USDT");
  const ETHBefore = await client.balanceQueryByAsset(userId, "ETH");
  await printBalance();
  const startTime = Date.now();
  function elapsedSecs() {
    return (Date.now() - startTime) / 1000;
  }
  let count = 0;
  for (;;) {
    let promises = [];
    for (let i = 0; i < parallel; i++) {
      promises.push(putRandOrder(userId, market));
    }
    await Promise.all(promises);
    if (interval > 0) {
      await sleep(interval);
    }
    count += 1;
    console.log(
      "avg orders/s:",
      (parallel * count) / elapsedSecs(),
      "orders",
      parallel * count,
      "secs",
      elapsedSecs()
    );
    if (repeat != 0 && count >= repeat) {
      break;
    }
  }
  const totalTime = elapsedSecs();
  await printBalance();
  const USDTAfter = await client.balanceQueryByAsset(userId, "USDT");
  const ETHAfter = await client.balanceQueryByAsset(userId, "ETH");
  decimalEqual(USDTAfter, USDTBefore);
  decimalEqual(ETHAfter, ETHBefore);
  const tradeCountAfter = (await client.marketSummary(market)).trade_count;
  console.log("avg orders/s:", (parallel * repeat) / totalTime);
  console.log(
    "avg trades/s:",
    (tradeCountAfter - tradeCountBefore) / totalTime
  );
  console.log("stressTest done");
}

async function main() {
  try {
    await stressTest({ parallel: 500, interval: 100, repeat: 100 });
    // await stressTest({ parallel: 1, interval: 500, repeat: 0 });
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}
main();
