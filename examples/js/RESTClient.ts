import axios, { AxiosInstance } from "axios";
import * as _ from "lodash";

const REST_API_SERVER = "http://localhost:50053/restapi";

class RESTClient {
  client: AxiosInstance;

  constructor(server = process.env.REST_API_SERVER || REST_API_SERVER) {
    console.log("using REST API server: ", server);
    this.client = axios.create({
      baseURL: server,
      timeout: 1000
    });
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
      params: _.pickBy(params, _.identity)
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
