import { Account, TxSignature } from './account';
import { hash } from './hash';

export class WithdrawTx {
  account_id: bigint = 0n;
  token_id: bigint = 0n;
  amount: bigint = 0n;
  nonce: bigint = 0n;
  old_balance: bigint = 0n;
  sig: TxSignature = null;
  constructor(data: Partial<WithdrawTx> = {}) {
    Object.assign(this, data);
  }
  hash(): bigint {
    const magicHead = 3n; // TxType.Withdraw
    return hash([magicHead, this.account_id, this.token_id, this.amount, this.nonce, this.old_balance]);
  }
  signWith(account: Account) {
    this.sig = account.signHash(this.hash());
  }
}
