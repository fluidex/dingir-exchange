import * as Dotenv from "dotenv";
import { inspect } from "util";
import ID from "./tests/ids";
Dotenv.config();

export const VERBOSE = !!process.env.VERBOSE;

// constants
export const ORDER_SIDE_ASK = 0;
export const ORDER_SIDE_BID = 1;
export const ORDER_TYPE_LIMIT = 0;
export const ORDER_TYPE_MARKET = 1;

// fake data
export const userId = ID.userID[0];
export const brokerId = ID.brokerID[0];
export const accountId = ID.accountID[0];
export const base = "ETH";
export const quote = "USDT";
export const market = `${base}_${quote}`;
export const fee = "0";

// global config
inspect.defaultOptions.depth = null;
