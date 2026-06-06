import { assert } from 'console';
import { Account } from './account';
import { OrderInput, OrderSide } from './order';
import { packSignature } from './eddsa';

function testOrderSignature() {
  // https://github.com/Fluidex/rollup-state-manager/blob/e176ecada9080917e5b3a114cc1e6f373400c11c/src/account.rs#L501
  let acc = Account.fromMnemonic(
    'olympic comfort palm large heavy verb acid lion attract vast dash memory olympic syrup announce sure body cruise flip merge fabric frame question result',
    1,
  );
  console.log('bjj key', acc.bjjPubKey);
  assert(acc.bjjPubKey == '0x5d182c51bcfe99583d7075a7a0c10d96bef82b8a059c4bf8c5f6e7124cf2bba3');
  let order = new OrderInput({
    accountID: 1n,
    orderId: 1n,
    side: OrderSide.Buy,
    tokenBuy: 1n,
    tokenSell: 2n,
    totalBuy: 999n,
    totalSell: 888n,
  });
  console.log('hash', order.hash().toString());
  assert(order.hash().toString() == '8056692562185768785417295010793063162660984530596417435073781442183268221458');
  let sig = acc.signHashPacked(order.hash());
  console.log('sig', sig);
  assert(
    sig ==
      '57e6cf2e5b8db0a90072d15bc49e737df2e10746e5f531a24d72557894f2c90964d77726505232a4c9e7631eed22ad9210dce2858642fdfe3e58e95d44b99002',
  );
  console.log('sig len', sig.length);
}
testOrderSignature();
