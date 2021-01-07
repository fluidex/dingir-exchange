import "./config.mjs"; // dotenv
import * as process from "process";
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
let depositId = Math.floor(Date.now() / 1000);
const base = "ETH";
const quote = "BTC";
const market = `${base}_${quote}`;
const fee = "0";

async function prettyPrint(obj) {
  console.dir(await obj, { depth: null });
}

function floatEqual(result, gt) {
  assert(new Decimal(result).equals(new Decimal(gt)), `${result} != ${gt}`);
}

async function printBalance(printList = ["BTC", "ETH"]) {
  const balances = await balanceQuery(userId);
  console.log("\nasset\tsum\tavaiable\tfrozen");
  for (const asset of printList) {
    const balance = balances[asset];
    console.log(
      asset,
      "\t",
      new Decimal(balance.available).add(new Decimal(balance.frozen)),
      "\t",
      balance.available,
      "\t",
      balance.frozen
    );
  }
  //console.log('\n');
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

async function depositAssets(assets) {
  for (const [asset, amount] of Object.entries(assets)) {
    console.log("deposit", amount, asset);
    await balanceUpdate(userId, asset, "deposit", depositId, amount, {
      key: "value"
    });
    depositId++;
  }
}

async function putLimitOrder(side, amount, price) {
  return await orderPut(
    userId,
    market,
    side,
    ORDER_TYPE_LIMIT,
    amount,
    price,
    fee,
    fee
  );
}

async function putRandOrder() {
  // TODO: market order?
  function getRandomArbitrary(min, max) {
    return Math.random() * (max - min) + min;
  }
  function getRandomInt(min, max) {
    min = Math.ceil(min);
    max = Math.floor(max);
    return Math.floor(Math.random() * (max - min)) + min;
  }
  const side = [ORDER_SIDE_ASK, ORDER_SIDE_BID][getRandomInt(0, 10000) % 2];
  const price = getRandomArbitrary(1, 50);
  const amount = getRandomArbitrary(1, 7);
  const order = await putLimitOrder(side, amount, price);
  console.log("order put", order.id.toString(), { side, price, amount });
}

async function stressTest({ parallel, interval, repeat }) {
  await depositAssets({ BTC: "100000", ETH: "50000" });

  await printBalance();
  const startTime = new Date();
  function elapsedSecs() {
    return (new Date() - startTime) / 1000;
  }
  let count = 0;
  // TODO: check balance before and after stress test
  // depends https://github.com/Fluidex/dingir-exchange/issues/30
  while (true) {
    let promises = [];
    for (let i = 0; i < parallel; i++) {
      promises.push(putRandOrder());
    }
    await Promise.all(promises);
    if (interval > 0) {
      await sleep(interval);
    }
    count += 1;
    console.log(
      "avg op/s:",
      (parallel * count) / elapsedSecs(),
      "orders",
      parallel * count,
      "secs",
      elapsedSecs()
    );
    if (repeat != 0 && count >= repeat) {
      break;
    }
    //await printBalance();
  }
  await printBalance();
  const endTime = new Date();
  console.log("avg op/s:", (parallel * repeat) / elapsedSecs());
  console.log("stressTest done");
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
  await depositAssets({ BTC: "100.0", ETH: "50.0" });
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

async function mainTest(withMQ) {
  // TODO: something seems to go wrong... after the `mainTest`, the db is empty??!!
  // https://github.com/Fluidex/dingir-exchange/issues/29
  if (process.platform != "darwin") {
    // just skip the wrong debugXXX to make system behavior reasonable...
    await debugReset();
    // await sleep(5000);
  }

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
    //await stressTest({ parallel: 100, interval: 1000, repeat: 100 });
    await mainTest(false);
    //await debugReload();
    //await testStatusAfterTrade(askOrderId, bidOrderId);
  } catch (error) {
    console.error("Catched error:", error);
  }
}
main();
