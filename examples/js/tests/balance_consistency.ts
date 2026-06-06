import { userId, base, quote, market, fee, ORDER_SIDE_BID, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT } from "../config";
import { defaultClient as client } from "../client";
import { getTestAccount } from "../accounts";
import { Account } from "../fluidex";
import { depositAssets } from "../exchange_helper";
import { assertDecimalEqual, decimalAdd, sleep } from "../util";
import { strict as assert } from "assert";
import Decimal from "decimal.js";

const askUser = userId;
const bidUser = userId + 1;
const transferUser = userId + 2;

async function initAccounts() {
  await client.connect();
  for (let uid of [askUser, bidUser, transferUser]) {
    let acc = Account.fromMnemonic(getTestAccount(uid).mnemonic);
    client.addAccount(uid, acc);
    await client.client.RegisterUser({
      user_id: uid,
      l1_address: acc.ethAddr,
      l2_pubkey: acc.bjjPubKey,
    });
  }
}

async function getTotalBalance(uid: number, asset: string): Promise<Decimal> {
  const b = await client.balanceQueryByAsset(uid, asset);
  return new Decimal(b.available.toString()).add(new Decimal(b.frozen.toString()));
}

async function checkInvariant(uid: number, assets: string[]) {
  for (const asset of assets) {
    const b = await client.balanceQueryByAsset(uid, asset);
    const total = new Decimal(b.available.toString()).add(new Decimal(b.frozen.toString()));
    // Just verify it's non-negative and consistent
    assert(total.gte(0), `Negative total balance for user ${uid} asset ${asset}: ${total}`);
  }
}

// Test: available + frozen invariant holds through order/trade/cancel/transfer
async function testBalanceInvariant() {
  await client.debugReset();
  await initAccounts();

  // Deposit
  await depositAssets({ USDT: "1000.0", ETH: "500.0" }, askUser);
  await depositAssets({ USDT: "1000.0", ETH: "500.0" }, bidUser);
  await depositAssets({ ETH: "100.0" }, transferUser);

  const askTotalBefore = await getTotalBalance(askUser, "ETH");
  const bidTotalBefore = await getTotalBalance(bidUser, "USDT");

  // Place order: freeze
  await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "10", "2.0", fee, fee);
  await checkInvariant(askUser, ["ETH", "USDT"]);
  await checkInvariant(bidUser, ["ETH", "USDT"]);

  // Place matching order: trade
  await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "10", "2.0", fee, fee);
  await checkInvariant(askUser, ["ETH", "USDT"]);
  await checkInvariant(bidUser, ["ETH", "USDT"]);

  // Verify totals changed correctly (fee=0)
  const askTotalAfter = await getTotalBalance(askUser, "ETH");
  const bidTotalAfter = await getTotalBalance(bidUser, "USDT");
  assertDecimalEqual(askTotalBefore.sub(askTotalAfter), "10"); // sold 10 ETH
  assertDecimalEqual(bidTotalBefore.sub(bidTotalAfter), "20"); // spent 20 USDT

  // Cancel remaining orders
  await client.orderCancelAll(askUser, market);
  await client.orderCancelAll(bidUser, market);
  await checkInvariant(askUser, ["ETH", "USDT"]);
  await checkInvariant(bidUser, ["ETH", "USDT"]);

  // Transfer
  const transferTotalBefore = await getTotalBalance(transferUser, "ETH");
  const askTotalBeforeTransfer = await getTotalBalance(askUser, "ETH");

  await client.transfer(transferUser, askUser, "ETH", 10);
  await checkInvariant(transferUser, ["ETH"]);
  await checkInvariant(askUser, ["ETH"]);

  const transferTotalAfter = await getTotalBalance(transferUser, "ETH");
  const askTotalAfterTransfer = await getTotalBalance(askUser, "ETH");
  assertDecimalEqual(transferTotalBefore.sub(transferTotalAfter), "10");
  assertDecimalEqual(askTotalAfterTransfer.sub(askTotalBeforeTransfer), "10");

  console.log("testBalanceInvariant passed");
}

// Test: precision and min_amount enforcement
async function testPrecisionAndLimits() {
  await client.debugReset();
  await initAccounts();
  await depositAssets({ USDT: "1000.0", ETH: "500.0" }, askUser);

  // Invalid amount precision (ETH has amount_prec, likely 4)
  await assert.rejects(async () => {
    await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "1.12345678", "1.0", fee, fee);
  });

  // Invalid price precision
  await assert.rejects(async () => {
    await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "1.0", "1.12345678", fee, fee);
  });

  console.log("testPrecisionAndLimits passed");
}

// Test: user order num limit
async function testOrderNumLimit() {
  await client.debugReset();
  await initAccounts();
  await depositAssets({ USDT: "1000000.0" }, askUser);

  // Put many small orders
  let count = 0;
  try {
    for (let i = 0; i < 2100; i++) {
      await client.orderPut(askUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "0.01", "0.01", fee, fee);
      count++;
    }
  } catch (e) {
    // Expected to fail when reaching limit
  }

  // Should have hit the limit (default 2000)
  assert(count >= 1990 && count <= 2005, `Expected to hit order limit around 2000, got ${count}`);

  // Cancel all
  await client.orderCancelAll(askUser, market);
  console.log("testOrderNumLimit passed");
}

async function main() {
  try {
    await testBalanceInvariant();
    await testPrecisionAndLimits();
    await testOrderNumLimit();
    console.log("\n=== All balance_consistency tests passed ===");
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}
main();
