import { ORDER_SIDE_BID, ORDER_SIDE_ASK } from "./config.mjs";
import { sleep, putLimitOrder, getRandomFloatAround } from "./util.mjs";
import axios from "axios";
async function main() {
  const url =
    "https://api.coinstats.app/public/v1/coins?skip=0&limit=5&currency=USD";
  while (true) {
    try {
      await sleep(1000);
      const data = await axios.get(url);
      const ticker = data.data.coins.find(item => item.symbol == "ETH");
      const price = ticker.price;
      console.log("price", price);
      await putLimitOrder(
        ORDER_SIDE_ASK,
        getRandomFloatAround(ticker.volume / 1e10, 0.5),
        getRandomFloatAround(ticker.price)
      );
      await putLimitOrder(
        ORDER_SIDE_BID,
        getRandomFloatAround(ticker.volume / 1e10, 0.5),
        getRandomFloatAround(ticker.price)
      );
    } catch (e) {
      console.log(e);
    }
  }
}

main().catch(console.log);
