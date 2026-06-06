import { userId, base, quote, market, fee, ORDER_SIDE_BID, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT } from "../config";
import { defaultClient as client } from "../client";
import { defaultRESTClient as rest } from "../RESTClient";
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
  await depositAssets({ USDT: "10000.0", ETH: "5000.0" }, askUser);
  await depositAssets({ USDT: "10000.0", ETH: "5000.0" }, bidUser);
}

// Generate trades at known prices for market data verification
async function generateTrades() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  const t0 = Math.floor(Date.now() / 1000);

  // Trade 1: price 100, amount 5
  await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "5", "100", fee, fee);
  await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "5", "100", fee, fee);

  await sleep(2000);

  // Trade 2: price 110, amount 3
  await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "3", "110", fee, fee);
  await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "3", "110", fee, fee);

  await sleep(2000);

  // Trade 3: price 105, amount 2
  await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "2", "105", fee, fee);
  await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "2", "105", fee, fee);

  const t1 = Math.floor(Date.now() / 1000);

  // Wait for persistor to write to DB
  await sleep(8000);

  return { t0, t1 };
}

async function testRecentTrades() {
  const { t0, t1 } = await generateTrades();

  const trades = await rest.recent_trades(market, 10);
  assert(trades.length >= 3, `Expected at least 3 trades, got ${trades.length}`);

  // Should be ordered by time desc
  const prices = trades.map(t => parseFloat(t.price));
  assert(prices.includes(100));
  assert(prices.includes(105));
  assert(prices.includes(110));

  console.log("testRecentTrades passed");
}

async function testOrderTrades() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  const ask = await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "5", "100", fee, fee);
  const bid = await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "5", "100", fee, fee);

  await sleep(5000);

  // Query trades for ask order
  const askTrades = await rest.order_trades(market, ask.id);
  assert.equal(askTrades.trades.length, 1);
  assertDecimalEqual(askTrades.trades[0].price, "100");
  assertDecimalEqual(askTrades.trades[0].amount, "5");

  // Query trades for bid order
  const bidTrades = await rest.order_trades(market, bid.id);
  assert.equal(bidTrades.trades.length, 1);
  assertDecimalEqual(bidTrades.trades[0].price, "100");

  console.log("testOrderTrades passed");
}

async function testTicker() {
  const { t0, t1 } = await generateTrades();

  // Query ticker with a large interval to capture all trades
  const ticker = await rest.ticker("1h", market);
  assert(ticker.last > 0, "Ticker last should be > 0");
  assert.equal(ticker.market, market);

  // High should be max of 100, 110, 105 = 110
  // Low should be min = 100
  assert(ticker.high >= 100 && ticker.high <= 110, `Ticker high ${ticker.high} out of range`);
  assert(ticker.low >= 100 && ticker.low <= 110, `Ticker low ${ticker.low} out of range`);
  assert(ticker.volume >= 10, `Ticker volume ${ticker.volume} too low`);

  console.log("testTicker passed");
}

async function testKline() {
  const { t0, t1 } = await generateTrades();

  // Query 1-minute K-line covering the trades
  const kline = await rest.kline_history(market, 1, t0 - 60, t1 + 60);
  assert.equal(kline.s, "ok", `K-line status should be ok, got ${kline.s}`);
  assert(kline.t.length > 0, "K-line should have at least one candle");

  // Verify OHLCV values are within expected range
  // open should be around 100, close around 105, high <= 110, low >= 100
  for (let i = 0; i < kline.t.length; i++) {
    assert(kline.h[i] <= 115, `K-line high ${kline.h[i]} too high`);
    assert(kline.l[i] >= 95, `K-line low ${kline.l[i]} too low`);
    assert(kline.v[i] > 0, `K-line volume should be > 0`);
  }

  // The first candle's open should be near 100, last candle's close near 105
  assert(kline.o[0] >= 95 && kline.o[0] <= 115, `K-line open ${kline.o[0]} out of range`);
  const lastIdx = kline.c.length - 1;
  assert(kline.c[lastIdx] >= 95 && kline.c[lastIdx] <= 115, `K-line close ${kline.c[lastIdx]} out of range`);

  console.log("testKline passed");
}

async function testClosedOrdersPagination() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  // Create 5 trades (10 orders, all closed)
  for (let i = 0; i < 5; i++) {
    await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "1", (100 + i).toString(), fee, fee);
    await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "1", (100 + i).toString(), fee, fee);
  }

  await sleep(5000);

  const page1 = await rest.closed_orders(market, askUser, 2, 0);
  assert.equal(page1.orders.length, 2);
  assert(page1.total >= 5, `Expected total >= 5, got ${page1.total}`);

  const page2 = await rest.closed_orders(market, askUser, 2, 2);
  assert.equal(page2.orders.length, 2);

  console.log("testClosedOrdersPagination passed");
}

async function main() {
  try {
    await testRecentTrades();
    await testOrderTrades();
    await testTicker();
    await testKline();
    await testClosedOrdersPagination();
    console.log("\n=== All market_data tests passed ===");
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}
main();
