import * as caller from "@eeston/grpc-caller";
import Decimal from "decimal.js";
import { Account, OrderInput, TransferTx, WithdrawTx } from "fluidex.js";
import { ORDER_SIDE_BID, ORDER_SIDE_ASK, ORDER_TYPE_LIMIT, VERBOSE } from "./config";
import { assertDecimalEqual, decimalEqual } from "./util";

const file = "../../orchestra/proto/exchange/matchengine.proto";
const load = {
  keepCase: true,
  longs: String,
  enums: String,
  defaults: true,
  oneofs: true,
};

function fullPrec(d, p): Decimal {
  return new Decimal(d).mul(new Decimal(10).pow(p));
}

class Client {
  client: any;
  markets: Map<string, any> = new Map();
  assets: Map<string, any> = new Map();
  accounts: Map<string, Account> = new Map();
  constructor(server = process.env.GRPC_SERVER || "localhost:50051") {
    console.log("using grpc", server);
    this.client = caller(`${server}`, { file, load }, "Matchengine");
  }

  addAccount(account_id: string, acc: Account) {
    this.accounts.set(account_id, acc);
  }

  async connect() {
    this.markets = await this.marketList();
    for (const elem of await this.assetList()) {
      this.assets.set(elem.symbol, elem);
    }
    console.log("markets", this.markets);
    console.log("assets", this.assets);
  }

  async balanceQuery(user_id, broker_id, account_id): Promise<Map<string, any>> {
    const balances = (await this.client.BalanceQuery({ user_id, broker_id, account_id })).balances;
    let result = new Map();
    for (const entry of balances) {
      result.set(entry.asset_id, entry);
    }
    return result;
  }
  async balanceQueryByAsset(user_id, broker_id, account_id, asset) {
    const allBalances = (await this.client.BalanceQuery({ user_id, broker_id, account_id, assets: [asset] })).balances;
    const balance = allBalances.find(item => item.asset_id == asset);
    let available = new Decimal(balance.available);
    let frozen = new Decimal(balance.frozen);
    let total = available.add(frozen);
    return { available, frozen, total };
  }

  async orderQuery(user_id, market) {
    return await this.client.OrderQuery({ user_id, market });
  }

  async balanceUpdate(user_id, broker_id, account_id, asset, business, business_id, delta, detail) {
    return await this.client.BalanceUpdate({
      user_id,
      broker_id,
      account_id,
      asset,
      business,
      business_id,
      delta,
      detail: JSON.stringify(detail),
      signature: "",
    });
  }
  roundOrderInput(market, amount, price) {
    let marketInfo = this.markets.get(market);
    let amountRounded = Number(amount).toFixed(marketInfo.amount_precision);
    let priceRounded = Number(price).toFixed(marketInfo.price_precision);
    return { amount: amountRounded, price: priceRounded };
  }
  async createOrder(user_id, broker_id, account_id, market, order_side, order_type, amount, price, taker_fee, maker_fee) {
    if (!this.markets || this.markets.size == 0) {
      await this.connect();
    }
    if (!this.markets.has(market)) {
      throw new Error("invalid market " + market);
    }
    // TODO: round down? decimal?
    let marketInfo = this.markets.get(market);
    let baseTokenInfo = this.assets.get(marketInfo.base);
    let quoteTokenInfo = this.assets.get(marketInfo.quote);
    let amountRounded = Number(amount).toFixed(marketInfo.amount_precision);
    let priceRounded = Number(price).toFixed(marketInfo.price_precision);

    let signature = "";
    let order = {
      user_id,
      broker_id,
      account_id,
      market,
      order_side,
      order_type,
      amount: amountRounded,
      price: priceRounded,
      taker_fee,
      maker_fee,
      signature,
    };
    // TODO: better type check
    if (this.accounts.has(user_id) && (order_type == ORDER_TYPE_LIMIT || order_type == "LIMIT")) {
      // add signature for this order
      let account = this.accounts.get(user_id);
      order = signOrder(account, marketInfo, baseTokenInfo, quoteTokenInfo, order);
    }
    return order;
  }
  async orderPut(user_id, broker_id, account_id, market, order_side, order_type, amount, price, taker_fee, maker_fee) {
    const order = await this.createOrder(
      user_id,
      broker_id,
      account_id,
      market,
      order_side,
      order_type,
      amount,
      price,
      taker_fee,
      maker_fee
    );
    if (VERBOSE) {
      const { user_id, market, order_side: side, amount, price } = order;
      console.log("putLimitOrder", { user_id, market, side, amount, price });
    }
    return await this.client.OrderPut(order);
  }
  async batchOrderPut(market, reset, orders) {
    let order_reqs = [];
    for (const o of orders) {
      const { user_id, broker_id, account_id, market, order_side, order_type, amount, price, taker_fee, maker_fee } = o;
      order_reqs.push(
        await this.createOrder(user_id, broker_id, account_id, market, order_side, order_type, amount, price, taker_fee, maker_fee)
      );
    }
    return await this.client.batchOrderPut({
      market,
      reset,
      orders: order_reqs,
    });
  }

  async assetList() {
    return (await this.client.AssetList({})).asset_lists;
  }

  async marketList(): Promise<Map<string, any>> {
    const markets = (await this.client.MarketList({})).markets;
    let map = new Map();
    for (const m of markets) {
      map.set(m.name, m);
    }
    return map;
  }

