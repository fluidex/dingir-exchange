import { Account } from "fluidex.js";
import { defaultRESTClient, RESTClient } from "../RESTClient";
import { defaultClient as defaultGrpcClient, Client as grpcClient, defaultClient } from "../client";
import { sleep } from "../util";
import { depositAssets, getPriceOfCoin } from "../exchange_helper";
import { ORDER_SIDE_BID, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, VERBOSE } from "../config";

// TODO: add a similar function using quoteAmount. "i want to sell some eth to get 5000 usdt"
// TODO: exclude my orders
async function estimateMarketOrderSell(client: grpcClient, market, baseAmount: number) {
  const orderbook = await client.orderDepth(market, 20, "0.01");
  //console.log('depth', orderbook);
  //console.log(client.markets);
  let quoteAcc = 0;
  let baseAcc = 0;
  let worstPrice = 0; //
  let bestPrice = Number(orderbook.bids[0].price);
  for (const elem of orderbook.bids) {
    let amount = Number(elem.amount);
    let price = Number(elem.price);
    if (baseAcc + amount > baseAmount) {
      amount = baseAmount - baseAcc;
    }
    baseAcc += amount;
    quoteAcc += amount * price;
    worstPrice = price;
  }
  let estimateResult = {
    base: baseAcc,
    quote: quoteAcc,
    avgPrice: quoteAcc / baseAcc,
    bestPrice,
    worstPrice,
  };
  //console.log("estimateMarketOrderSell:", estimateResult);
  return estimateResult;
}

async function estimateMarketOrderBuy(client: grpcClient, market, quoteAmount: number) {
  //await client.connect();
  const orderbook = await client.orderDepth(market, 20, "0.01");
  //console.log('depth', orderbook);
  //console.log(client.markets);
  let quoteAcc = 0;
  let tradeAmount = 0;
  let worstPrice = 0; //
  let bestPrice = Number(orderbook.asks[0].price);
  for (const elem of orderbook.asks) {
    let amount = Number(elem.amount);
    let price = Number(elem.price);
    let quote = amount * price;
    if (quoteAcc + quote > quoteAmount) {
      amount = (quoteAmount - quoteAcc) / price;
    }
    tradeAmount += amount;
    quoteAcc += amount * price;
    worstPrice = price;
  }
  let estimateResult = {
    base: tradeAmount,
    quote: quoteAcc,
    avgPrice: quoteAcc / tradeAmount,
    bestPrice,
    worstPrice,
  };
  //console.log("estimateMarketOrderBuy:", estimateResult);
  return estimateResult;
}

async function execMarketOrderAsLimit_Sell(client: grpcClient, market, baseAmount: string, uid) {
  /*
    let estimateResult = await estimateMarketOrderBuy(
      client,
      market,
      Number(amount)
    );
    */
  const price = "0.01"; // low enough as a market order...
  let order = await client.orderPut(uid, market, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, baseAmount, price, "0", "0");
  //console.log("execMarketOrderAsLimit_Sell", order);
}

async function execMarketOrderAsLimit_Buy(client: grpcClient, market, quoteAmount: string, uid) {
  let estimateResult = await estimateMarketOrderBuy(client, market, Number(quoteAmount));
  let order = await client.orderPut(
    uid,
    market,
    ORDER_SIDE_BID,
    ORDER_TYPE_LIMIT,
    estimateResult.base,
    estimateResult.worstPrice * 1.1,
    "0",
    "0"
  );
  //console.log("execMarketOrderAsLimit_Buy", order);
}

async function rebalance(user_id, baseCoin, quoteCoin, market) {
  let rebalanced = false;
  const balance = await defaultGrpcClient.balanceQuery(user_id);
  const allBase = Number(balance.get(baseCoin).available) + Number(balance.get(baseCoin).frozen);
  const allQuote = Number(balance.get(quoteCoin).available) + Number(balance.get(quoteCoin).frozen);
  //onsole.log("balance when start", { balance, allBase, allQuote });

  if (allBase < 0.1) {
    await defaultGrpcClient.orderCancelAll(user_id, market);

    await execMarketOrderAsLimit_Buy(defaultClient, market, "5000", user_id);
    rebalanced = true;
  }
  if (allQuote < 1000) {
    await defaultGrpcClient.orderCancelAll(user_id, market);

    // TODO: use quote amount rather than base amount
    await execMarketOrderAsLimit_Sell(defaultClient, market, "1.5" /*base*/, user_id);
    rebalanced = true;
  }
  return rebalanced;
}

async function totalBalance(user_id, baseCoin, quoteCoin, market, externalPrice = null) {
  if (externalPrice == null) {
    externalPrice = await getPriceOfCoin(baseCoin);
  }
  const balance = await defaultGrpcClient.balanceQuery(user_id);
  const allBase = Number(balance.get(baseCoin).available) + Number(balance.get(baseCoin).frozen);
  const allQuote = Number(balance.get(quoteCoin).available) + Number(balance.get(quoteCoin).frozen);
  return {
    quote: allQuote,
    base: allBase,
    quoteValue: allQuote, // stable coin
    baseValue: allBase * externalPrice,
    totalValue: allQuote + allBase * externalPrice,
    totalValueInBase: allQuote / externalPrice + allBase,
  };
}

async function printBalance(user_id, baseCoin, quoteCoin, market) {
  const balance = await defaultGrpcClient.balanceQuery(user_id);
  const allBase = Number(balance.get(baseCoin).available) + Number(balance.get(baseCoin).frozen);
  const allQuote = Number(balance.get(quoteCoin).available) + Number(balance.get(quoteCoin).frozen);

  let res = await estimateMarketOrderSell(defaultGrpcClient, market, allBase);
  console.log("------- BALANCE1:", {
    quote: allQuote,
    base: res.quote,
    total: allQuote + res.quote,
  });

  const externalPrice = await getPriceOfCoin(baseCoin);
  console.log("external base price", externalPrice);
  console.log("------- BALANCE2:", {
    quote: allQuote,
    base: allBase,
    quoteValue: allQuote, // stable coin
    baseValue: allBase * externalPrice,
    totalValue: allQuote + allBase * externalPrice,
    totalValueInBase: allQuote / externalPrice + allBase,
  });
}

export {
  estimateMarketOrderSell,
  estimateMarketOrderBuy,
  execMarketOrderAsLimit_Sell,
  execMarketOrderAsLimit_Buy,
  rebalance,
  printBalance,
  totalBalance,
};
