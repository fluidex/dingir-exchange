/* modified from circomlib/src/eddsa.js */

import * as createBlakeHash from 'blake-hash';
import { Scalar, F1Field, ffutils } from './ffjs';
import { babyJub, poseidon } from 'circomlib';

function pruneBuffer(_buff) {
  const buff = Buffer.from(_buff);
  buff[0] = buff[0] & 0xf8;
  buff[31] = buff[31] & 0x7f;
  buff[31] = buff[31] | 0x40;
  return buff;
}

function prv2bigint(prv) {
  const sBuff = pruneBuffer(createBlakeHash('blake512').update(prv).digest().slice(0, 32));
  let s = ffutils.leBuff2int(sBuff);
  return Scalar.shr(s, 3);
}

function prv2pub(prv) {
  const A = babyJub.mulPointEscalar(babyJub.Base8, prv2bigint(prv));
  return A;
}

function signWithHasher(prv, msg, hasher) {
  const h1 = createBlakeHash('blake512').update(prv).digest();
  const sBuff = pruneBuffer(h1.slice(0, 32));
  const s = ffutils.leBuff2int(sBuff);
  const A = babyJub.mulPointEscalar(babyJub.Base8, Scalar.shr(s, 3));

  const msgBuff = ffutils.leInt2Buff(msg, 32);
  const rBuff = createBlakeHash('blake512')
    .update(Buffer.concat([h1.slice(32, 64), msgBuff]))
    .digest();
  let r = ffutils.leBuff2int(rBuff);
  const Fr = new F1Field(babyJub.subOrder);
  r = Fr.e(r);
  const R8 = babyJub.mulPointEscalar(babyJub.Base8, r);
  const hm = hasher([R8[0], R8[1], A[0], A[1], msg]);
  const S = Fr.add(r, Fr.mul(hm, s));
  return {
    R8: R8,
    S: S,
  };
}

function verifyWithHasher(msg, sig, A, hasher) {
  // Check parameters
  if (typeof sig != 'object') return false;
  if (!Array.isArray(sig.R8)) return false;
  if (sig.R8.length != 2) return false;
  if (!babyJub.inCurve(sig.R8)) return false;
  if (!Array.isArray(A)) return false;
  if (A.length != 2) return false;
  if (!babyJub.inCurve(A)) return false;
  if (sig.S >= babyJub.subOrder) return false;

  const hm = hasher([sig.R8[0], sig.R8[1], A[0], A[1], msg]);

  const Pleft = babyJub.mulPointEscalar(babyJub.Base8, sig.S);
  let Pright = babyJub.mulPointEscalar(A, Scalar.mul(hm, 8));
  Pright = babyJub.addPoint(sig.R8, Pright);

  if (!babyJub.F.eq(Pleft[0], Pright[0])) return false;
  if (!babyJub.F.eq(Pleft[1], Pright[1])) return false;
  return true;
}

function packSignature(sig) {
  const R8p = babyJub.packPoint(sig.R8);
  const Sp = ffutils.leInt2Buff(sig.S, 32);
  return Buffer.concat([R8p, Sp]);
}

function unpackSignature(sigBuff) {
  return {
    R8: babyJub.unpackPoint(sigBuff.slice(0, 32)),
    S: ffutils.leBuff2int(sigBuff.slice(32, 64)),
  };
}

export { prv2bigint, prv2pub, signWithHasher, verifyWithHasher, packSignature, unpackSignature, pruneBuffer };
