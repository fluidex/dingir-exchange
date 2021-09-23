import { Account } from "fluidex.js";
import { userId, base, quote, market, fee, ORDER_SIDE_BID, ORDER_SIDE_ASK, ORDER_TYPE_MARKET, ORDER_TYPE_LIMIT } from "./config"; // dotenv

let PROTO_PATH = __dirname + "/ordersigner.proto";

let grpc = require("@grpc/grpc-js");
let protoLoader = require("@grpc/proto-loader");
let packageDefinition = protoLoader.loadSync(PROTO_PATH, {
  keepCase: true,
  longs: String,
  enums: String,
  defaults: true,
  oneofs: true,
});
let ordersigner = grpc.loadPackageDefinition(packageDefinition).ordersigner;

import { defaultClient as client } from "./client";

// FIXME
// account11
const ethPrivKey = "0x7105836e5d903f273075ab50429c36c08afb0b786986c3612b522bf59bcecc20";
const acc = Account.fromPrivkey(ethPrivKey);
const uid = 3;
client.addAccount(uid, acc);

async function signOrder(call, callback) {
  let inputOrder = call.request;
  console.log({ inputOrder });
  //let accountID = call.accountID;
  let { user_id, market, order_side, order_type, amount, price, taker_fee, maker_fee } = inputOrder;
  // FIXME
  if (uid != user_id) {
    throw new Error("set user key first!");
  }
  //order_type = ORDER_TYPE_LIMIT;
  //order_side = ORDER_SIDE_BID;
  let signedOrder = await client.createOrder(user_id, market, order_side, order_type, amount, price, taker_fee, maker_fee);
  console.log({ signedOrder });
  //console.log(await client.client.orderPut(signedOrder));
  callback(null, { signature: signedOrder.signature });
}

/**
 * Starts an RPC server that receives requests for the Greeter service at the
 * sample server port
 */
function main() {
  let server = new grpc.Server();
  server.addService(ordersigner.OrderSigner.service, { signOrder: signOrder });
  server.bindAsync("0.0.0.0:50061", grpc.ServerCredentials.createInsecure(), () => {
    server.start();
  });
}

main();
