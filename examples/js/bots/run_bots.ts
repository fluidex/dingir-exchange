import { PriceBot } from "./golden_price";

import { Account } from "fluidex.js";
import { defaultRESTClient, RESTClient } from "../RESTClient";
import {
  defaultClient as defaultGrpcClient,
  Client as grpcClient,
  defaultClient
} from "../client";
import { sleep, depositAssets, getPriceOfCoin } from "../util";
import {
  ORDER_SIDE_BID,
  ORDER_SIDE_ASK,
  ORDER_TYPE_LIMIT,
  VERBOSE
} from "../config";

// TODO: add a similar function using quoteAmount. "i want to sell some eth to get 5000 usdt"
async function estimateMarketOrderSell(
  client: grpcClient,
  market,
  baseAmount: number
) {
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
    worstPrice
  };
  //console.log("estimateMarketOrderSell:", estimateResult);
  return estimateResult;
}

async function estimateMarketOrderBuy(
  client: grpcClient,
  market,
  quoteAmount: number
) {
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
    worstPrice
  };
  //console.log("estimateMarketOrderBuy:", estimateResult);
  return estimateResult;
}

async function execMarketOrderAsLimit_Sell(
  client: grpcClient,
  market,
  baseAmount: string,
  uid
) {
  /*
  let estimateResult = await estimateMarketOrderBuy(
    client,
    market,
    Number(amount)
  );
  */
  const price = "0.01"; // low enough as a market order...
  let order = await client.orderPut(
    uid,
    market,
    ORDER_SIDE_ASK,
    ORDER_TYPE_LIMIT,
    baseAmount,
    price,
    "0",
    "0"
  );
  //console.log("execMarketOrderAsLimit_Sell", order);
}

