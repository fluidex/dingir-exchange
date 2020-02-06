Object.entries(require("./grpc_client.js")).forEach(([name, exported]) => {
  global[name] = exported;
});

var Decimal = require("decimal.js");
const assert = require("assert").strict;

const user_id = 3;
const DEPOSIT_ID = Math.floor(Date.now() / 1000);
const base = "ETH";
const quote = "BTC";
const market = `${base}/${quote}`;
const ask = 0;
const bid = 1;
const fee = "0";

async function pretty_print(obj) {
  console.dir(await obj, { depth: null });
}

function float_equal(result, gt) {
  assert(new Decimal(result).equals(new Decimal(gt)), `${result} != ${gt}`);
}

async function ensure_asset_valid() {
  const balance2 = await balance_query(user_id);
  float_equal(balance2.BTC.available, "100");
  float_equal(balance2.BTC.freeze, "0");
  float_equal(balance2.ETH.available, "50");
  float_equal(balance2.ETH.freeze, "0");
}

async function ensure_asset_zero() {
  const balance1 = await balance_query(user_id);
  float_equal(balance1.BTC.available, "0");
  float_equal(balance1.BTC.freeze, "0");
  float_equal(balance1.ETH.available, "0");
  float_equal(balance1.ETH.freeze, "0");
}

async function setup_asset() {
  await balance_update(user_id, "BTC", "deposit", DEPOSIT_ID, "100.0", {
    key: "value"
  });
  await balance_update(user_id, "ETH", "deposit", DEPOSIT_ID + 1, "50.0", {
    key: "value"
  });
}

async function trade_test() {
  const bid_order = await order_put_limit(
    user_id,
    market,
    bid,
    /*amount*/ "10",
    /*price*/ "1.1",
    fee,
    fee,
    "source"
  );
  const ask_order = await order_put_limit(
    user_id,
    market,
    ask,
    /*amount*/ "4",
    /*price*/ "1.0",
    fee,
    fee,
    "src"
  );

  const bid_order_pending = await order_pending_detail(
    market,
    /*order_id*/ bid_order.id
  );
  float_equal(bid_order_pending.left, "6");

  //TODO
  await assert.rejects(async () => {
    const ask_order_pending = await order_pending_detail(
      market,
      /*order_id*/ ask_order.id
    );
  }, /invalid order_id/);
  //assert.equal(ask_order_pending, null);

  // should check trade price is 1.1 rather than 1.0 here.

  const summary = (await market_summary(market))[0];
  float_equal(summary.bid_amount, "6");
  assert.equal(summary.bid_count, 1);

  const depth = await order_depth(market, 100, /*not merge*/ "0");
  assert.deepEqual(depth, { asks: [], bids: [{ price: "1.1", amount: "6" }] });

  console.log("trade_test passed!");
}

async function order_test() {
  const order = await order_put_limit(
    user_id,
    market,
    bid,
    /*amount*/ "10",
    /*price*/ "1.1",
    fee,
    fee,
    "src"
  );
  const balance3 = await balance_query(user_id);
  float_equal(balance3.BTC.available, "89");
  float_equal(balance3.BTC.freeze, "11");

  const order_pending = await order_pending_detail(
    market,
    /*order_id*/ order.id
  );
  assert.deepEqual(order_pending, order);

  const summary = (await market_summary(market))[0];
  float_equal(summary.bid_amount, "10");
  assert.equal(summary.bid_count, 1);

  const depth = await order_depth(market, 100, /*not merge*/ "0");
  assert.deepEqual(depth, { asks: [], bids: [{ price: "1.1", amount: "10" }] });

  await order_cancel(user_id, market, 1);
  const balance4 = await balance_query(user_id);
  float_equal(balance4.BTC.available, "100");
  float_equal(balance4.BTC.freeze, "0");

  console.log("order_test passed");
}

async function info_list() {
  console.log(await asset_list([]));
  console.log(await market_list([]));
}

async function simple_test() {
  await ensure_asset_zero();
  await setup_asset();
  await ensure_asset_valid();
  await order_test();
  await trade_test();
}

async function naive_example() {
  console.log(await asset_list());
  console.log(await balance_query(1));
  console.log(await balance_update(1, "BTC", "1.3", "deposit", 15, {}));
  console.log(await balance_query(1));
}

async function main() {
  try {
    //console.log(await balance_query(user_id));
    //await naive_example();
    //await setup_asset();
    await simple_test();
  } catch (error) {
    console.error("Catched error:", error);
  }
}
main();
