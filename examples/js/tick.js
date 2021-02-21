import { ORDER_SIDE_BID, ORDER_SIDE_ASK, market, userId } from "./config.mjs";
import { orderCancelAll } from "./client.mjs";
import { sleep, putLimitOrder, getRandomFloatAround } from "./util.mjs";
import axios from "axios";
async function main() {
  const url =
    "https://api.coinstats.app/public/v1/coins?skip=0&limit=5&currency=USD";
  let cnt = 0;
  while (true) {
    try {
      await sleep(1000);
      if (cnt % 300 == 0) {
        await orderCancelAll(userId, market);
      }
      const data = await axios.get(url);
      const ticker = data.data.coins.find(item => item.symbol == "ETH");
      const price = ticker.price;
      console.log("price", price);
      await putLimitOrder(
        ORDER_SIDE_BID,
        getRandomFloatAround(ticker.volume / 1e10, 0.5),
        getRandomFloatAround(ticker.price)
      );
      await sleep(1000);
      await putLimitOrder(
        ORDER_SIDE_ASK,
        getRandomFloatAround(ticker.volume / 1e10, 0.5),
        getRandomFloatAround(ticker.price)
      );
      cnt += 1;
    } catch (e) {
      console.log(e);
    }
  }
}

main().catch(console.log);
