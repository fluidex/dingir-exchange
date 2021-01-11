import {
  userId,
  base,
  quote,
  market,
  fee,
  ORDER_SIDE_BID,
  ORDER_SIDE_ASK,
  ORDER_TYPE_MARKET,
  ORDER_TYPE_LIMIT
} from "./config.mjs"; // dotenv
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
import { depositAssets, printBalance, sleep, floatEqual } from "./util.mjs";
import { KafkaConsumer } from "./kafka_client.mjs";

import Decimal from "decimal.js";
import { strict as assert } from "assert";
import whynoderun from "why-is-node-running";

async function infoList() {
  console.log(await assetList([]));
  console.log(await marketList([]));
}

async function setupAsset() {
  // check balance is zero
  const balance1 = await balanceQuery(userId);
  floatEqual(balance1.BTC.available, "0");
  floatEqual(balance1.BTC.frozen, "0");
  floatEqual(balance1.ETH.available, "0");
  floatEqual(balance1.ETH.frozen, "0");

  await depositAssets({ BTC: "100.0", ETH: "50.0" });

  // check deposit success
  const balance2 = await balanceQuery(userId);
  floatEqual(balance2.BTC.available, "100");
  floatEqual(balance2.BTC.frozen, "0");
  floatEqual(balance2.ETH.available, "50");
  floatEqual(balance2.ETH.frozen, "0");
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
  await setupAsset();
  await orderTest();
  return await tradeTest();
}

function checkMessages(messages) {
  // TODO: more careful check
  assert.equal(messages.orders.length, 5);
  assert.equal(messages.balances.length, 2);
  assert.equal(messages.trades.length, 1);
}

async function mainTest(withMQ) {
  await debugReset();

  let kafkaConsumer;
  if (withMQ) {
    kafkaConsumer = new KafkaConsumer();
    kafkaConsumer.Init();
  }
  const [askOrderId, bidOrderId] = await simpleTest();
  if (withMQ) {
    await sleep(3 * 1000);
    const messages = kafkaConsumer.GetAllMessages();
    console.log(messages);
    await kafkaConsumer.Stop();
    checkMessages(messages);
  }
}

async function main() {
  try {
    await mainTest(false);
  } catch (error) {
    console.error("Catched error:", error);
  }
}
main();
