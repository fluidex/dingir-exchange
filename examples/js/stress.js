import { market, userId } from "./config.mjs"; // dotenv
import {
  balanceQuery,
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
  debugReload
} from "./client.mjs";

import {
  depositAssets,
  printBalance,
  putRandOrder,
  sleep,
  decimalAdd,
  decimalEqual
} from "./util.mjs";

async function stressTest({ parallel, interval, repeat }) {
  const tradeCountBefore = (await marketSummary()).find(
    item => item.name == market
  ).trade_count;
  console.log(await orderCancelAll(userId, market));
  await depositAssets({ USDT: "10000000", ETH: "10000" });
  const balancesBefore = await balanceQuery(userId);
  const USDTBefore = decimalAdd(
    balancesBefore.USDT.available,
    balancesBefore.USDT.frozen
  );
  const ETHBefore = decimalAdd(
    balancesBefore.USDT.available,
    balancesBefore.USDT.frozen
  );
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
  const balancesAfter = await balanceQuery(userId);
  const USDTAfter = decimalAdd(
    balancesAfter.USDT.available,
    balancesAfter.USDT.frozen
  );
  const ETHAfter = decimalAdd(
    balancesAfter.USDT.available,
    balancesAfter.USDT.frozen
  );
  decimalEqual(USDTAfter, USDTBefore);
  decimalEqual(ETHAfter, ETHBefore);
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
    await stressTest({ parallel: 50, interval: 500, repeat: 50 });
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}
main();
