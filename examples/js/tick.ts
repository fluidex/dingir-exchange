import { ORDER_SIDE_BID, ORDER_SIDE_ASK } from "./config";
import { defaultClient as client } from "./client";
import {
  sleep,
  putLimitOrder,
  getRandomFloatAround,
  getRandomElem,
  depositAssets
} from "./util";
import axios from "axios";
import { Account } from "fluidex.js";
import { getTestAccount } from "./accounts";
import { strict as assert } from "assert";

const verbose = true;
const botsIds = [1, 2, 3, 4, 5];
let markets: Array<string> = [];
let prices = new Map<string, number>();

async function initClient() {
  await client.connect();
  markets = Array.from(client.markets.keys());
}
async function loadAccounts() {
  for (const user_id of botsIds) {
    let acc = Account.fromMnemonic(getTestAccount(user_id).mnemonic);
    client.addAccount(user_id, acc);
  }
}
async function registerAccounts() {
  for (const user_id of botsIds) {
    // TODO: clean codes here
    let acc = Account.fromMnemonic(getTestAccount(user_id).mnemonic);
    await client.client.RegisterUser({
      user_id,
      l1_address: acc.ethAddr,
      l2_pubkey: acc.bjjPubKey
    });
  }
}
async function initAssets() {
  for (const user_id of botsIds) {
    await depositAssets({ USDT: "500000.0" }, user_id);
    for (const [name, info] of client.markets) {
      const base = info.base;
      const depositReq = {};
      depositReq[base] = "10";
      await depositAssets(depositReq, user_id);
    }
  }
}
function randUser() {
  return getRandomElem(botsIds);
}
let priceUpdatedTime = 0;
async function updatePrices(backend) {
  try {
    // limit query rate
    if (Date.now() <= priceUpdatedTime + 60 * 1000) {
      return;
    }
    if (backend == "coinstats") {
      const url =
        "https://api.coinstats.app/public/v1/coins?skip=0&limit=100&currency=USD";
      const data = await axios.get(url);
      for (const elem of data.data.coins) {
        prices.set(elem.symbol, elem.price);
      }
    } else if (backend == "cryptocompare") {
      const url =
        "https://min-api.cryptocompare.com/data/price?fsym=ETH&tsyms=USD";
      // TODO
    }
    priceUpdatedTime = Date.now();
  } catch (e) {
    console.log("update price err", e);
  }
}

function getPrice(token: string): number {
  const price = prices.get(token);
  if (verbose) {
    console.log("price", token, price);
  }
  return price;
}

async function cancelAllForUser(user_id) {
  for (const [market, _] of client.markets) {
    console.log(
      "cancel all",
      user_id,
      market,
      await client.orderCancelAll(user_id, market)
    );
  }
  console.log(
    "after cancel all, balance",
    user_id,
    await client.balanceQuery(user_id)
  );
}

async function cancelAll() {
  for (const user_id of botsIds) {
    await cancelAllForUser(user_id);
  }
}

async function transferTest() {
  console.log("successTransferTest BEGIN");

  const res1 = await client.transfer(botsIds[0].toString(), botsIds[1].toString(), "USDT", 1000);
  assert.equal(res1.success, true);

  const res2 = await client.transfer(botsIds[0].toString(), botsIds[2].toString(), "USDT", 1000);
  assert.equal(res2.success, true);

  const res3 = await client.transfer(botsIds[2].toString(), botsIds[3].toString(), "USDT", 1000);
  assert.equal(res3.success, true);

  const res4 = await client.transfer(botsIds[3].toString(), botsIds[1].toString(), "USDT", 1000);
  assert.equal(res4.success, true);

  console.log("successTransferTest END");
}

async function run() {
  for (let cnt = 0; ; cnt++) {
    try {
      await sleep(1000);
      await updatePrices("coinstats");
      async function tickForUser(user) {
        if (Math.floor(cnt / botsIds.length) % 200 == 0) {
          await cancelAllForUser(user);
        }
        for (let market of markets) {
          const price = getPrice(market.split("_")[0]);
          await putLimitOrder(
            user,
            market,
            getRandomElem([ORDER_SIDE_BID, ORDER_SIDE_ASK]),
            getRandomFloatAround(0.3, 0.3),
            getRandomFloatAround(price)
          );
        }
      }
      const userId = botsIds[cnt % botsIds.length];
      await tickForUser(userId);
    } catch (e) {
      console.log(e);
    }
  }
}
async function main() {
  const reset = true;
  await initClient();
  await loadAccounts();
  //await cancelAll();
  if (reset) {
    await client.debugReset();
    await registerAccounts();
    await initAssets();
    await transferTest();
  }
  await run();
}
main().catch(console.log);
