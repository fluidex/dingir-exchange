import axios from "axios";
async function main() {
  console.log("active orders:");
  console.log(
    (await axios.get("http://localhost:8765/api/orders/ETH_USDT/3")).data
  );
  console.log("closed orders:");
  console.log(
    (await axios.get("http://localhost:8765/restapi/closedorders/ETH_USDT/3"))
      .data
  );
}
main().catch(console.log);