async function execMarketOrderAsLimit_Buy(
  client: grpcClient,
  market,
  quoteAmount: string,
  uid
) {
  let estimateResult = await estimateMarketOrderBuy(
    client,
    market,
    Number(quoteAmount)
  );
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

async function initUser(): Promise<number> {
  const mnemonic1 =
    "split logic consider degree smile field term style opera dad believe indoor item type beyond";
  const mnemonic2 =
    "camp awful sand include refuse cash reveal mystery pupil salad length plunge square admit vocal draft found side same clock hurt length say figure";
  const mnemonic3 =
    "sound select report rug run cave provide index grief foster bar someone garage donate nominee crew once oil sausage flight tail holiday style afford";
  const acc = Account.fromMnemonic(mnemonic2);
  //console.log('acc is', acc);
  const restClient = defaultRESTClient;
  let userInfo = await restClient.get_user_by_addr(acc.ethAddr);
  if (userInfo == null) {
    // register
    console.log("register new user");
    let resp = await defaultGrpcClient.registerUser({
      user_id: 0, // discard in server side
      l1_address: acc.ethAddr,
      l2_pubkey: acc.bjjPubKey
    });
    const t = Date.now();
    console.log("register resp", resp);
    await sleep(2000); // FIXME
    userInfo = await restClient.get_user_by_addr(acc.ethAddr);
    await sleep(2000); // FIXME
    await depositAssets({ USDT: "10000.0" }, userInfo.id);
  } else {
    console.log("user", "already registered");
  }
  console.log("user", userInfo);

  defaultClient.addAccount(userInfo.id, acc);
  return userInfo.id;
}

async function rebalance(user_id) {
  let rebalanced = false;
  const balance = await defaultGrpcClient.balanceQuery(user_id);
  const allBase =
    Number(balance.get(baseCoin).available) +
    Number(balance.get(baseCoin).frozen);
  const allQuote =
    Number(balance.get(quoteCoin).available) +
    Number(balance.get(quoteCoin).frozen);
  //onsole.log("balance when start", { balance, allBase, allQuote });

  if (allBase < 0.1) {
    await defaultGrpcClient.orderCancelAll(user_id, market);

    await execMarketOrderAsLimit_Buy(defaultClient, market, "5000", user_id);
    rebalanced = true;
  }
  if (allQuote < 1000) {
    await defaultGrpcClient.orderCancelAll(user_id, market);

    // TODO: use quote amount rather than base amount
    await execMarketOrderAsLimit_Sell(
      defaultClient,
      market,
      "1.5" /*base*/,
      user_id
    );
    rebalanced = true;
  }
  return rebalanced;
}

const market = "ETH_USDT";
const baseCoin = "ETH";
const quoteCoin = "USDT";

async function main() {
  const user_id = await initUser();

  await defaultClient.connect();

  await rebalance(user_id);

  let count = 0;
  while (true) {
    //console.log("count:", count);
    try {
      if (count % 100 == 0) {
        const externalPrice = await getPriceOfCoin(baseCoin);
        console.log("externalPrice:", externalPrice);
        console.log(
          "depth:",
          await defaultClient.orderDepth(market, 20, "0.1")
        );
        console.log("orders:", await defaultClient.orderQuery(user_id, market));
        await defaultGrpcClient.orderCancelAll(user_id, market);

        async function printBalance() {
          const balance = await defaultGrpcClient.balanceQuery(user_id);
          const allBase =
            Number(balance.get(baseCoin).available) +
            Number(balance.get(baseCoin).frozen);
          const allQuote =
            Number(balance.get(quoteCoin).available) +
            Number(balance.get(quoteCoin).frozen);

          let res = await estimateMarketOrderSell(
            defaultGrpcClient,
            market,
            allBase
          );
          console.log("external base price", externalPrice);
          console.log("------- BALANCE1:", {
            quote: allQuote,
            base: res.quote,
            total: allQuote + res.quote
          });
          console.log("------- BALANCE2:", {
            quote: allQuote,
            base: allBase * externalPrice,
            total: allQuote + allBase * externalPrice,
            totalInB: allQuote / externalPrice + allBase
          });
        }

        //console.log("before rebalance");
        await printBalance();
        /*
      const relanced = await rebalance(user_id);
      if (relanced) {
        console.log("after rebalance");
        await printBalance();
      }
      */
      }
      let orderbook = null; // fetch orderbook
      // put a big buy order and a big sell order
      const price = await getPriceOfCoin(baseCoin, 5);

      const balance = await defaultGrpcClient.balanceQuery(user_id);
      const allBase =
        Number(balance.get(baseCoin).available) +
        Number(balance.get(baseCoin).frozen);
      const allQuote =
        Number(balance.get(quoteCoin).available) +
        Number(balance.get(quoteCoin).frozen);
      //console.log({allBase, allQuote});
      const ratio = 0.8; // use 80% of my assets to make market

      const spread = 0.0005;
      const askPrice = price * (1 + spread);
      const bidPrice = price * (1 - spread);
      const bidAmount = (allQuote * ratio) / bidPrice;
      const askAmount = allBase * ratio;

      // const { user_id, market, order_side, order_type, amount, price, taker_fee, maker_fee } = o;
      const bid_order = {
        user_id,
        market,
        order_side: ORDER_SIDE_BID,
        order_type: ORDER_TYPE_LIMIT,
        amount: bidAmount,
        price: bidPrice
      };
      const ask_order = {
        user_id,
        market,
        order_side: ORDER_SIDE_ASK,
        order_type: ORDER_TYPE_LIMIT,
        amount: askAmount,
        price: askPrice
      };

      await defaultGrpcClient.orderCancelAll(user_id, market);
      //console.log('batch orders:', orders);
      try {
        if (bidAmount > 0.001) {
          await defaultClient.orderPut(
            user_id,
            market,
            ORDER_SIDE_BID,
            ORDER_TYPE_LIMIT,
            bidAmount,
            bidPrice,
            "0",
            "0"
          );
        }
      } catch (e) {
        console.log("put error", bid_order, e);
      }
      try {
        if (askAmount > 0.001) {
          await defaultClient.orderPut(
            user_id,
            market,
            ORDER_SIDE_ASK,
            ORDER_TYPE_LIMIT,
            askAmount,
            askPrice,
            "0",
            "0"
          );
        }
      } catch (e) {
        console.log("put error", ask_order, e);
      }
      //let batchPutResult = await defaultClient.batchOrderPut(market, true, orders);
      //console.log('batch put result:', batchPutResult)
    } catch (e) {
      console.log("err", e);
    }
    count += 1;
    sleep(500);
  }
}

main();
