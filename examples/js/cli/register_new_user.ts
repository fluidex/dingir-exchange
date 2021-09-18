import { getTestAccount } from "../accounts";
import { Account } from "fluidex.js";
import { defaultClient } from "../client";
async function main() {
  let acc = Account.fromPrivkey(getTestAccount(15).priv_key);
  console.log(getTestAccount(15).priv_key);
  let resp = await defaultClient.registerUser({
    user_id: 0, // discard in server side
    l1_address: acc.ethAddr,
    l2_pubkey: acc.bjjPubKey,
  });
  console.log(resp);
}
main();
