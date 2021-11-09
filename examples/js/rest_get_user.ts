import { Account } from "fluidex.js";
import { defaultClient as grpcClient } from "./client";
import { defaultRESTClient as restClient } from "./RESTClient";
import { sleep } from "./util";
import { strict as assert } from "assert";

async function initUser() {
  console.log("initUser BEGIN");

  const mnemonic =
    "sound select report rug run cave provide index grief foster bar someone garage donate nominee crew once oil sausage flight tail holiday style afford";
  const account = Account.fromMnemonic(mnemonic);
  const userInfo = {
    user_id: 0,
    l1_address: account.ethAddr.toLowerCase(),
    l2_pubkey: account.bjjPubKey.toLowerCase(),
  };
  if (!(await restClient.get_user(userInfo.l1_address))) {
    await grpcClient.registerUser(userInfo);
    await sleep(2000);
  }

  console.log("initUser END");

  return userInfo;
}

async function testGetUser(userInfo) {
  console.log("test get user by l1 address");
  let userResult = await restClient.get_user(userInfo.l1_address);
  assert.equal(userInfo.l1_address, userResult.l1_address);
  assert.equal(userInfo.l2_pubkey, userResult.l2_pubkey);
  userInfo.user_id = userResult.id;

  console.log("test get user by l2 public key");
  userResult = await restClient.get_user(userInfo.l2_pubkey);
  assert.equal(userInfo.l1_address, userResult.l1_address);
  assert.equal(userInfo.l2_pubkey, userResult.l2_pubkey);
  assert.equal(userInfo.user_id, userResult.id);

  console.log("test get user by id");
  userResult = await restClient.get_user(userInfo.user_id.toString());
  assert.equal(userInfo.l1_address, userResult.l1_address);
  assert.equal(userInfo.l2_pubkey, userResult.l2_pubkey);
  assert.equal(userInfo.user_id, userResult.id);
}

async function main() {
  try {
    const userInfo = await initUser();
    await testGetUser(userInfo);
  } catch (error) {
    console.error("Caught error:", error);
    process.exit(1);
  }
}

main();
