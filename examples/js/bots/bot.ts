interface Bot {
  tick: (balance, oldOrders) => Promise<{ reset; orders }>;
}
export { Bot };
