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
import { KafkaConsumer } from "./kafka_client.mjs";

import Decimal from "decimal.js";
import { strict as assert } from "assert";

import whynoderun from "why-is-node-running";

import { inspect } from "util";
inspect.defaultOptions.depth = null;

const ORDER_SIDE_ASK = 0;
const ORDER_SIDE_BID = 1;
const ORDER_TYPE_LIMIT = 0;
const ORDER_TYPE_MARKET = 1;

const userId = 3;
const depositId = Math.floor(Date.now() / 1000);
const base = "ETH";
const quote = "BTC";
const market = `${base}/${quote}`;
const fee = "0";

async function prettyPrint(obj) {
  console.dir(await obj, { depth: null });
}

function floatEqual(result, gt) {
  assert(new Decimal(result).equals(new Decimal(gt)), `${result} != ${gt}`);
}

async function ensureAssetValid() {
  const balance2 = await balanceQuery(userId);
  floatEqual(balance2.BTC.available, "100");
  floatEqual(balance2.BTC.frozen, "0");
  floatEqual(balance2.ETH.available, "50");
  floatEqual(balance2.ETH.frozen, "0");
}

async function ensureAssetZero() {
  const balance1 = await balanceQuery(userId);
  floatEqual(balance1.BTC.available, "0");
  floatEqual(balance1.BTC.frozen, "0");
  floatEqual(balance1.ETH.available, "0");
  floatEqual(balance1.ETH.frozen, "0");
}

async function setupAsset() {
  await balanceUpdate(userId, "BTC", "deposit", depositId, "100.0", {
    key: "value"
  });
  await balanceUpdate(userId, "ETH", "deposit", depositId + 1, "50.0", {
    key: "value"
  });
}

// Test order put and cancel
async function orderTest() {
  const order = await orderPut(
    userId,
    market,
    ORDER_SIDE_BID,
    ORDER_TYPE_LIMIT,
    /*amount*/ "10",
    /*price*/ "1.1",
    fee,
    fee
  );
  console.log(order);
  const balance3 = await balanceQuery(userId);
  floatEqual(balance3.BTC.available, "89");
  floatEqual(balance3.BTC.frozen, "11");

  const orderPending = await orderDetail(market, order.id);
  assert.deepEqual(orderPending, order);

  const summary = (await marketSummary(market))[0];
  floatEqual(summary.bid_amount, "10");
  assert.equal(summary.bid_count, 1);

  const depth = await orderDepth(market, 100, /*not merge*/ "0");
  assert.deepEqual(depth, { asks: [], bids: [{ price: "1.1", amount: "10" }] });

  await orderCancel(userId, market, 1);
  const balance4 = await balanceQuery(userId);
  floatEqual(balance4.BTC.available, "100");
  floatEqual(balance4.BTC.frozen, "0");

  console.log("orderTest passed");
}

async function info_list() {
  console.log(await assetList([]));
  console.log(await marketList([]));
}

// Test order trading
async function tradeTest() {
  const askOrder = await orderPut(
    userId,
    market,
    ORDER_SIDE_ASK,
    ORDER_TYPE_LIMIT,
    /*amount*/ "4",
    /*price*/ "1.1",
    fee,
    fee
  );
  const bidOrder = await orderPut(
    userId,
    market,
    ORDER_SIDE_BID,
    ORDER_TYPE_LIMIT,
    /*amount*/ "10",
    /*price*/ "1.1",
    fee,
    fee
  );
  console.log("ask order id", askOrder.id);
  console.log("bid order id", bidOrder.id);
  await testStatusAfterTrade(askOrder.id, bidOrder.id);

  console.log("tradeTest passed!");
  return [askOrder.id, bidOrder.id];
}

async function testStatusAfterTrade(askOrderId, bidOrderId) {
  const bidOrderPending = await orderDetail(market, bidOrderId);
  floatEqual(bidOrderPending.remain, "6");

  // Now, the `askOrder` will be matched and traded
  // So it will not be kept by the match engine
  await assert.rejects(async () => {
    const askOrderPending = await orderDetail(market, askOrderId);
    console.log(askOrderPending);
  }, /invalid order_id/);

  // should check trade price is 1.1 rather than 1.0 here.
  const summary = (await marketSummary(market))[0];
  floatEqual(summary.bid_amount, "6");
  assert.equal(summary.bid_count, 1);

  const depth = await orderDepth(market, 100, /*not merge*/ "0");
  //assert.deepEqual(depth, { asks: [], bids: [{ price: "1.1", amount: "6" }] });
  //assert.deepEqual(depth, { asks: [], bids: [{ price: "1.1", amount: "6" }] });
  const balance1 = await balanceQuery(userId);
  floatEqual(balance1.BTC.available, "93.4");
  floatEqual(balance1.BTC.frozen, "6.6");
  floatEqual(balance1.ETH.available, "50");
  floatEqual(balance1.ETH.frozen, "0");
}

async function simpleTest() {
  await ensureAssetZero();
  await setupAsset();
  await ensureAssetValid();
  await orderTest();
  return await tradeTest();
}

async function naiveExample() {
  console.log(await assetList());
  console.log(await balanceQuery(1));
  console.log(await balanceUpdate(1, "BTC", "deposit", depositId, 15, {}));
  console.log(await balanceQuery(1));
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function checkMessages(messages) {
  // TODO: more careful check
  assert.equal(messages.orders.length, 5);
  assert.equal(messages.balances.length, 2);
  assert.equal(messages.trades.length, 1);
}

async function main() {
  try {
    await debugReset();
    const kafkaConsumer = new KafkaConsumer();
    kafkaConsumer.Init();
    const [askOrderId, bidOrderId] = await simpleTest();
    await sleep(3 * 1000);
    const messages = kafkaConsumer.GetAllMessages();
    console.log(messages);
    checkMessages(messages);
    await kafkaConsumer.Stop();
    //await debugReload();
    //await testStatusAfterTrade(askOrderId, bidOrderId);
  } catch (error) {
    console.error("Catched error:", error);
  }
}
main();
