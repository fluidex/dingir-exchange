import { market } from "./config"; // dotenv
import { defaultClient as client } from "./client";

const userId1 = 21;
const userId2 = 22;

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
  console.log(await client.orderCancelAll(userId1, market));
  console.log(await client.orderCancelAll(userId2, market));
  await depositAssets({ USDT: "10000000", ETH: "10000" }, userId1);
  await depositAssets({ USDT: "10000000", ETH: "10000" }, userId2);
  /*
  const USDTBefore = await client.balanceQueryByAsset(userId, "USDT");
  const ETHBefore = await client.balanceQueryByAsset(userId, "ETH");
  */
  await printBalance();
  const startTime = Date.now();
  function elapsedSecs() {
    return (Date.now() - startTime) / 1000;
  }
  let count = 0;
  for (;;) {
    let promises = [];
    for (let i = 0; i < parallel; i++) {
      promises.push(putRandOrder(userId1, market));
      promises.push(putRandOrder(userId2, market));
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
  /*
  const USDTAfter = await client.balanceQueryByAsset(userId, "USDT");
  const ETHAfter = await client.balanceQueryByAsset(userId, "ETH");
  decimalEqual(USDTAfter.available, USDTBefore.available);
  decimalEqual(USDTAfter.frozen, USDTBefore.frozen);
  decimalEqual(USDTAfter.total, USDTBefore.total);
  */
  const result = await client.marketSummary(market);
  console.log(result);
  const tradeCountAfter = (await client.marketSummary(market)).trade_count;
  console.log(tradeCountAfter);
  console.log(tradeCountBefore);
  console.log("avg orders/s:", (parallel * repeat) / totalTime);
  console.log(
    "avg trades/s:",
    (tradeCountAfter - tradeCountBefore) / totalTime
  );
  console.log("stressTest done");
}

async function main() {
  try {
    await stressTest({ parallel: 150, interval: 100, repeat: 150 });
    // await stressTest({ parallel: 1, interval: 500, repeat: 0 });
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}
main();