  async orderDetail(market, order_id) {
    return await this.client.OrderDetail({ market, order_id });
  }

  async marketSummary(req) {
    let markets;
    if (req == null) {
      markets = [];
    } else if (typeof req === "string") {
      markets = [req];
    } else if (Array.isArray(req)) {
      markets = req;
    }
    let resp = (await this.client.MarketSummary({ markets })).market_summaries;
    if (typeof req === "string") {
      return resp.find(item => item.name === req);
    }
    return resp;
  }

  async reloadMarkets(from_scratch: boolean = false) {
    return await this.client.ReloadMarkets({ from_scratch });
  }

  async orderCancel(user_id, broker_id, account_id, market, order_id) {
    return await this.client.OrderCancel({ user_id, broker_id, account_id, market, order_id });
  }

  async orderCancelAll(user_id, market) {
    return await this.client.OrderCancelAll({ user_id, market });
  }

  async orderDepth(market, limit, interval) {
    return await this.client.OrderBookDepth({ market, limit, interval });
  }

  createTransferTx(from, brokerFrom, accountFrom, to, brokerTo, accountTo, asset, delta, memo) {
    let user_id = from;
    let signature = "";
    if (this.accounts.has(user_id)) {
      // add signature for this tx
      // let nonce = 0; // use 0 as nonce for now
    /*  let tx = new TransferTx({
        token_id: this.assets.get(asset).inner_id,
        amount: delta,
        from,

        from_nonce: nonce,
        to,
      });*/
      // signature = this.accounts.get(user_id).signHashPacked(tx.hash());
    }
    return {
      from,
      to,
      asset,
      delta,
      memo,
      signature,
      from_broker_id: brokerFrom,
      from_account_id: accountFrom,
      to_broker_id: brokerTo,
      to_account_id: accountTo,
    };
  }

  createWithdrawTx(account_id, asset, business, business_id, delta, detail) {
    let signature = "";
/*    if (this.accounts.has(account_id)) {
      let tx = new WithdrawTx({
        account_id,
        token_id: this.assets.get(asset).inner_id,
        amount: delta,
        nonce: 0,
        old_balance: 0, // TODO: Update `old_balance` with precision.
      });
      signature = this.accounts.get(account_id).signHashPacked(tx.hash());
    }*/
    return {
      user_id: account_id,
      asset,
      business,
      business_id,
      delta: -delta,
      detail: JSON.stringify(detail),
      signature: signature,
    };
  }

  async transfer(from, brokerFrom, accountFrom, to, brokerTo, accountTo, asset, delta, memo = "") {
    let tx = this.createTransferTx(from, brokerFrom, accountFrom, to, brokerTo, accountTo, asset, delta, memo);
    return await this.client.transfer(tx);
  }

  async withdraw(user_id, asset, business, business_id, delta, detail) {
    if (delta < 0) {
      throw new Error("Parameter `delta` must be positive in `withdraw` function");
    }
    let tx = this.createWithdrawTx(user_id, asset, business, business_id, delta, detail);
    return await this.client.BalanceUpdate(tx);
  }

  async debugDump() {
    return await this.client.DebugDump({});
  }

  async debugReset() {
    return await this.client.DebugReset({});
  }

  async debugReload() {
    return await this.client.DebugReload({});
  }

  async registerUser(user) {
    return await this.client.RegisterUser({
      user_id: user.id || user.user_id, // legacy reasons
      broker_id: user.broker_id,
      account_id: user.account_id,
      l1_address: user.l1_address,
      l2_pubkey: user.l2_pubkey,
    });
  }
}

// TODO: move else where
function signOrder(account, marketInfo, baseTokenInfo, quoteTokenInfo, order, checkPrec = true) {
  let { user_id, broker_id, account_id, market, order_side, order_type, amount, price, taker_fee, maker_fee } = order;

  let amountRounded = Number(amount).toFixed(marketInfo.amount_precision);
  let priceRounded = Number(price).toFixed(marketInfo.price_precision);
  if (checkPrec && !decimalEqual(amountRounded, amount)) {
    throw new Error("invalid amount precision");
  }
  if (checkPrec && !decimalEqual(priceRounded, price)) {
    throw new Error("invalid price precision");
  }

  let tokenBuy, tokenSell, totalSell, totalBuy;
  let amountFullPrec = fullPrec(amountRounded, marketInfo.amount_precision);
  let priceFullPrec = fullPrec(priceRounded, marketInfo.price_precision);
  let quoteFullPrec = amountFullPrec.mul(priceFullPrec);
  if (order_side == ORDER_SIDE_BID || order_side == "BID") {
    tokenBuy = baseTokenInfo.inner_id;
    tokenSell = quoteTokenInfo.inner_id;
    totalBuy = amountFullPrec;
    totalSell = quoteFullPrec;
  } else {
    tokenSell = baseTokenInfo.inner_id;
    tokenBuy = quoteTokenInfo.inner_id;
    totalSell = amountFullPrec;
    totalBuy = quoteFullPrec;
  }
  let orderInput = new OrderInput({
    tokenSell,
    tokenBuy,
    totalSell,
    totalBuy,
  });
  let signature = account.signHashPacked(orderInput.hash());

  return {
    user_id,
    broker_id,
    account_id,
    market,
    order_side,
    order_type,
    amount: amountRounded,
    price: priceRounded,
    taker_fee,
    maker_fee,
    signature,
  };
}

let defaultClient = new Client();
export { defaultClient, Client };
