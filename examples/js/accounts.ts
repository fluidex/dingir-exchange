let accounts = require("fs").readFileSync("./accounts.jsonl", "utf-8").split("\n").filter(Boolean).map(JSON.parse);

export function getTestAccount(id) {
  let a = accounts.filter(account => {
    return account.account_id == id;
  });
  return a[0];
}
