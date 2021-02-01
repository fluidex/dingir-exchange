import axios from "axios";
async function main() {
  const server = "0.0.0.0";
  console.log("closed orders:");
  console.log(
    (await axios.get(`http://${server}:8765/restapi/closedorders/ETH_USDT/3`))
      .data
  );
  console.log("active orders:");
  console.log(
    (await axios.get(`http://${server}:8765/api/orders/ETH_USDT/3`)).data
  );
  console.log("market ticker:");
  console.log(
    (await axios.get(`http://${server}:8765/restapi/ticker_24h/ETH_USDT`)).data
  );
}
main().catch(console.log);
