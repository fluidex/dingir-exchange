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
  const USDTBefore1 = await client.balanceQueryByAsset(userId1, "USDT");
  const ETHBefore1 = await client.balanceQueryByAsset(userId1, "ETH");
  const USDTBefore2 = await client.balanceQueryByAsset(userId2, "USDT");
  const ETHBefore2 = await client.balanceQueryByAsset(userId2, "ETH");
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
  const USDTAfter1 = await client.balanceQueryByAsset(userId1, "USDT");
  const ETHAfter1 = await client.balanceQueryByAsset(userId1, "ETH");
  const USDTAfter2 = await client.balanceQueryByAsset(userId2, "USDT");
  const ETHAfter2 = await client.balanceQueryByAsset(userId2, "ETH");
  /* TODO: Needs to validate the all balances for each asset.
  decimalEqual(USDTAfter.available, USDTBefore.available);
  decimalEqual(USDTAfter.frozen, USDTBefore.frozen);
  decimalEqual(USDTAfter.total, USDTBefore.total);
  */
  const tradeCountAfter = (await client.marketSummary(market)).trade_count;
  console.log(tradeCountBefore);
  console.log(tradeCountAfter);
  console.log("avg orders/s:", (parallel * repeat) / totalTime);
  console.log(
    "avg trades/s:",
    (tradeCountAfter - tradeCountBefore) / totalTime
  );
  console.log("stressTest done");
}

async function main() {
  try {
    await stressTest({ parallel: 120, interval: 100, repeat: 230 });
    // await stressTest({ parallel: 1, interval: 500, repeat: 0 });
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}
main();
