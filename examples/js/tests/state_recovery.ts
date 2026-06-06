import { userId, base, quote, market, fee, ORDER_SIDE_BID, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT } from "../config";
import { defaultClient as client } from "../client";
import { getTestAccount } from "../accounts";
import { Account } from "../fluidex";
import { depositAssets } from "../exchange_helper";
import { assertDecimalEqual, sleep } from "../util";
import { strict as assert } from "assert";

const askUser = userId;
const bidUser = userId + 1;

async function initAccounts() {
  await client.connect();
  for (let uid of [askUser, bidUser]) {
    let acc = Account.fromMnemonic(getTestAccount(uid).mnemonic);
    client.addAccount(uid, acc);
    await client.client.RegisterUser({
      user_id: uid,
      l1_address: acc.ethAddr,
      l2_pubkey: acc.bjjPubKey,
    });
  }
}

async function setupBalances() {
  await depositAssets({ USDT: "1000.0", ETH: "500.0" }, askUser);
  await depositAssets({ USDT: "1000.0", ETH: "500.0" }, bidUser);
}

async function captureState() {
  const depth = await client.orderDepth(market, 100, "0");
  const summary = await client.marketSummary(market);
  const askBalanceEth = await client.balanceQueryByAsset(askUser, "ETH");
  const askBalanceUsdt = await client.balanceQueryByAsset(askUser, "USDT");
  const bidBalanceEth = await client.balanceQueryByAsset(bidUser, "ETH");
  const bidBalanceUsdt = await client.balanceQueryByAsset(bidUser, "USDT");

  // Get open orders
  let openOrders = [];
  try {
    const askOrders = await client.orderQuery(askUser, market);
    openOrders = openOrders.concat(askOrders.orders || []);
  } catch (e) {
    // ignore
  }
  try {
    const bidOrders = await client.orderQuery(bidUser, market);
    openOrders = openOrders.concat(bidOrders.orders || []);
  } catch (e) {
    // ignore
  }

  return {
    depth,
    summary,
    balances: {
      askEth: { available: askBalanceEth.available.toString(), frozen: askBalanceEth.frozen.toString() },
      askUsdt: { available: askBalanceUsdt.available.toString(), frozen: askBalanceUsdt.frozen.toString() },
      bidEth: { available: bidBalanceEth.available.toString(), frozen: bidBalanceEth.frozen.toString() },
      bidUsdt: { available: bidBalanceUsdt.available.toString(), frozen: bidBalanceUsdt.frozen.toString() },
    },
    openOrders: openOrders.map(o => ({ id: o.id, side: o.order_side, price: o.price, remain: o.remain })),
  };
}

function compareState(a, b) {
  // Compare depths
  assert.deepEqual(a.depth.asks, b.depth.asks, "Depth asks mismatch after recovery");
  assert.deepEqual(a.depth.bids, b.depth.bids, "Depth bids mismatch after recovery");

  // Compare balances
  assert.deepEqual(a.balances, b.balances, "Balances mismatch after recovery");

  // Compare open orders (by id and remain)
  type OrderInfo = { id: any; side: any; price: any; remain: any };
  const aOrders = new Map<string, OrderInfo>((a.openOrders as OrderInfo[]).map((o: OrderInfo) => [o.id, o]));
  const bOrders = new Map<string, OrderInfo>((b.openOrders as OrderInfo[]).map((o: OrderInfo) => [o.id, o]));
  assert.equal(aOrders.size, bOrders.size, `Order count mismatch: ${aOrders.size} vs ${bOrders.size}`);
  for (const [id, oa] of aOrders) {
    const ob = bOrders.get(id);
    assert(ob, `Order ${id} missing after recovery`);
    assertDecimalEqual(oa.remain, ob.remain);
    assert.equal(oa.side, ob.side);
    assertDecimalEqual(oa.price, ob.price);
  }
}

async function testDebugReset() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  // Create some state: partial fills + open orders
  await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "5", "2.0", fee, fee);
  await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "3", "2.5", fee, fee);
  await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "4", "2.0", fee, fee);

  const before = await captureState();

  // Reset and replay from operation log
  await client.debugReset();

  const after = await captureState();
  compareState(before, after);

  console.log("testDebugReset passed");
}

async function testDebugReload() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  // Create state
  await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "7", "3.0", fee, fee);
  await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "7", "3.0", fee, fee);

  const before = await captureState();

  // Dump snapshot to DB
  await client.debugDump();

  // Reload from snapshot
  await client.debugReload();

  const after = await captureState();
  compareState(before, after);

  console.log("testDebugReload passed");
}

async function main() {
  try {
    await testDebugReset();
    await testDebugReload();
    console.log("\n=== All state_recovery tests passed ===");
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}
main();
