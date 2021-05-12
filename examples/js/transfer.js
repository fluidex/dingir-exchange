import { userId } from "./config.mjs"; // dotenv
import {
  balanceQuery,
  debugReset,
  transfer
} from "./client.mjs";
import { depositAssets, decimalEqual } from "./util.mjs";

import { strict as assert } from "assert";

const anotherUserId = userId + 1;

async function setupAsset() {
  await depositAssets({ ETH: "100.0" });

  const balance1 = await balanceQuery(userId);
  decimalEqual(balance1.ETH.available, "100");
  const balance2 = await balanceQuery(anotherUserId);
  decimalEqual(balance2.ETH.available, "0");
}

// Test failure with argument delta of value zero
async function failureWithZeroDeltaTest() {
  const res = await transfer(userId, anotherUserId, "ETH", 0);

  assert.equal(res.success, false);
  assert.equal(res.asset, "ETH");
  decimalEqual(res.balance_from, "100");

  const balance1 = await balanceQuery(userId);
  decimalEqual(balance1.ETH.available, "100");
  const balance2 = await balanceQuery(anotherUserId);
  decimalEqual(balance2.ETH.available, "0");
}

// Test failure with insufficient balance of from user
async function failureWithInsufficientFromBalanceTest() {
  const res = await transfer(userId, anotherUserId, "ETH", 101);

  assert.equal(res.success, false);
  assert.equal(res.asset, "ETH");
  decimalEqual(res.balance_from, "100");

  const balance1 = await balanceQuery(userId);
  decimalEqual(balance1.ETH.available, "100");
  const balance2 = await balanceQuery(anotherUserId);
  decimalEqual(balance2.ETH.available, "0");
}

// Test success transfer
async function successTransferTest() {
  const res = await transfer(userId, anotherUserId, "ETH", 50);

  assert.equal(res.success, true);
  assert.equal(res.asset, "ETH");
  decimalEqual(res.balance_from, "50");

  const balance1 = await balanceQuery(userId);
  decimalEqual(balance1.ETH.available, "50");
  const balance2 = await balanceQuery(anotherUserId);
  decimalEqual(balance2.ETH.available, "50");
}

async function simpleTest() {
  await setupAsset();
  await failureWithZeroDeltaTest();
  await failureWithInsufficientFromBalanceTest();
  await successTransferTest();
}

async function mainTest() {
  await debugReset();
  await simpleTest();
}

async function main() {
  try {
    await mainTest();
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}
main();
