use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use std::str::FromStr;

pub use babyjubjub_rs::{Point, Signature};
pub use ff::{from_hex, to_hex};
pub use ff::{Field, PrimeField, PrimeFieldRepr};
pub use num_bigint::BigInt;
pub use poseidon_rs::Fr;
pub use rust_decimal::Decimal;

lazy_static! {
    //pub static ref POSEIDON_PARAMS: poseidon_rs::Constants = poseidon_rs::load_constants();
    pub static ref POSEIDON_HASHER: poseidon_rs::Poseidon = poseidon_rs::Poseidon::new();
}

pub fn hash(inputs: &[Fr]) -> Fr {
    (&POSEIDON_HASHER).hash(inputs.to_vec()).unwrap()
}

// TODO: these functions needed to be rewrite...

pub fn u32_to_fr(x: u32) -> Fr {
    Fr::from_str(&format!("{}", x)).unwrap()
}

pub fn u64_to_fr(x: u64) -> Fr {
    Fr::from_repr(poseidon_rs::FrRepr::from(x)).unwrap()
}

pub fn bigint_to_fr(x: BigInt) -> Fr {
    let mut s = x.to_str_radix(16);
    if s.len() % 2 != 0 {
        // convert "f" to "0f"
        s.insert(0, '0');
    }
    from_hex(&s).unwrap()
}

pub fn str_to_fr(x: &str) -> Fr {
    if x.starts_with("0x") {
        vec_to_fr(&hex::decode(x.trim_start_matches("0x")).unwrap()).unwrap()
    } else {
        let i = BigInt::from_str(x).unwrap();
        bigint_to_fr(i)
    }
}

pub fn vec_to_fr(arr: &[u8]) -> Result<Fr> {
    if arr.len() > 32 {
        anyhow::bail!("invalid vec len for fr");
    }
    let mut repr = <Fr as PrimeField>::Repr::default();

    // prepad 0
    let mut buf = arr.to_vec();
    let required_length = repr.as_ref().len() * 8;
    buf.reverse();
    buf.resize(required_length, 0);
    buf.reverse();

    repr.read_be(&buf[..])?;
    Ok(Fr::from_repr(repr)?)
}

pub fn fr_to_u32(f: &Fr) -> u32 {
    fr_to_string(f).parse::<u32>().unwrap()
}

pub fn fr_to_i64(f: &Fr) -> i64 {
    fr_to_string(f).parse::<i64>().unwrap()
}

pub fn fr_to_bigint(elem: &Fr) -> BigInt {
    BigInt::parse_bytes(to_hex(elem).as_bytes(), 16).unwrap()
}

pub fn fr_to_string(elem: &Fr) -> String {
    fr_to_bigint(&elem).to_str_radix(10)
}

pub fn fr_to_decimal(f: &Fr, scale: u32) -> Decimal {
    Decimal::new(fr_to_i64(f), scale)
}

// big endian
pub fn fr_to_vec(f: &Fr) -> Vec<u8> {
    let repr = f.into_repr();
    let required_length = repr.as_ref().len() * 8;
    let mut buf: Vec<u8> = Vec::with_capacity(required_length);
    repr.write_be(&mut buf).unwrap();
    buf
}

pub fn fr_to_bool(f: &Fr) -> Result<bool> {
    if f.is_zero() {
        Ok(false)
    } else if f == &Fr::one() {
        Ok(true)
    } else {
        Err(anyhow!("invalid fr"))
    }
}
