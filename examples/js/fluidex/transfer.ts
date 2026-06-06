import { Account, TxSignature } from './account';
import { hash } from './hash';

export class TransferTx {
  token_id: bigint = 0n;
  amount: bigint = 0n;
  from: bigint = 0n;
  from_nonce: bigint = 0n;
  to: bigint = 0n;
  sig: TxSignature = null;
  constructor(data: Partial<TransferTx> = {}) {
    Object.assign(this, data);
  }
  hash(): bigint {
    const magicHead = 2n; // TxType.Transfer
    let data = hash([magicHead, this.token_id, this.amount, this.from, this.from_nonce, this.to]);
    return data;
  }
  signWith(account: Account) {
    this.sig = account.signHash(this.hash());
  }
}
