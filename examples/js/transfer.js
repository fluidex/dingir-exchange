import { userId } from "./config.mjs"; // dotenv
import {
  balanceQuery,
  debugReset,
  transfer,
  registerUser
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

// Test no user_to
async function failureWithNoUser2() {
  const res = await transfer(userId, anotherUserId, "ETH", 50);

  assert.equal(res.success, false);
  assert.equal(res.asset, "ETH");
  decimalEqual(res.balance_from, "100");

  const balance1 = await balanceQuery(userId);
  decimalEqual(balance1.ETH.available, "100");
  const balance2 = await balanceQuery(anotherUserId);
  decimalEqual(balance2.ETH.available, "0");

  console.log("successTransferTest passed");
}

async function registerUsers() {
  await registerUser({ id: userId, l1_address: "l1_address_1", l2_pubkey: "l2_pubkey_1" });
  console.log("register user 1");
  await registerUser({ id: anotherUserId, l1_address: "l1_address_2", l2_pubkey: "l2_pubkey_2" });
  console.log("register user 2");
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

  console.log("failureWithZeroDeltaTest passed");
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

  console.log("failureWithInsufficientFromBalanceTest passed");
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

  console.log("successTransferTest passed");
}

async function simpleTest() {
  await setupAsset();
  await registerUsers();
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
