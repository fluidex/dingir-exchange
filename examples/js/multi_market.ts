import axios from "axios";
import { Account } from "fluidex.js";

import { defaultClient as client } from "./client";
import { getTestAccount } from "./accounts";
import { fee, market, ORDER_SIDE_BID, ORDER_TYPE_LIMIT } from "./config";
import { depositAssets } from "./util";
import { strict as assert } from "assert";

const userId = 1;
const isCI = !!process.env.GITHUB_ACTIONS;
const server = process.env.API_ENDPOINT || "0.0.0.0:8765";

async function initAccounts() {
  await client.debugReset();
  await client.connect();
  let acc = Account.fromMnemonic(getTestAccount(userId).mnemonic);
  client.addAccount(userId, acc);
  await client.client.RegisterUser({
    userId,
    l1_address: acc.ethAddr,
    l2_pubkey: acc.bjjPubKey
  });
}

async function setupAsset() {
  await depositAssets({ USDT: "100.0", ETH: "50.0" }, userId);
}

async function orderTest() {
  const markets = Array.from([
    "ETH_USDT",
    "LINK_USDT",
    "MATIC_USDT",
    "UNI_USDT"
  ]);
  let orders = await Promise.all(
    markets.map(market =>
      client
        .orderPut(
          userId,
          market,
          ORDER_SIDE_BID,
          ORDER_TYPE_LIMIT,
          /*amount*/ "1",
          /*price*/ "1.1",
          fee,
          fee
        )
        .then(o => [market, o.id])
    )
  );
  console.log(orders);
  assert.equal(orders.length, 4);

  const openOrders = (await axios.get(`http://${server}/api/orders/all/1`))
    .data;
  console.log(openOrders);
  if (isCI) {
    assert.equal(openOrders.orders.length, orders.length);
  }

  await Promise.all(
    orders.map(([market, id]) => client.orderCancel(1, market, Number(id)))
  );

  const closedOrders = (
    await axios.get(`http://${server}/restapi/closedorders/all/1`)
  ).data;
  console.log(closedOrders);
  if (isCI) {
    assert.equal(closedOrders.orders.length, orders.length);
  }
}

async function main() {
  try {
    console.log("ci mode:", isCI);
    await initAccounts();
    await setupAsset();
    await orderTest();
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}
main();
