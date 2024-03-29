import axios, { AxiosInstance } from "axios";
import * as _ from "lodash";

const REST_API_SERVER = "http://localhost:50053/api/exchange/panel";

class UserInfo {
  id: number;
  l1_address: string;
  l2_pubkey: string;
}

class RESTClient {
  client: AxiosInstance;

  constructor(server = process.env.REST_API_SERVER || REST_API_SERVER) {
    console.log("using REST API server: ", server);
    this.client = axios.create({
      baseURL: server,
      timeout: 1000,
    });
  }

  async get_user_by_addr(addr: string): Promise<UserInfo> {
    let resp = await this.client.get(`/user/${addr}`);
    //console.log('user info', resp.data);
    if (resp.data.error) {
      console.log("error:", resp.data);
      return null;
    }
    let userInfo = resp.data as unknown as UserInfo;
    //console.log('raw', resp.data, 'result', userInfo);
    return userInfo;
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
}

let defaultRESTClient = new RESTClient();
export { defaultRESTClient, RESTClient };
