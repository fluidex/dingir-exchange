import axios from "axios";
import { Account } from "fluidex.js";
import { defaultClient as client } from "../client";
import { depositAssets } from "../exchange_helper";
import { fee, ORDER_SIDE_BID, ORDER_TYPE_LIMIT } from "../config";
import { getTestAccount } from "../accounts";
import { strict as assert } from "assert";
import ID from "./ids";

const botsIds = ID.userID;
const brokerIds = ID.brokerID;
const accountIds = ID.accountID;
const apiServer = process.env.API_ENDPOINT || "0.0.0.0:8765";

async function loadAccounts() {
  for (const user_id of botsIds) {
    let acc = Account.fromMnemonic(getTestAccount(user_id).mnemonic);
    // console.log("acc", user_id, acc);
    client.addAccount(user_id, acc);
  }
}

async function initClient() {
  await client.connect();
}

async function registerAccounts() {
  for (let i = 0; i < botsIds.length; i++) {
    let acc = Account.fromMnemonic(getTestAccount(botsIds[i]).mnemonic);
    const req = {
      user_id: botsIds[i],
      broker_id: brokerIds[i],
      account_id: accountIds[i],
      l1_address: acc.ethAddr,
      l2_pubkey: acc.bjjPubKey,
    };
    await client.client.RegisterUser(req);
  }
}

async function initAssets() {
  for (let i = 0; i < botsIds.length; i++) {
    await depositAssets({ USDT: "500000.0" }, botsIds[i], brokerIds[i], accountIds[i]);

    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    for (const [name, info] of client.markets) {
      const base = info.base;
      const depositReq = {};
      depositReq[base] = "10";
      await depositAssets(depositReq, botsIds[i], brokerIds[i], accountIds[i]);
    }
  }
}

async function mainTest() {
  await putOrdersTest();
  await putAndResetOrdersTest();
}

// Put multiple orders
async function putOrdersTest() {
  console.log("putOrdersTest Begin");

  const oldOrderNum1 = await openOrderNum(botsIds[0], brokerIds[0], accountIds[0]);
  const oldOrderNum2 = await openOrderNum(botsIds[1], accountIds[1], accountIds[1]);

  const res = await client.batchOrderPut("ETH_USDT", false, [
    {
      user_id: botsIds[0],
      market: "ETH_USDT",
      order_side: ORDER_SIDE_BID,
      order_type: ORDER_TYPE_LIMIT,
      amount: "1",
      price: "1",
      taker_fee: fee,
      maker_fee: fee,
      broker_id: brokerIds[0],
      account_id: accountIds[0],
    },
    {
      user_id: botsIds[1],
      market: "ETH_USDT",
      order_side: ORDER_SIDE_BID,
      order_type: ORDER_TYPE_LIMIT,
      amount: "1",
      price: "1",
      taker_fee: fee,
      maker_fee: fee,
      broker_id: brokerIds[1],
      account_id: accountIds[1],
    },
  ]);
  console.log(res);
  const newOrderNum1 = await openOrderNum(botsIds[0], brokerIds[0], accountIds[0]);
  const newOrderNum2 = await openOrderNum(botsIds[1], brokerIds[1], accountIds[1]);

  assert.equal(newOrderNum1 - oldOrderNum1, 1);
  assert.equal(newOrderNum2 - oldOrderNum2, 1);

  console.log("putOrdersTest End");
}

// Put and reset multiple orders
async function putAndResetOrdersTest() {
  console.log("putAndResetOrdersTest Begin");

  const userId1 = botsIds[0];
  const userId2 = botsIds[1];
  const oldOrderNum1 = await openOrderNum(userId1, brokerIds[0], accountIds[0]);
  assert(oldOrderNum1 > 0);
  const oldOrderNum2 = await openOrderNum(userId2, brokerIds[1], accountIds[1]);
  assert(oldOrderNum2 > 0);

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const res = await client.batchOrderPut("ETH_USDT", true, [
    {
      user_id: botsIds[0],
      market: "ETH_USDT",
      order_side: ORDER_SIDE_BID,
      order_type: ORDER_TYPE_LIMIT,
      amount: "1",
      price: "1",
      taker_fee: fee,
      maker_fee: fee,
      broker_id: brokerIds[0],
      account_id: accountIds[0],
    },
    {
      user_id: botsIds[1],
      market: "ETH_USDT",
      order_side: ORDER_SIDE_BID,
      order_type: ORDER_TYPE_LIMIT,
      amount: "1",
      price: "1",
      taker_fee: fee,
      maker_fee: fee,
      broker_id: brokerIds[1],
      account_id: accountIds[1],
    },
  ]);

  const newOrderNum1 = await openOrderNum(userId1, brokerIds[0], accountIds[0]);
  const newOrderNum2 = await openOrderNum(userId2, brokerIds[1], accountIds[1]);
  assert.equal(newOrderNum1, 1);
  assert.equal(newOrderNum2, 1);

  console.log("putAndResetOrdersTest End");
}

async function openOrderNum(userId, brokeId, accountId) {
  return (await axios.get(`http://${apiServer}/api/exchange/action/orders/ETH_USDT/${userId}/${brokeId}/${accountId}`)).data.orders.length;
}

async function main() {
  try {
    await loadAccounts();
    await initClient();
    await client.debugReset();
    await registerAccounts();
    await initAssets();
    await mainTest();
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}

main();
