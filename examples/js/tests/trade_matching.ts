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
  const order = await client.createOrder(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "3", "2.0", fee, fee);
  (order as any).post_only = true;
  const bid = await client.client.OrderPut(order);

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

// 7. Market order: success with liquidity, fail without
async function testMarketOrder() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  // No liquidity: market order should fail
  // Note: disable_market_order may be true in config. If so, skip this test.
  try {
    const markets = await client.marketList();
    const marketInfo = markets.get(market);
    if (marketInfo && marketInfo.disable_market_order) {
      console.log("testMarketOrder skipped: market orders disabled");
      return;
    }
  } catch (e) {
    // ignore, try anyway
  }

  // With liquidity
  await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "5", "1.5", fee, fee);
  const bid = await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_MARKET, "5", "0", fee, fee);
  assertDecimalEqual(bid.remain, "0");

  // Without liquidity
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  // No asks in book
  await assert.rejects(async () => {
    await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_MARKET, "5", "0", fee, fee);
  }, /no counter orders/);

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
