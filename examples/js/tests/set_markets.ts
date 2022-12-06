import axios from "axios";
import { strict as assert } from "assert";
import "../config";

const isCI = !!process.env.GITHUB_ACTIONS;

const new_asset = {
  assets: [{ name: "BTC", prec_save: 4, prec_show: 4 }],
  not_reload: true,
};

const new_market1 = {
  market: {
    name: "USDC_BTC",
    base: "USDC",
    quote: "BTC",
    amount_prec: 2,
    price_prec: 2,
    fee_prec: 2,
    min_amount: 0.01,
  },
  asset_base: {
    id: "USDC",
    symbol: "USDC",
    name: "USD Coin",
    chain_id: 3,
    token_address: "",
    rollup_token_id: 2,
    prec_save: 8,
    prec_show: 8,
    logo_uri: "",
  },
  asset_quote: {
    id: "BTC",
    symbol: "BTC",
    name: "Bitcoin",
    chain_id: 2,
    token_address: "",
    rollup_token_id: 2,
    prec_save: 8,
    prec_show: 8,
    logo_uri: "",
  },
  not_reload: false,
};

const new_market2 = {
  market: {
    name: "USDT_BTC",
    base: "USDT",
    quote: "BTC",
    amount_prec: 2,
    price_prec: 2,
    fee_prec: 2,
    min_amount: 0.01,
  },
  not_reload: false,
};
async function main() {
  const server = process.env.API_ENDPOINT || "0.0.0.0:8765";
  console.log("ci mode:", isCI);
  console.log("add asset");
  const ret1 = (await axios.post(`http://${server}/api/exchange/panel/manage/market/assets`, new_asset)).data;
  console.log(ret1);
  if (isCI) {
    assert.equal(ret1, "done");
  }
  console.log("add market 1");
  const ret2 = (await axios.post(`http://${server}/api/exchange/panel/manage/market/tradepairs`, new_market1)).data;
  console.log(ret2);
  if (isCI) {
    assert.equal(ret2, "done");
  }
  const { markets } = (await axios.get(`http://${server}/api/exchange/action/markets`)).data;
  console.log(markets);
  if (isCI) {
    assert.equal(markets.length, 2);
  }
  console.log("add market 2");
  const ret3 = (await axios.post(`http://${server}/api/exchange/panel/manage/market/tradepairs`, new_market2)).data;
  console.log(ret3);
  if (isCI) {
    assert.equal(ret3, "done");
  }
  const { markets: markets2 } = (await axios.get(`http://${server}/api/exchange/action/markets`)).data;
  console.log(markets2);
  if (isCI) {
    assert.equal(markets.length, 3);
  }
}
main().catch(function (e) {
  console.error(e.message);
  process.exit(1);
  //throw e;
});
