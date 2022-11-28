import { userId, fee, ORDER_SIDE_BID, ORDER_SIDE_ASK, ORDER_TYPE_MARKET, ORDER_TYPE_LIMIT, VERBOSE } from "./config"; // dotenv
import { defaultClient as client } from "./client";

import Decimal from "decimal.js";
const gaussian = require("gaussian");
import { strict as assert } from "assert";
import axios from "axios";
import { getRandomFloat, getRandomInt } from "./util";

function depositId() {
  return Date.now();
}

export async function printBalance(printList = ["USDT", "ETH"]) {
  const balances = await client.balanceQuery(userId);
  console.log("\nasset\tsum\tavaiable\tfrozen");
  for (const asset of printList) {
    const balance = balances.get(asset);
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

export async function depositAssets(assets: any, userId: number, broker_id: string, account_id: string) {
  for (const [asset, amount] of Object.entries(assets)) {
    console.log("deposit", amount, asset);
    await client.balanceUpdate(userId, broker_id, account_id, asset, "deposit", depositId(), amount, {
      key: "value",
    });
  }
}

export async function putLimitOrder(userId, broker_id, account_id, market, side, amount, price) {
  return await client.orderPut(userId, broker_id, account_id, market, side, ORDER_TYPE_LIMIT, amount, price, fee, fee);
}
export async function putRandOrder(userId, broker_id, account_id, market) {
  // TODO: market order?
  const side = [ORDER_SIDE_ASK, ORDER_SIDE_BID][getRandomInt(0, 10000) % 2];
  const price = getRandomFloat(1350, 1450);
  const amount = getRandomFloat(0.5, 1.5);
  const order = await putLimitOrder(userId, broker_id, account_id, market, side, amount, price);
  //console.log("order put", order.id.toString(), { side, price, amount });
}

let pricesCache = new Map();
let pricesUpdatedTime = 0;
export async function getPriceOfCoin(
  sym,
  timeout = 60, // default 1min
  backend = "coinstats"
): Promise<number> {
  // limit query rate
  if (Date.now() > pricesUpdatedTime + timeout * 1000) {
    // update prices
    try {
      if (backend == "coinstats") {
        const url = "https://api.coinstats.app/public/v1/coins?skip=0&limit=100&currency=USD";
        const data = await axios.get(url);
        for (const elem of data.data.coins) {
          pricesCache.set(elem.symbol, elem.price);
        }
      } else if (backend == "cryptocompare") {
        const url = "https://min-api.cryptocompare.com/data/price?fsym=ETH&tsyms=USD";
        // TODO
      }

      pricesUpdatedTime = Date.now();
    } catch (e) {
      console.log("update prices err", e);
    }
  }

  return pricesCache.get(sym);
}
