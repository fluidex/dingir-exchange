import { market, userId } from "./config"; // dotenv
import {
  balanceQuery,
  balanceQueryByAsset,
  orderPut,
  balanceUpdate,
  assetList,
  marketList,
  orderDetail,
  marketSummary,
  orderCancel,
  orderCancelAll,
  orderDepth,
  debugReset,
  debugReload,
} from "./client";

import {
  depositAssets,
  printBalance,
  putRandOrder,
  sleep,
  decimalAdd,
  decimalEqual
} from "./util";

async function stressTest({ parallel, interval, repeat }) {
  const tradeCountBefore = (await marketSummary(market)).trade_count;
  console.log("cancel", tradeCountBefore, "trades");
  console.log(await orderCancelAll(userId, market));
  await depositAssets({ USDT: "10000000", ETH: "10000" }, userId);
  const USDTBefore = await balanceQueryByAsset(userId, "USDT");
  const ETHBefore = await balanceQueryByAsset(userId, "ETH");
  await printBalance();
  const startTime = Date.now();
  function elapsedSecs() {
    return (Date.now() - startTime) / 1000;
  }
  let count = 0;
  // TODO: check balance before and after stress test
  // depends https://github.com/Fluidex/dingir-exchange/issues/30
  while (true) {
    let promises = [];
    for (let i = 0; i < parallel; i++) {
      promises.push(putRandOrder(userId));
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
    //await printBalance();
  }
  const totalTime = elapsedSecs();
  await printBalance();
  const USDTAfter = await balanceQueryByAsset(userId, "USDT");
  const ETHAfter = await balanceQueryByAsset(userId, "ETH");
  decimalEqual(USDTAfter, USDTBefore);
  decimalEqual(ETHAfter, ETHBefore);
  const tradeCountAfter = (await marketSummary(market)).trade_count;
  console.log("avg orders/s:", (parallel * repeat) / totalTime);
  console.log(
    "avg trades/s:",
    (tradeCountAfter - tradeCountBefore) / totalTime
  );
  console.log("stressTest done");
}

async function main() {
  try {
    await stressTest({ parallel: 50, interval: 500, repeat: 50 });
    //await stressTest({ parallel: 1, interval: 500, repeat: 0 });
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}
main();
