import { ORDER_SIDE_BID, ORDER_SIDE_ASK } from "../config";
import { defaultClient as client } from "../client";
import { sleep, getRandomFloatAround, getRandomFloatAroundNormal, getRandomElem } from "../util";
import { Account } from "fluidex.js";
import { getTestAccount } from "../accounts";
import { strict as assert } from "assert";
import { getPriceOfCoin, putLimitOrder } from "../exchange_helper";
import ID from "./ids";

const verbose = true;
const botsIds = ID.userID;
const brokerID = ID.brokerID;
const accountID = ID.accountID;
let markets: Array<string> = [];

function businessId() {
  return Date.now();
}

async function initClient() {
  await client.connect();
  markets = Array.from(client.markets.keys());
}
async function loadAccounts() {
  for (const user_id of botsIds) {
    let acc = Account.fromMnemonic(getTestAccount(user_id).mnemonic);
    console.log("acc", user_id, acc);
    client.addAccount(user_id, acc);
  }
}
/*
async function registerAccounts() {
  for (let i = 0; i < botsIds.length; i++) {
    // TODO: clean codes here
    let acc = Account.fromMnemonic(getTestAccount(botsIds[i]).mnemonic);
    await client.registerUser({
      user_id: botsIds[i],
      broker_id: brokerID[i],
      account_id: accountID[i],
      l1_address: acc.ethAddr,
      l2_pubkey: acc.bjjPubKey,
    });
  }
}
async function initAssets() {
  for (let i = 0; i < botsIds.length; i++) {
    await depositAssets({ USDT: "500000.0" }, botsIds[i], brokerID[i], accountID[i]);
    for (const [name, info] of client.markets) {
      const base = info.base;
      const depositReq = {};
      depositReq[base] = "10";
      await depositAssets(depositReq, botsIds[i], brokerID[i], accountID[i]);
    }
  }
}
function randUser() {
  return getRandomElem(botsIds);
}
*/

async function getPrice(token: string): Promise<number> {
  const price = await getPriceOfCoin(token);
  if (verbose) {
    console.log("price", token, price);
  }
  return price;
}

async function cancelAllForUser(user_id: string, broker_id: string, account_id: string) {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  for (const [market, _] of client.markets) {
    console.log("cancel all", user_id, market, await client.orderCancelAll(user_id, market));
  }
  console.log("after cancel all, balance", user_id, await client.balanceQuery(user_id, broker_id, account_id));
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
async function cancelAll() {
  for (let i = 0; i < botsIds.length; i++) {
    await cancelAllForUser(botsIds[i], brokerID[i], accountID[i]);
  }
}

async function transferTest() {
  console.log("successTransferTest BEGIN");

  const res1 = await client.transfer(botsIds[0], brokerID[0], accountID[0], botsIds[1], brokerID[1], accountID[1], "USDT", 1000);
  assert.equal(res1.success, true);

  const res2 = await client.transfer(botsIds[1], brokerID[1], accountID[1], botsIds[2], brokerID[2], accountID[2], "USDT", 1000);
  assert.equal(res2.success, true);

  const res3 = await client.transfer(botsIds[2], brokerID[2], accountID[2], botsIds[3], brokerID[3], accountID[3], "USDT", 1000);
  assert.equal(res3.success, true);

  const res4 = await client.transfer(botsIds[3], brokerID[3], accountID[3], botsIds[0], brokerID[0], accountID[0], "USDT", 1000);
  assert.equal(res4.success, true);

  console.log("successTransferTest END");
}

async function withdrawTest() {
  console.log("withdrawTest BEGIN");

  await client.withdraw(botsIds[0], "USDT", "withdraw", businessId(), 100, {
    key0: "value0",
  });

  await client.withdraw(botsIds[1], "USDT", "withdraw", businessId(), 100, {
    key1: "value1",
  });

  await client.withdraw(botsIds[2], "USDT", "withdraw", businessId(), 100, {
    key2: "value2",
  });

  await client.withdraw(botsIds[3], "USDT", "withdraw", businessId(), 100, {
    key3: "value3",
  });

  console.log("withdrawTest END");
}

async function run() {
  for (let cnt = 0; ; cnt++) {
    try {
      await sleep(1000);
      async function tickForUser(user, brokerId, accountId) {
        if (Math.floor(cnt / botsIds.length) % 200 == 0) {
          await cancelAllForUser(user, brokerId, accountId);
        }
        for (let market of markets) {
          const price = await getPrice(market.split("_")[0]);
          await putLimitOrder(
            user,
            brokerId,
            accountId,
            market,
            getRandomElem([ORDER_SIDE_BID, ORDER_SIDE_ASK]),
            getRandomFloatAround(0.3, 0.3),
            getRandomFloatAroundNormal(price)
          );
        }
      }
      const userId = botsIds[cnt % botsIds.length];
      const brokerId = `${userId}`;
      const accountId = `${userId}`;
      await tickForUser(userId, brokerId, accountId);
    } catch (e) {
      console.log(e);
    }
  }
}
async function main() {
  const reset = true;
  await loadAccounts();
  await initClient();
  //await cancelAll();
  if (reset) {
    await client.debugReset();
    //    await registerAccounts();
    //    await initAssets();
    await transferTest();
    await withdrawTest();
  }
  await run();
}
main().catch(console.log);
