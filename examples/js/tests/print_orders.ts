import axios from "axios";
import { strict as assert } from "assert";
import "../config";
import ID from "./ids";

const isCI = !!process.env.GITHUB_ACTIONS;

async function main() {
  const userID = ID.userID[4];
  const brokerID = ID.brokerID[4];
  const accountID = ID.accountID[4];
  const server = process.env.API_ENDPOINT || "0.0.0.0:8765";
  console.log("ci mode:", isCI);
  console.log("closed orders:");
  const closedOrders = (await axios.get(`http://${server}/api/exchange/panel/closedorders/ETH_USDT/${userID}`)).data;
  console.log(closedOrders);
  if (isCI) {
    assert.equal(closedOrders.orders.length, 2);
  }
  console.log("active orders:");
  const openOrders = (await axios.get(`http://${server}/api/exchange/action/orders/ETH_USDT/${userID}/${brokerID}/${accountID}`)).data;
  console.log(openOrders);
  if (isCI) {
    assert.equal(openOrders.orders.length, 1);
  }
  console.log("market ticker:");
  const ticker = (await axios.get(`http://${server}/api/exchange/panel/ticker_24h/ETH_USDT`)).data;
  console.log(ticker);
  if (isCI) {
    assert.equal(ticker.volume, 4);
    assert.equal(ticker.quote_volume, 4.4);
  }
}
main().catch(function (e) {
  console.log(e);
  process.exit(1);
  //throw e;
});
