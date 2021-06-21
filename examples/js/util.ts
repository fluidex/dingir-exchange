import {
  userId,
  base,
  quote,
  market,
  fee,
  ORDER_SIDE_BID,
  ORDER_SIDE_ASK,
  ORDER_TYPE_MARKET,
  ORDER_TYPE_LIMIT,
  VERBOSE
} from "./config"; // dotenv
import { defaultClient as client } from "./client";

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
  const balances = await client.balanceQuery(userId);
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

export async function depositAssets(assets, userId) {
  for (const [asset, amount] of Object.entries(assets)) {
    console.log("deposit", amount, asset);
    await client.balanceUpdate(userId, asset, "deposit", depositId(), amount, {
      key: "value"
    });
  }
}

export async function putLimitOrder(userId, side, amount, price) {
  if (VERBOSE) {
    console.log("putLimitOrder", { userId, side, amount, price });
  }
  return await client.orderPut(
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

export function getRandomFloat(min, max) {
  return Math.random() * (max - min) + min;
}
export function getRandomFloatAround(value, ratio = 0.05, abs = 0) {
  const eps1 = getRandomFloat(-abs, abs);
  const eps2 = getRandomFloat(-value * ratio, value * ratio);
  return value + eps1 + eps2;
}
export function getRandomInt(min, max) {
  min = Math.ceil(min);
  max = Math.floor(max);
  return Math.floor(Math.random() * (max - min)) + min;
}
export function getRandomElem(arr) {
  return arr[Math.floor(Math.random() * arr.length)];
}
export async function putRandOrder(userId) {
  // TODO: market order?
  const side = [ORDER_SIDE_ASK, ORDER_SIDE_BID][getRandomInt(0, 10000) % 2];
  const price = getRandomFloat(1350, 1450);
  const amount = getRandomFloat(0.5, 1.5);
  const order = await putLimitOrder(userId, side, amount, price);
  //console.log("order put", order.id.toString(), { side, price, amount });
}

export function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}
