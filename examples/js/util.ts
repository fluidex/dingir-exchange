import { userId, fee, ORDER_SIDE_BID, ORDER_SIDE_ASK, ORDER_TYPE_MARKET, ORDER_TYPE_LIMIT, VERBOSE } from "./config"; // dotenv

import Decimal from "decimal.js";
let gaussian = require("gaussian");
import { strict as assert } from "assert";
import axios from "axios";

export function decimalEqual(a, b): boolean {
  return new Decimal(a).equals(new Decimal(b));
}

export function assertDecimalEqual(result, gt) {
  assert(decimalEqual(result, gt), `${result} != ${gt}`);
}

export function decimalAdd(a, b) {
  return new Decimal(a).add(new Decimal(b));
}

export function getRandomFloat(min, max) {
  return Math.random() * (max - min) + min;
}
export function getRandomFloatAroundNormal(value, stddev_ratio = 0.02) {
  let distribution = gaussian(value, value * stddev_ratio);
  // Take a random sample using inverse transform sampling method.
  let sample = distribution.ppf(Math.random());
  return sample;
}
export function getRandomFloatAround(value, ratio = 0.05, abs = 0) {
  const eps1 = getRandomFloat(-abs, abs);
  const eps2 = getRandomFloat(-value * ratio, value * ratio);
  return value + eps1 + eps2;
}
export function getRandomInt(min, max) {
  min = Math.ceil(min);
  max = Math.floor(max);
  return Math.floor(Math.random() * (max - min)) + min;
}
export function getRandomElem<T>(arr: Array<T>): T {
  return arr[Math.floor(Math.random() * arr.length)];
}

export function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}
