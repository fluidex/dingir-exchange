import axios, { AxiosInstance } from "axios";
import * as _ from "lodash";

const REST_API_SERVER = "http://localhost:50053/api/exchange/panel";

class UserInfo {
  id: number;
  l1_address: string;
  l2_pubkey: string;
}

class KlineResult {
  s: string;
  t: number[];
  c: number[];
  o: number[];
  h: number[];
  l: number[];
  v: number[];
  nxt?: number;
}

class TickerResult {
  market: string;
  change: number;
  last: number;
  high: number;
  low: number;
  volume: number;
  quote_volume: number;
  from: number;
  to: number;
}

class MarketTrade {
  time: number;
  trade_id: number;
  price: string;
  amount: string;
  quote_amount: string;
  taker_side: number;
}

class OrderTrade {
  time: number;
  user_id: number;
  trade_id: number;
  order_id: number;
  price: string;
  amount: string;
  quote_amount: string;
  fee: string;
}

class RESTClient {
  client: AxiosInstance;

  constructor(server = process.env.REST_API_SERVER || REST_API_SERVER) {
    console.log("using REST API server: ", server);
    this.client = axios.create({
      baseURL: server,
      timeout: 5000,
    });
  }

  async get_user_by_addr(addr: string): Promise<UserInfo> {
    let resp = await this.client.get(`/user/${addr}`);
    if (resp.data.error) {
      console.log("error:", resp.data);
      return null;
    }
    return resp.data as unknown as UserInfo;
  }

  async internal_txs(
    user_id: number | string,
    params?: {
      limit?: number;
      offset?: number;
      start_time?: number;
      end_time?: number;
      order?: "asc" | "desc";
      side?: "from" | "to" | "both";
    }
  ) {
    let resp = await this.client.get(`/internal_txs/${user_id}`, {
      params: _.pickBy(params, _.identity),
    });
    if (resp.status === 200) {
      return resp.data;
    } else {
      throw new Error(`request failed with ${resp.status} ${resp.statusText}`);
    }
  }

  async recent_trades(market: string, limit?: number): Promise<MarketTrade[]> {
    let resp = await this.client.get(`/recenttrades/${market}`, {
      params: _.pickBy({ limit }, _.identity),
    });
    if (resp.status === 200) {
      return resp.data;
    } else {
      throw new Error(`recent_trades failed: ${resp.status}`);
    }
  }

  async order_trades(market: string, order_id: number): Promise<{ trades: OrderTrade[] }> {
    let resp = await this.client.get(`/ordertrades/${market}/${order_id}`);
    if (resp.status === 200) {
      return resp.data;
    } else {
      throw new Error(`order_trades failed: ${resp.status}`);
    }
  }

  async closed_orders(market: string, user_id: number, limit?: number, offset?: number) {
    let resp = await this.client.get(`/closedorders/${market}/${user_id}`, {
      params: _.pickBy({ limit, offset }, _.identity),
    });
    if (resp.status === 200) {
      return resp.data;
    } else {
      throw new Error(`closed_orders failed: ${resp.status}`);
    }
  }

  async ticker(interval: string, market: string): Promise<TickerResult> {
    let resp = await this.client.get(`/ticker_${interval}/${market}`);
    if (resp.status === 200) {
      return resp.data;
    } else {
      throw new Error(`ticker failed: ${resp.status}`);
    }
  }

  async kline_history(symbol: string, resolution: number, from: number, to: number): Promise<KlineResult> {
    let resp = await this.client.get(`/tradingview/history`, {
      params: { symbol, resolution, from, to },
    });
    if (resp.status === 200) {
      return resp.data;
    } else {
      throw new Error(`kline_history failed: ${resp.status}`);
    }
  }
}

let defaultRESTClient = new RESTClient();
export { defaultRESTClient, RESTClient };
