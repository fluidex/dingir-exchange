import caller from "grpc-caller";
const file = "../../proto/exchange/matchengine.proto";
const load = { keepCase: "true", defaults: "true" };
const client = caller("0.0.0.0:50051", { file, load }, "Matchengine");

export async function balanceQuery(user_id) {
  const balances = (await client.BalanceQuery({ user_id: user_id })).balances;
  let result = {};
  for (const entry of balances) {
    result[entry.asset_name] = entry;
  }
  return result;
}

export async function balanceUpdate(
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

export async function orderPut(
  user_id,
  market,
  order_side,
  order_type,
  amount,
  price,
  taker_fee,
  maker_fee
) {
  return await client.OrderPut({
    user_id,
    market,
    order_side,
    order_type,
    amount,
    price,
    taker_fee,
    maker_fee
  });
}

export async function assetList() {
  return (await client.AssetList({})).asset_lists;
}

export async function orderDetail(market, order_id) {
  return await client.OrderDetail({ market, order_id });
}

export async function marketSummary(market) {
  return (await client.MarketSummary({ market: [market] })).market_summaries;
}

export async function orderCancel(user_id, market, order_id) {
  return await client.OrderCancel({ user_id, market, order_id });
}

export async function orderDepth(market, limit, interval) {
  return await client.OrderBookDepth({ market, limit, interval });
}

export async function debugReset() {
  return await client.DebugReset({});
}


export async function debugReload() {
  return await client.DebugReload({});
}
