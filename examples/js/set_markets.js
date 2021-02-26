import axios from "axios";
import { strict as assert } from "assert";
import './config.mjs';

const isCI = !!process.env.GITHUB_ACTIONS;

const new_asset = {
  "assets": [{"name":"BTC", "prec_save":4, "prec_show":4}],
  "force_update": false
}

const new_market1 = {
  "market": {
      "name":"BTC_USDC", 
      "base":{"name":"BTC", "prec": 2}, 
      "quote":{"name":"USDC", "prec": 4}, 
      "fee_prec":2,
      "min_amount":0.01
  },
  "asset_quote": {
      "name":"USDC",
      "prec_save": 6,
      "prec_show": 6
  }
}

const new_market2 = {
  "market": {
      "name":"BTC_USDT", 
      "base":{"name":"BTC", "prec": 2}, 
      "quote":{"name":"USDT", "prec": 4}, 
      "fee_prec":2,
      "min_amount":0.01
  }
}
async function main() {
  const server = process.env.API_ENDPOINT || "0.0.0.0:8765";
  console.log("ci mode:", isCI);
  console.log("add asset");
  const ret1 = (
    await axios.post(`http://${server}/restapi/manage/market/assets`, new_asset)
  ).data;
  console.log(ret1);
  if (isCI) {
    assert.equal(ret1, "done");
  }
  console.log("add market 1");
  const ret2 = (
    await axios.post(`http://${server}/restapi/manage/market/tradepairs`, new_market1)
  ).data;
  console.log(ret2);
  if (isCI) {
    assert.equal(ret2, "done");
  }
  const {markets} = (
    await axios.get(`http://${server}/api/markets`)
  ).data;
  console.log(markets);
  if (isCI) {
    assert.equal(markets.length, 2);
  }
  console.log("add market 2");
  const ret3 = (
    await axios.post(`http://${server}/restapi/manage/market/tradepairs`, new_market2)
  ).data;
  console.log(ret3);
  if (isCI) {
    assert.equal(ret3, "done");
  }
  const {markets2} = (
    await axios.get(`http://${server}/api/markets`)
  ).data;
  console.log(markets2);
  if (isCI) {
    assert.equal(markets.length, 3);
  }  
}
main().catch(function(e) {
  console.log(e);
  process.exit(1);
  //throw e;
});
