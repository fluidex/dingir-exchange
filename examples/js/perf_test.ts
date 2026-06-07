import { market, userId, ORDER_SIDE_ASK, ORDER_SIDE_BID, ORDER_TYPE_LIMIT } from "./config";
import { defaultClient as client } from "./client";
import { Account } from "./fluidex";
import { getTestAccount } from "./accounts";

import { sleep, decimalAdd, assertDecimalEqual, getRandomFloat, getRandomInt } from "./util";

const USER_COUNT = 50;
const PARALLEL_PER_USER = 1; // each user sends 1 req per round
const INTERVAL_MS = 100;
const CANCEL_EVERY_N_ROUNDS = 30;
const DURATION_MINUTES = 10;

const fee = "0";

interface UserState {
  id: number;
  activeOrders: Set<number>;
  totalOrders: number;
  errors: number;
}

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
    console.log("register user", uid, e.message || "already exists");
  }
}

async function putLimit(uid: number, market: string, side: string, amount: string, price: string) {
  return await client.orderPut(uid, market, side, ORDER_TYPE_LIMIT, amount, price, fee, fee);
}

function getRandomSide(): string {
  return Math.random() < 0.5 ? ORDER_SIDE_BID : ORDER_SIDE_ASK;
}

function getRandomPrice(): string {
  return getRandomFloat(1350, 1450).toFixed(2);
}

function getRandomAmount(): string {
  return getRandomFloat(0.1, 1.0).toFixed(4);
}

async function runMixedAction(user: UserState, market: string) {
  const roll = Math.random();
  try {
    if (roll < 0.7) {
      // 70% place order
      const side = getRandomSide();
      const price = getRandomPrice();
      const amount = getRandomAmount();
      const resp = await putLimit(user.id, market, side, amount, price);
      if (resp && resp.id) {
        user.activeOrders.add(Number(resp.id));
      }
      user.totalOrders++;
    } else if (roll < 0.9 && user.activeOrders.size > 0) {
      // 20% cancel a random order
      const ids = Array.from(user.activeOrders);
      const target = ids[getRandomInt(0, ids.length)];
      await client.orderCancel(user.id, market, target);
      user.activeOrders.delete(target);
    } else {
      // 10% query
      const q = Math.random();
      if (q < 0.5) {
        await client.balanceQueryByAsset(user.id, "USDT");
      } else {
        await client.orderQuery(user.id, market);
      }
    }
  } catch (e) {
    user.errors++;
    if (user.errors <= 3) {
      console.error("user", user.id, "error:", e.message || e);
    }
  }
}

async function perfTest() {
  const durationMs = DURATION_MINUTES * 60 * 1000;

  // Warm up gRPC connection
  await client.connect();

  const userIds = Array.from({ length: USER_COUNT }, (_, i) => userId + i);

  // Register and deposit all users
  for (const uid of userIds) {
    await registerIfNeeded(uid);
    await client.orderCancelAll(uid, market);
  }

  for (const uid of userIds) {
    await client.balanceUpdate(uid, "USDT", "deposit", Date.now() + uid, "10000000", {});
    await client.balanceUpdate(uid, "ETH", "deposit", Date.now() + uid + 1, "10000", {});
  }

  const balancesBefore = new Map<number, any>();
  for (const uid of userIds) {
    const usdt = await client.balanceQueryByAsset(uid, "USDT");
    const eth = await client.balanceQueryByAsset(uid, "ETH");
    balancesBefore.set(uid, {
      usdt: decimalAdd(usdt.available, usdt.frozen),
      eth: decimalAdd(eth.available, eth.frozen),
    });
  }

  const users: UserState[] = userIds.map((id) => ({
    id,
    activeOrders: new Set(),
    totalOrders: 0,
    errors: 0,
  }));

  const startTime = Date.now();
  let round = 0;
  let lastReportTime = startTime;

  for (;;) {
    const promises = users.map((u) => runMixedAction(u, market));
    await Promise.all(promises);
    await sleep(INTERVAL_MS);
    round++;

    if (round % CANCEL_EVERY_N_ROUNDS === 0) {
      console.log("periodic cancelAll at round", round);
      for (const u of users) {
        await client.orderCancelAll(u.id, market);
        u.activeOrders.clear();
      }
    }

    const now = Date.now();
    const elapsed = now - startTime;
    if (elapsed >= durationMs) {
      break;
    }

    if (now - lastReportTime >= 30000) {
      const totalOrders = users.reduce((s, u) => s + u.totalOrders, 0);
      const totalErrors = users.reduce((s, u) => s + u.errors, 0);
      const avgOrdersPerSec = totalOrders / (elapsed / 1000);
      console.log(
        `[${(elapsed / 1000).toFixed(1)}s] avg orders/s: ${avgOrdersPerSec.toFixed(1)}, ` +
          `total orders: ${totalOrders}, errors: ${totalErrors}, active users: ${USER_COUNT}`
      );
      lastReportTime = now;
    }
  }

  const totalTime = (Date.now() - startTime) / 1000;

  // Final cancelAll
  for (const u of users) {
    await client.orderCancelAll(u.id, market);
    u.activeOrders.clear();
  }
  await sleep(2000); // wait for balances to settle

  const totalOrders = users.reduce((s, u) => s + u.totalOrders, 0);
  const totalErrors = users.reduce((s, u) => s + u.errors, 0);

  console.log("\n=== Perf Test Summary ===");
  console.log("Duration:", totalTime.toFixed(1), "seconds");
  console.log("Total rounds:", round);
  console.log("Total orders:", totalOrders);
  console.log("Total errors:", totalErrors);
  console.log("Success rate:", ((totalOrders / (totalOrders + totalErrors || 1)) * 100).toFixed(2) + "%");
  console.log("Avg orders/s:", (totalOrders / totalTime).toFixed(1));

  const tradeCountBefore = 0; // we didn't track this precisely at start
  const tradeCountAfter = (await client.marketSummary(market)).trade_count;
  console.log("Total trades:", tradeCountAfter);
  console.log("Avg trades/s:", (tradeCountAfter / totalTime).toFixed(1));

  // Balance consistency check
  let balancePass = true;
  for (const uid of userIds) {
    const usdt = await client.balanceQueryByAsset(uid, "USDT");
    const eth = await client.balanceQueryByAsset(uid, "ETH");
    const after = {
      usdt: decimalAdd(usdt.available, usdt.frozen),
      eth: decimalAdd(eth.available, eth.frozen),
    };
    const before = balancesBefore.get(uid);
    try {
      assertDecimalEqual(after.usdt, before.usdt);
      assertDecimalEqual(after.eth, before.eth);
    } catch (e) {
      console.error("Balance mismatch for user", uid, ":", e.message);
      balancePass = false;
    }
  }
  console.log("Balance consistency:", balancePass ? "PASS" : "FAIL");
  if (!balancePass) {
    throw new Error("Balance consistency check failed");
  }
}

async function main() {
  try {
    await perfTest();
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}
main();
