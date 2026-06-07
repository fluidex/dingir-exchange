import { market, userId } from "./config"; // dotenv
import { defaultClient as client } from "./client";
import { Account } from "./fluidex";
import { getTestAccount } from "./accounts";

import { sleep, decimalAdd, assertDecimalEqual } from "./util";

import { depositAssets, printBalance, putRandOrder } from "./exchange_helper";

const CANCEL_EVERY_N_ROUNDS = 30;
const OPPONENT_USER_ID = userId + 1;

async function registerIfNeeded(uid: number) {
  try {
    const acc = Account.fromMnemonic(getTestAccount(uid).mnemonic);
    await client.client.RegisterUser({
      user_id: uid,
      l1_address: acc.ethAddr,
      l2_pubkey: acc.bjjPubKey,
    });
    client.addAccount(uid, acc);
    console.log("registered user", uid);
  } catch (e) {
    // may already exist
    console.log("register user", uid, e.message || "already exists");
  }
}

async function stressTest({ parallel, interval, durationMinutes }) {
  const durationMs = durationMinutes * 60 * 1000;

  // Warm up gRPC connection
  await client.connect();

  // Ensure both users exist
  await registerIfNeeded(userId);
  await registerIfNeeded(OPPONENT_USER_ID);

  const tradeCountBefore = (await client.marketSummary(market)).trade_count;
  console.log("cancel", tradeCountBefore, "trades");
  await client.orderCancelAll(userId, market);
  await client.orderCancelAll(OPPONENT_USER_ID, market);

  await depositAssets({ USDT: "10000000", ETH: "10000" }, userId);
  await depositAssets({ USDT: "10000000", ETH: "10000" }, OPPONENT_USER_ID);

  const balanceBefore = async (uid: number) => {
    const usdt = await client.balanceQueryByAsset(uid, "USDT");
    const eth = await client.balanceQueryByAsset(uid, "ETH");
    return {
      usdt: decimalAdd(usdt.available, usdt.frozen),
      eth: decimalAdd(eth.available, eth.frozen),
    };
  };

  const usdtBeforeUser = await balanceBefore(userId);
  const usdtBeforeOpp = await balanceBefore(OPPONENT_USER_ID);

  await printBalance();
  const startTime = Date.now();
  function elapsedSecs() {
    return (Date.now() - startTime) / 1000;
  }
  let count = 0;
  let errorCount = 0;
  let lastReportTime = startTime;

  for (;;) {
    let promises = [];
    for (let i = 0; i < parallel; i++) {
      // Alternate between user and opponent
      const uid = i % 2 === 0 ? userId : OPPONENT_USER_ID;
      promises.push(
        putRandOrder(uid, market).catch((e) => {
          errorCount++;
          if (errorCount <= 5) {
            console.error("order error:", e.message || e);
          }
        })
      );
    }
    await Promise.all(promises);
    if (interval > 0) {
      await sleep(interval);
    }
    count += 1;

    // Periodic cancel to avoid hitting user_order_num_limit
    if (count % CANCEL_EVERY_N_ROUNDS === 0) {
      console.log("periodic cancel at round", count);
      await client.orderCancelAll(userId, market);
      await client.orderCancelAll(OPPONENT_USER_ID, market);
    }

    const now = Date.now();
    const elapsed = now - startTime;
    if (elapsed >= durationMs) {
      break;
    }

    if (now - lastReportTime >= 30000) {
      const totalOrders = parallel * count;
      const avgOrdersPerSec = totalOrders / (elapsed / 1000);
      console.log(
        `[${(elapsed / 1000).toFixed(1)}s] avg orders/s: ${avgOrdersPerSec.toFixed(1)}, ` +
          `total orders: ${totalOrders}, errors: ${errorCount}`
      );
      lastReportTime = now;
    }
  }

  const totalTime = elapsedSecs();
  await printBalance();

  const usdtAfterUser = await balanceBefore(userId);
  const usdtAfterOpp = await balanceBefore(OPPONENT_USER_ID);

  console.log("\n=== Stress Test Summary ===");
  console.log("Duration:", totalTime.toFixed(1), "seconds");
  console.log("Total rounds:", count);
  console.log("Total orders:", parallel * count);
  console.log("Errors:", errorCount);
  console.log("Avg orders/s:", (parallel * count / totalTime).toFixed(1));

  const tradeCountAfter = (await client.marketSummary(market)).trade_count;
  console.log("Trades before:", tradeCountBefore);
  console.log("Trades after:", tradeCountAfter);
  console.log("Avg trades/s:", ((tradeCountAfter - tradeCountBefore) / totalTime).toFixed(1));

  try {
    // Check total asset conservation across all users
    const totalUsdtBefore = decimalAdd(usdtBeforeUser.usdt, usdtBeforeOpp.usdt);
    const totalEthBefore = decimalAdd(usdtBeforeUser.eth, usdtBeforeOpp.eth);
    const totalUsdtAfter = decimalAdd(usdtAfterUser.usdt, usdtAfterOpp.usdt);
    const totalEthAfter = decimalAdd(usdtAfterUser.eth, usdtAfterOpp.eth);

    assertDecimalEqual(totalUsdtAfter, totalUsdtBefore);
    assertDecimalEqual(totalEthAfter, totalEthBefore);
    console.log("Balance consistency: PASS (total USDT and ETH conserved)");
  } catch (e) {
    console.error("Balance consistency: FAIL", e.message);
    throw e;
  }
}

async function main() {
  try {
    await stressTest({ parallel: 50, interval: 100, durationMinutes: 10 });
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}
main();
