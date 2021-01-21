import Dotenv from "dotenv";
Dotenv.config();

export const VERBOSE = !!process.env.VERBOSE;

// constants
export const ORDER_SIDE_ASK = 0;
export const ORDER_SIDE_BID = 1;
export const ORDER_TYPE_LIMIT = 0;
export const ORDER_TYPE_MARKET = 1;

// fake data
export const userId = 3;
export const base = "ETH";
export const quote = "USDT";
export const market = `${base}_${quote}`;
export const fee = "0";

// global config
import { inspect } from "util";
inspect.defaultOptions.depth = null;
