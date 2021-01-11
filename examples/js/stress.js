import { market } from "./config.mjs"; // dotenv
import {
  balanceQuery,
  orderPut,
  balanceUpdate,
  assetList,
  marketList,
  orderDetail,
  marketSummary,
  orderCancel,
  orderDepth,
  debugReset,
  debugReload
} from "./client.mjs";

import { depositAssets, printBalance, putRandOrder, sleep } from "./util.mjs";

async function stressTest({ parallel, interval, repeat }) {
  const tradeCountBefore = (await marketSummary()).find(
    item => item.name == market
  ).trade_count;
  await depositAssets({ BTC: "1000000", ETH: "500000" });

  await printBalance();
  const startTime = new Date();
  function elapsedSecs() {
    return (new Date() - startTime) / 1000;
  }
  let count = 0;
  // TODO: check balance before and after stress test
  // depends https://github.com/Fluidex/dingir-exchange/issues/30
  while (true) {
    let promises = [];
    for (let i = 0; i < parallel; i++) {
      promises.push(putRandOrder());
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
  const tradeCountAfter = (await marketSummary()).find(
    item => item.name == market
  ).trade_count;
  console.log("avg orders/s:", (parallel * repeat) / totalTime);
  console.log(
    "avg trades/s:",
    (tradeCountAfter - tradeCountBefore) / totalTime
  );
  console.log("stressTest done");
}

async function main() {
  try {
    await stressTest({ parallel: 100, interval: 100, repeat: 500 });
  } catch (error) {
    console.error("Catched error:", error);
  }
}
main();
