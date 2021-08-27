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
// TODO: exclude my orders
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
  const acc = Account.fromMnemonic(mnemonic3);
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
  //let lastBidPrice = '0';
  //let lastBidAmount = '0';
  //let lastAskPrice = '0';
  //let lastAskAmount = '0';
  while (true) {
    if (VERBOSE) {
      console.log("count:", count);
    }
    count += 1;
    if (VERBOSE) {
      console.log("sleep 500ms");
    }
    await sleep(500);
    try {
      if (count % 100 == 1) {
        const externalPrice = await getPriceOfCoin(baseCoin);
        console.log("externalPrice:", externalPrice);
        //       console.log(
        //       "depth:",
        //     await defaultClient.orderDepth(market, 20, "0.1")
        // );
        console.log(
          "my orders:",
          await defaultClient.orderQuery(user_id, market)
        );
        //        await defaultGrpcClient.orderCancelAll(user_id, market);

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

      const oldOrders = await defaultClient.orderQuery(user_id, market);
      if (VERBOSE) {
        console.log("oldOrders", oldOrders);
      }
      const oldAskOrder = oldOrders.orders.find(
        elem => elem.order_side == "ASK"
      );
      const oldBidOrder = oldOrders.orders.find(
        elem => elem.order_side == "BID"
      );

      //      await defaultGrpcClient.orderCancelAll(user_id, market);

      //console.log('batch orders:', orders);
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
      let askPriceRaw = price * (1 + spread);
      let bidPriceRaw = price * (1 - spread);
      let bidAmountRaw = (allQuote * ratio) / bidPriceRaw;
      let askAmountRaw = allBase * ratio;
      let {
        price: askPrice,
        amount: askAmount
      } = defaultGrpcClient.roundOrderInput(market, askAmountRaw, askPriceRaw);
      let {
        price: bidPrice,
        amount: bidAmount
      } = defaultGrpcClient.roundOrderInput(market, bidAmountRaw, bidPriceRaw);
      let minAmount = 0.001;
      if (askAmountRaw < minAmount) {
        askAmount = "";
        askPrice = "";
      }
      if (bidAmountRaw < minAmount) {
        bidAmount = "";
        bidPrice = "";
      }

      // const { user_id, market, order_side, order_type, amount, price, taker_fee, maker_fee } = o;
      if (VERBOSE) {
        console.log({ bidPrice, bidAmount, askAmount, askPrice });
        //console.log({ bidPriceRaw, bidAmountRaw, askAmountRaw, askPriceRaw });
      }
      let lastBidPrice = oldBidOrder?.price || "";
      let lastBidAmount = oldBidOrder?.amount || "";
      let lastAskPrice = oldAskOrder?.price || "";
      let lastAskAmount = oldAskOrder?.amount || "";
      //if(bidPrice == lastBidPrice && bidAmount == lastBidAmount && askPrice ==lastAskPrice && askAmount == lastAskAmount) {
      if (bidPrice == lastBidPrice && askPrice == lastAskPrice) {
        if (VERBOSE) {
          console.log("same order shape, skip");
        }
        continue;
      }
      //lastAskPrice = askPrice;
      //lastAskAmount = askAmount;
      //lastBidPrice = bidPrice;
      //lastBidAmount = bidAmount;
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

      try {
        if (
          Number(bidAmount) > 0.001 &&
          (true || bidAmount != lastBidAmount || bidPrice != lastBidPrice)
        ) {
          let res = await defaultClient.orderPut(
            user_id,
            market,
            ORDER_SIDE_BID,
            ORDER_TYPE_LIMIT,
            bidAmount,
            bidPrice,
            "0",
            "0"
          );
          if (true || VERBOSE) {
            console.log("put", res);
          }
        }
      } catch (e) {
        console.log("put error", bid_order, e);
      }
      try {
        if (
          Number(askAmount) > 0.001 &&
          (true || askAmount != lastAskAmount || askPrice != lastAskPrice)
        ) {
          let res = await defaultClient.orderPut(
            user_id,
            market,
            ORDER_SIDE_ASK,
            ORDER_TYPE_LIMIT,
            askAmount,
            askPrice,
            "0",
            "0"
          );
          if (true || VERBOSE) {
            console.log("put", res);
          }
        }
      } catch (e) {
        console.log("put error", ask_order, e);
      }
      //let batchPutResult = await defaultClient.batchOrderPut(market, true, orders);
      //console.log('batch put result:', batchPutResult)
    } catch (e) {
      console.log("err", e);
    }
  }
}

main();
