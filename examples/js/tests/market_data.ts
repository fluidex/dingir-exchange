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
  await depositAssets({ USDT: "10000.0", ETH: "5000.0" }, askUser);
  await depositAssets({ USDT: "10000.0", ETH: "5000.0" }, bidUser);
}

// Generate trades at known prices for market data verification
async function generateTrades() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  const t0 = Math.floor(Date.now() / 1000);

  // Use unique high prices to avoid collision with data from earlier tests
  // Trade 1: price 500, amount 5
  await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "5", "500", fee, fee);
  await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "5", "500", fee, fee);

  await sleep(2000);

  // Trade 2: price 510, amount 3
  await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "3", "510", fee, fee);
  await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "3", "510", fee, fee);

  await sleep(2000);

  // Trade 3: price 505, amount 2
  await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "2", "505", fee, fee);
  await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "2", "505", fee, fee);

  const t1 = Math.floor(Date.now() / 1000);

  // Wait for persistor to write to DB
  await sleep(15000);

  return { t0, t1 };
}

async function testRecentTrades() {
  const { t0, t1 } = await generateTrades();

  const trades = await rest.recent_trades(market, 10);
  assert(trades.length >= 3, `Expected at least 3 trades, got ${trades.length}`);

  // Should be ordered by time desc and contain our unique prices
  const prices = trades.map(t => parseFloat(t.price));
  assert(prices.includes(500), `Expected price 500 in recent trades, got ${prices}`);
  assert(prices.includes(505), `Expected price 505 in recent trades, got ${prices}`);
  assert(prices.includes(510), `Expected price 510 in recent trades, got ${prices}`);

  console.log("testRecentTrades passed");
}

async function testOrderTrades() {
  await client.debugReset();
  await initAccounts();
  await setupBalances();

  const ask = await client.orderPut(askUser, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, "5", "100", fee, fee);
  const bid = await client.orderPut(bidUser, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT, "5", "100", fee, fee);

  await sleep(10000);

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

  // High/low may include data from earlier tests (DB is not reset).
  // Just verify our trades contributed positively.
  assert(ticker.high >= ticker.low, `Ticker high ${ticker.high} < low ${ticker.low}`);
  assert(ticker.volume >= 10, `Ticker volume ${ticker.volume} too low`);

  console.log("testTicker passed");
}

async function testKline() {
  const { t0, t1 } = await generateTrades();

  // Query 1-minute K-line covering the trades
  const kline = await rest.kline_history(market, 1, t0 - 60, t1 + 60);
  assert.equal(kline.s, "ok", `K-line status should be ok, got ${kline.s}`);
  assert(kline.t.length > 0, "K-line should have at least one candle");

  // Verify OHLCV: candles have positive volume and sensible bounds
  for (let i = 0; i < kline.t.length; i++) {
    assert(kline.h[i] >= kline.l[i], `K-line high ${kline.h[i]} < low ${kline.l[i]}`);
    assert(kline.v[i] > 0, `K-line volume should be > 0`);
  }

  // Last candle's close should be > 0 (we generated trades)
  const lastIdx = kline.c.length - 1;
  assert(kline.c[lastIdx] > 0, `K-line close ${kline.c[lastIdx]} should be > 0`);

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

  await sleep(10000);

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
