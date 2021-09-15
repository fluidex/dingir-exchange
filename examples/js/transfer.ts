import { userId } from "./config"; // dotenv
import { defaultClient as client } from "./client";
import { defaultRESTClient as rest_client } from "./RESTClient";
import { depositAssets, decimalEqual, sleep } from "./util";

import { strict as assert } from "assert";

const anotherUserId = userId + 10;

async function setupAsset() {
  await depositAssets({ ETH: "100.0" }, userId);

  const balance1 = await client.balanceQueryByAsset(userId, "ETH");
  decimalEqual(balance1.available, "100");
  const balance2 = await client.balanceQueryByAsset(anotherUserId, "ETH");
  decimalEqual(balance2.available, "0");
}

async function registerUsers() {
  for (var i = 1; i <= anotherUserId; i++) {
    await client.registerUser({
      id: i,
      l1_address: "l1_address_" + i,
      l2_pubkey: "l2_pubkey_" + i,
    });
    console.log("register user", i);
  }
}

// Test failure with argument delta of value zero
async function failureWithZeroDeltaTest() {
  const res = await client.transfer(userId, anotherUserId, "ETH", 0);

  assert.equal(res.success, false);
  assert.equal(res.asset, "ETH");
  decimalEqual(res.balance_from, "100");

  const balance1 = await client.balanceQueryByAsset(userId, "ETH");
  decimalEqual(balance1.available, "100");
  const balance2 = await client.balanceQueryByAsset(anotherUserId, "ETH");
  decimalEqual(balance2.available, "0");

  console.log("failureWithZeroDeltaTest passed");
}

// Test failure with insufficient balance of from user
async function failureWithInsufficientFromBalanceTest() {
  const res = await client.transfer(userId, anotherUserId, "ETH", 101);

  assert.equal(res.success, false);
  assert.equal(res.asset, "ETH");
  decimalEqual(res.balance_from, "100");

  const balance1 = await client.balanceQueryByAsset(userId, "ETH");
  decimalEqual(balance1.available, "100");
  const balance2 = await client.balanceQueryByAsset(anotherUserId, "ETH");
  decimalEqual(balance2.available, "0");

  console.log("failureWithInsufficientFromBalanceTest passed");
}

// Test success transfer
async function successTransferTest() {
  const res = await client.transfer(userId, anotherUserId, "ETH", 50);

  assert.equal(res.success, true);
  assert.equal(res.asset, "ETH");
  decimalEqual(res.balance_from, "50");

  const balance1 = await client.balanceQueryByAsset(userId, "ETH");
  decimalEqual(balance1.available, "50");
  const balance2 = await client.balanceQueryByAsset(anotherUserId, "ETH");
  decimalEqual(balance2.available, "50");

  console.log("successTransferTest passed");
}

async function listTxs() {
  const res1 = (await rest_client.internal_txs(userId))[0];
  const res2 = (await rest_client.internal_txs(anotherUserId))[0];
  console.log(res1, res2);
  assert.equal(res1.amount, res2.amount);
  assert.equal(res1.asset, res2.asset);
  assert.equal(res1.time, res2.time);
  assert.equal(res1.user_from, res2.user_from);
  assert.equal(res1.user_to, res2.user_to);
}

async function simpleTest() {
  await setupAsset();
  await registerUsers();
  await failureWithZeroDeltaTest();
  await failureWithInsufficientFromBalanceTest();
  await successTransferTest();
  await sleep(3 * 1000);
  await listTxs();
}

async function mainTest() {
  await client.debugReset();
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
