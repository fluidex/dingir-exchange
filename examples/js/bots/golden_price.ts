import { Client } from "../client";
class PriceBotParams {}
class PriceBot {
  client: Client;
  market: string;
  params: PriceBotParams;
  latestPrice: number;
  init(client: Client, market: string, params: PriceBotParams) {
    this.client = client;
    this.market = market;
    this.params = params;
  }
  // run every second
  tick() {}
  handleTrade(trade) {}
  handleOrderbookUpdate(orderbook) {}
  handleOrderEvent() {}
  getLatestPrice(): number {
    return this.latestPrice;
  }
  estimatePrice(): number {
    return 3;
  }
  getMyBalance() {}
}

export { PriceBot };
