const Scalar = require('ffjavascript').Scalar;
import { Account, TxSignature } from './account';
import { hash } from './hash';

export enum OrderSide {
  Buy,
  Sell,
}

export class OrderInput {
  accountID: bigint = 0n;
  orderId: bigint = 0n;
  tokenBuy: bigint = 0n;
  tokenSell: bigint = 0n;
  totalSell: bigint = 0n;
  totalBuy: bigint = 0n;
  sig: TxSignature = null;
  side: OrderSide;
  constructor(data: Partial<OrderInput> = {}) {
    Object.assign(this, data);
  }
  static createEmpty(): OrderInput {
    let result = new OrderInput();
    return result;
  }
  hash(): bigint {
    // although there is no 'TxType.PlaceOrder' now, we can see it as a 'SignType'
    const magicHead = 4n; // TxType.PlaceOrder
    let data = hash([magicHead, this.tokenSell, this.tokenBuy, this.totalSell, this.totalBuy]);
    //data = hash([data, accountID, nonce]);
    // nonce and orderID seems redundant?
    //data = hash([data, this.accountID]);
    return data;
  }
  signWith(account: Account) {
    this.sig = account.signHash(this.hash());
  }
}

export class OrderState {
  orderInput: OrderInput;

  filledSell: bigint;
  filledBuy: bigint;

  get orderId(): bigint {
    return this.orderInput.orderId;
  }
  get tokenBuy(): bigint {
    return this.orderInput.tokenBuy;
  }
  get tokenSell(): bigint {
    return this.orderInput.tokenSell;
  }
  get totalSell(): bigint {
    return this.orderInput.totalSell;
  }
  get totalBuy(): bigint {
    return this.orderInput.totalBuy;
  }
  get side(): OrderSide {
    return this.orderInput.side;
  }
  isFilled(): boolean {
    return (
      (this.orderInput.side == OrderSide.Buy && this.filledBuy >= this.totalBuy) ||
      (this.orderInput.side == OrderSide.Sell && this.filledSell >= this.totalSell)
    );
  }
  static fromOrderInput(orderInput): OrderState {
    let result = new OrderState();
    result.orderInput = orderInput;
    result.filledBuy = 0n;
    result.filledSell = 0n;
    return result;
  }
  static createEmpty(): OrderState {
    return OrderState.fromOrderInput(OrderInput.createEmpty());
  }
  /**
   * Encode an order state object into an array
   * @returns {Array} Resulting array
   */
  private orderState2Array(): Array<bigint> {
    let data = Scalar.e(0);

    data = Scalar.add(data, this.orderId);
    data = Scalar.add(data, Scalar.shl(this.tokenBuy, 32));
    data = Scalar.add(data, Scalar.shl(this.tokenSell, 64));

    return [data, Scalar.e(this.filledSell), Scalar.e(this.filledBuy), Scalar.e(this.totalSell), Scalar.e(this.totalBuy)];
  }

  /**
   * Return the hash of an order state object
   * @returns {Scalar} Resulting hash
   */
  hash(): bigint {
    return hash(this.orderState2Array());
  }
}
