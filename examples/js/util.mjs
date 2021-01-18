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

import Decimal from "decimal.js";
import { strict as assert } from "assert";

function depositId() {
  return Date.now();
}

export function decimalEqual(result, gt) {
  assert(new Decimal(result).equals(new Decimal(gt)), `${result} != ${gt}`);
}

export function decimalAdd(a, b) {
  return new Decimal(a).add(new Decimal(b));
}

export async function printBalance(printList = ["USDT", "ETH"]) {
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

export async function depositAssets(assets) {
  for (const [asset, amount] of Object.entries(assets)) {
    console.log("deposit", amount, asset);
    await balanceUpdate(userId, asset, "deposit", depositId(), amount, {
      key: "value"
    });
  }
}

export async function putLimitOrder(side, amount, price) {
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

export async function putRandOrder() {
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
  const price = getRandomArbitrary(200, 1200);
  const amount = getRandomArbitrary(1, 5);
  const order = await putLimitOrder(side, amount, price);
  //console.log("order put", order.id.toString(), { side, price, amount });
}

export function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}
