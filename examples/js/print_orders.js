import axios from "axios";
import { strict as assert } from "assert";

const isCI = !!process.env.GITHUB_ACTIONS;

async function main() {
  const server = "0.0.0.0";
  console.log("ci mode:", isCI);
  console.log("closed orders:");
  const closedOrders = (
    await axios.get(`http://${server}:8765/restapi/closedorders/ETH_USDT/3`)
  ).data;
  console.log(closedOrders);
  if (isCI) {
    assert.equal(closedOrders.orders.length, 3);
  }
  console.log("active orders:");
  const openOrders = (
    await axios.get(`http://${server}:8765/api/orders/ETH_USDT/3`)
  ).data;
  console.log(openOrders);
  if (isCI) {
    assert.equal(openOrders.orders.length, 1);
  }
  console.log("market ticker:");
  const ticker = (
    await axios.get(`http://${server}:8765/restapi/ticker_24h/ETH_USDT`)
  ).data;
  console.log(ticker);
  if (isCI) {
    assert.equal(ticker.volume, 4);
    assert.equal(ticker.quote_volume, 4.4);
  }
}
main().catch(console.log);
