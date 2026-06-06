import * as ethers from 'ethers';
import * as assert from 'assert';
import * as eddsa from './eddsa';
import { hash } from './hash';
const babyJub = require('circomlib').babyJub;
const keccak256 = require('js-sha3').keccak256;
import { L2Account, Account, get_CREATE_L2_ACCOUNT_MSG, recoverPublicKeyFromSignature } from './account';

async function TestRecoverPublicKeyAndAddress() {
  const MNEMONIC = 'radar blur cabbage chef fix engine embark joy scheme fiction master release';
  const wallet = ethers.Wallet.fromMnemonic(MNEMONIC, null);
  const message = get_CREATE_L2_ACCOUNT_MSG(null);
  const expectedAddress = await wallet.getAddress();
  const expectedPublicKey = wallet._signingKey().publicKey;
  const signature = await wallet.signMessage(message);

  // console.log("Address:", expectedAddress);
  // console.log("PublicKey:", expectedPublicKey);
  // console.log("Message:", message);
  // console.log("Signature:", signature);

  const pk = recoverPublicKeyFromSignature(message, signature);
  assert(pk == expectedPublicKey, 'PublicKey mismatch');
  const addr = ethers.utils.computeAddress(pk);
  assert(addr == expectedAddress, 'Address mismatch');
}
const privKey = '0x0b22f852cd07386bce533f2038821fdcebd9c5ced9e3cd51e3a05d421dbfd785';
function testL2Sign() {
  const acc = Account.fromPrivkey(privKey);
  // 0x7b70843a42114e88149e3961495c03f9a41292c8b97bd1e2026597d185478293
  console.log(acc.bjjPubKey);
}

async function ethersSign() {
  let signer = new ethers.Wallet(privKey);
  const message = get_CREATE_L2_ACCOUNT_MSG(null);
  const signature = await signer.signMessage(message);
  // 0x9982364bf709fecdf830a71f417182e3a7f717a6363180ff33784e2823935f8b55932a5353fb128fc7e3d6c4aed57138adce772ce594338a8f4985d6668627b31c
  console.log(signature);
  const acc = Account.fromSignature(signature);
  // 0x7b70843a42114e88149e3961495c03f9a41292c8b97bd1e2026597d185478293
  console.log(acc.bjjPubKey);
}

function TestL2AccountKeyAndSign() {
  /*
  const wallet = ethers.Wallet.createRandom();
  const msgHash = ethers.utils.hashMessage(get_CREATE_L2_ACCOUNT_MSG(null));
  const signature = ethers.utils.joinSignature(wallet._signingKey().signDigest(msgHash));
  const seed = ethers.utils.arrayify(signature).slice(0, 32);
  */
  const seed = ethers.utils.arrayify('0x87b34b2b842db0cc945659366068053f325ff227fd9c6788b2504ac2c4c5dc2a');
  const account = new L2Account(seed);
  assert(account.bjjPubKey == '0xa59226beb68d565521497d38e37f7d09c9d4e97ac1ebc94fba5de524cb1ca4a0');
  assert(account.ax.toString(16) == '1fce25ec2e7eeec94079ec7866a933a8b21f33e0ebd575f3001d62d19251d455');
  assert(account.ay.toString(16) == '20a41ccb24e55dba4fc9ebc17ae9d4c9097d7fe3387d492155568db6be2692a5');
  assert(account.sign.toString(16) == '1');
  const sig = account.signHash(1357924680n);
  assert(sig.R8x.toString(10) == '15679698175365968671287592821268512384454163537665670071564984871581219397966');
  assert(sig.R8y.toString(10) == '1705544521394286010135369499330220710333064238375605681220284175409544486013');
  assert(sig.S.toString(10) == '2104729104368328243963691045555606467740179640947024714099030450797354625308');
  const packedSig = account.signHashPacked(1357924680n);
  assert(
    // @ts-ignore
    packedSig.toString('hex') ==
      '7ddc5c6aadf5e80200bd9f28e9d5bf932cbb7f4224cce0fa11154f4ad24dc5831c295fb522b7b8b4921e271bc6b265f4d7114fbe9516d23e69760065053ca704',
  );
  //console.log(packedSig.toString('hex'));
  //console.log(eddsa.unpackSignature(packedSig));
}

function TestL2SigVerify() {
  // copied from TestL2AccountKeyAndSign
  const pubkey = 'a59226beb68d565521497d38e37f7d09c9d4e97ac1ebc94fba5de524cb1ca4a0';
  const msg = 1357924680n;
  const sig =
    '7ddc5c6aadf5e80200bd9f28e9d5bf932cbb7f4224cce0fa11154f4ad24dc5831c295fb522b7b8b4921e271bc6b265f4d7114fbe9516d23e69760065053ca704';
  const pubkeyPoint = babyJub.unpackPoint(Buffer.from(pubkey, 'hex'));
  assert(eddsa.verifyWithHasher(msg, eddsa.unpackSignature(Buffer.from(sig, 'hex')), pubkeyPoint, hash));
}

async function main() {
  await TestL2SigVerify();
  await TestL2AccountKeyAndSign();
  await TestRecoverPublicKeyAndAddress();
  await ethersSign();
  await testL2Sign();
}

main();
