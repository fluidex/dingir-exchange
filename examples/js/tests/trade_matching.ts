import { userId, base, quote, market, fee, ORDER_SIDE_BID, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, ORDER_TYPE_MARKET } from "../config";
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
  for (let uid = 1; uid <= bidUser; uid++) {
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

// 1. Full fill: taker size == maker size
async function testFullFill() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  const ask = await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "10", "1.5", fee, fee);
  const bid = await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "10", "1.5", fee, fee);

  // Ask fully filled and removed
  await assert.rejects(async () => {
    await client.orderDetail(market, ask.id);
  }, /invalid order_id/);

  // Bid fully filled and removed
  await assert.rejects(async () => {
    await client.orderDetail(market, bid.id);
  }, /invalid order_id/);

  // Balances after full fill at price 1.5, amount 10, fee=0
  // askUser: sold 10 ETH, received 15 USDT
  const bAsk = await client.balanceQueryByAsset(askUser, "ETH");
  assertDecimalEqual(bAsk.available, "490");
  assertDecimalEqual(bAsk.frozen, "0");
  const bAskUsdt = await client.balanceQueryByAsset(askUser, "USDT");
  assertDecimalEqual(bAskUsdt.available, "1015");

  // bidUser: bought 10 ETH, spent 15 USDT
  const bBid = await client.balanceQueryByAsset(bidUser, "ETH");
  assertDecimalEqual(bBid.available, "510");
  const bBidUsdt = await client.balanceQueryByAsset(bidUser, "USDT");
  assertDecimalEqual(bBidUsdt.available, "985");

  console.log("testFullFill passed");
}

// 2. Partial fill: taker larger than maker
async function testPartialFill() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  const ask = await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "5", "2.0", fee, fee);
  const bid = await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "12", "2.0", fee, fee);

  // Ask fully filled
  await assert.rejects(async () => {
    await client.orderDetail(market, ask.id);
  }, /invalid order_id/);

  // Bid partially filled, remains 7
  const bidPending = await client.orderDetail(market, bid.id);
  assertDecimalEqual(bidPending.remain, "7");

  // bidUser frozen: 7 * 2.0 = 14 USDT
  const bBidUsdt = await client.balanceQueryByAsset(bidUser, "USDT");
  assertDecimalEqual(bBidUsdt.frozen, "14");

  // Depth should show bid at 2.0, amount 7
  const depth = await client.orderDepth(market, 100, "0");
  assert.equal(depth.bids.length, 1);
  assertDecimalEqual(depth.bids[0].price, "2.0");
  assertDecimalEqual(depth.bids[0].amount, "7");
  assert.equal(depth.asks.length, 0);

  console.log("testPartialFill passed");
}

// 3. Multiple makers at different prices
async function testMultiMakerFill() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  // Two asks at different prices
  const ask1 = await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "3", "1.0", fee, fee);
  const ask2 = await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "5", "2.0", fee, fee);
  // Bid crosses both
  const bid = await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "8", "2.0", fee, fee);

  // ask1 fully filled
  await assert.rejects(async () => await client.orderDetail(market, ask1.id), /invalid order_id/);
  // ask2 fully filled
  await assert.rejects(async () => await client.orderDetail(market, ask2.id), /invalid order_id/);
  // bid fully filled (3+5=8)
  await assert.rejects(async () => await client.orderDetail(market, bid.id), /invalid order_id/);

  // bidUser bought 8 ETH, spent 3*1.0 + 5*2.0 = 13 USDT
  const bBidUsdt = await client.balanceQueryByAsset(bidUser, "USDT");
  assertDecimalEqual(bBidUsdt.available, "987"); // 1000 - 13

  console.log("testMultiMakerFill passed");
}

// 4. Price-time priority: earlier order at same price gets filled first
async function testPriceTimePriority() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  // Two asks at same price, different times
  const ask1 = await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "4", "1.5", fee, fee);
  const ask2 = await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "4", "1.5", fee, fee);
  // Bid only fills 4
  const bid = await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "4", "1.5", fee, fee);

  // ask1 (earlier) should be filled, ask2 should remain
  await assert.rejects(async () => await client.orderDetail(market, ask1.id), /invalid order_id/);
  const ask2Pending = await client.orderDetail(market, ask2.id);
  assertDecimalEqual(ask2Pending.remain, "4");

  console.log("testPriceTimePriority passed");
}

// 5. Self-trade prevention
async function testSelfTradePrevention() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  // Same user places ask and bid at crossing price
  const ask = await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "5", "1.5", fee, fee);

  // With disable_self_trade=true (default), this bid should be rejected/cancelled
  // The exact behavior: the bid is created but immediately cancelled because it would self-trade
  const bid = await client.orderPut(askUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "5", "1.5", fee, fee);

  // The ask should still be in the book (not filled)
  const askPending = await client.orderDetail(market, ask.id);
  assertDecimalEqual(askPending.remain, "5");

  // The bid should be finished (cancelled due to self-trade)
  // Note: orderDetail for a finished order may return invalid order_id or the finished state
  // depending on implementation. We verify via balance: no trade occurred.
  const bAsk = await client.balanceQueryByAsset(askUser, "ETH");
  // If self-trade happened, ETH would have changed. It should still be frozen 5.
  assertDecimalEqual(bAsk.frozen, "5");

  console.log("testSelfTradePrevention passed");
}

// 6. Post-only order cancelled if would match immediately
async function testPostOnly() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  // Place an ask in the book
  const ask = await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "5", "2.0", fee, fee);

  // Post-only bid at same price should be cancelled (not inserted)
  // Use raw gRPC call with empty signature to skip signature check
  const bid = await client.client.OrderPut({
    user_id: bidUser,
    market,
    order_side: ORDER_SIDE_BID,
    order_type: ORDER_TYPE_LIMIT,
    amount: "3",
    price: "2.0",
    taker_fee: fee,
    maker_fee: fee,
    post_only: true,
    signature: "",
  });

  // Ask should remain unchanged
  const askPending = await client.orderDetail(market, ask.id);
  assertDecimalEqual(askPending.remain, "5");

  // Bid should be finished (cancelled), not in book
  // Verify by checking depth: only the ask
  const depth = await client.orderDepth(market, 100, "0");
  assert.equal(depth.asks.length, 1);
  assert.equal(depth.bids.length, 0);

  console.log("testPostOnly passed");
}

// 7. Market order behavior (disabled by default, or fills immediately when enabled)
async function testMarketOrder() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  // With liquidity
  await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "5", "1.5", fee, fee);

  // Try market order. If disabled, expect "market orders disabled".
  // If enabled, it should fill immediately.
  let marketOrderFailed = false;
  let bid;
  try {
    bid = await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_MARKET, "5", "0", fee, fee);
  } catch (e: any) {
    if (e.message && e.message.includes("market orders disabled")) {
      marketOrderFailed = true;
    } else {
      throw e;
    }
  }

  if (!marketOrderFailed) {
    // Market orders are enabled - verify immediate fill
    assertDecimalEqual(bid.remain, "0");

    // Without liquidity
    await client.debugReset();
    await initAccounts();
    await setupBalances();

    await assert.rejects(async () => {
      await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_MARKET, "5", "0", fee, fee);
    }, /no counter orders/);
  }

  console.log("testMarketOrder passed");
}

async function main() {
  try {
    await testFullFill();
    await testPartialFill();
    await testMultiMakerFill();
    await testPriceTimePriority();
    await testSelfTradePrevention();
    await testPostOnly();
    await testMarketOrder();
    console.log("\n=== All trade_matching tests passed ===");
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}
main();
