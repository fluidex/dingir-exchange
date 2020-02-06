const caller = require("grpc-caller");
const grpc = require("grpc");
const file = "../../proto/exchange/matchengine.proto";
const load = { keepCase: "true", defaults: "true" };
const client = caller("0.0.0.0:50051", { file, load }, "Matchengine");

async function balance_query(user_id) {
  return (await client.BalanceQuery({ user_id: user_id })).balances;
}

async function balance_update(
  user_id,
  asset,
  business,
  business_id,
  delta,
  detail
) {
  return await client.BalanceUpdate({
    user_id,
    asset,
    business,
    business_id,
    delta,
    detail: JSON.stringify(detail)
  });
}

async function order_put_limit(
  user_id,
  market,
  order_side,
  amount,
  price,
  taker_fee,
  maker_fee,
  category
) {
  return await client.OrderPut({
    user_id,
    market,
    order_side,
    amount,
    price,
    taker_fee,
    maker_fee,
    category
  });
}

async function asset_list() {
  return (await client.AssetList({})).asset_lists;
}

async function order_pending_detail(market, order_id) {
  return await client.OrderDetail({ market, order_id });
}

async function market_summary(market) {
  return (await client.MarketSummary({ market: [market] })).market_summaries;
}

async function order_cancel(user_id, market, order_id) {
  return await client.OrderCancel({ user_id, market, order_id });
}

async function order_depth(market, limit, interval) {
  return await client.OrderBookDepth({ market, limit, interval });
}

module.exports = {
  balance_query,
  order_put_limit,
  balance_update,
  asset_list,
  order_pending_detail,
  market_summary,
  order_cancel,
  order_depth
};
