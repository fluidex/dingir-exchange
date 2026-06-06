use ff::{Field, PrimeField, PrimeFieldRepr};
pub use num_bigint::BigInt;
use num_traits::{Pow, ToPrimitive};
use rust_decimal::Decimal;
use std::io;
use std::str::FromStr;
use std::sync::LazyLock;

pub use babyjubjub_rs::{Point as Pubkey, Signature, decompress_point, decompress_signature, verify};
pub use poseidon_rs::Fr;

/// Global Poseidon hasher instance.
pub static POSEIDON_HASHER: LazyLock<poseidon_rs::Poseidon> = LazyLock::new(poseidon_rs::Poseidon::new);

#[derive(Debug, thiserror::Error)]
pub enum FrExtError {
    #[error("invalid value for bool")]
    InvalidBool,
    #[error("invalid slice length for Fr")]
    InvalidLength,
    #[error(transparent)]
    BufferError(#[from] io::Error),
    #[error(transparent)]
    PrimeFieldDecodingError(#[from] ff::PrimeFieldDecodingError),
}

type Result<T, E = FrExtError> = std::result::Result<T, E>;

pub trait FrExt: Sized {
    fn shl(&self, x: u32) -> Self;
    fn sub(&self, b: &Fr) -> Self;
    fn add(&self, b: &Fr) -> Self;
    fn hash(inputs: &[Self]) -> Self;
    fn from_u32(x: u32) -> Self;
    fn from_u64(x: u64) -> Self;
    fn from_bigint(x: BigInt) -> Self;
    fn from_str(x: &str) -> Self;
    fn from_slice(slice: &[u8]) -> Result<Self>;
    fn to_u32(&self) -> u32;
    fn to_i64(&self) -> i64;
    fn to_bigint(&self) -> BigInt;
    fn to_decimal_string(&self) -> String;
    fn to_decimal(&self, scale: u32) -> Decimal;
    fn to_vec_be(&self) -> Vec<u8>;
    fn to_bool(&self) -> Result<bool>;
}

impl FrExt for Fr {
    fn shl(&self, x: u32) -> Self {
        let mut repr = self.into_repr();
        repr.shl(x);
        Fr::from_repr(repr).unwrap()
    }

    fn sub(&self, b: &Fr) -> Self {
        let mut r = *self;
        r.sub_assign(b);
        r
    }

    fn add(&self, b: &Fr) -> Self {
        let mut r = *self;
        r.add_assign(b);
        r
    }

    fn hash(inputs: &[Fr]) -> Fr {
        (&POSEIDON_HASHER).hash(inputs.to_vec()).unwrap()
    }

    fn from_u32(x: u32) -> Self {
        PrimeField::from_str(&format!("{}", x)).unwrap()
    }

    fn from_u64(x: u64) -> Self {
        Fr::from_repr(poseidon_rs::FrRepr::from(x)).unwrap()
    }

    fn from_bigint(x: BigInt) -> Self {
        let mut s = x.to_str_radix(16);
        if s.len() % 2 != 0 {
            s.insert(0, '0');
        }
        ff::from_hex(&s).unwrap()
    }

    fn from_str(x: &str) -> Self {
        if x.starts_with("0x") {
            Self::from_slice(&hex::decode(x.trim_start_matches("0x")).unwrap()).unwrap()
        } else {
            let i = BigInt::from_str(x).unwrap();
            Self::from_bigint(i)
        }
    }

    fn from_slice(slice: &[u8]) -> Result<Self> {
        if slice.len() > 32 {
            return Err(FrExtError::InvalidLength);
        }
        let mut repr = <Fr as PrimeField>::Repr::default();

        let required_length = repr.as_ref().len() * 8;
        let mut buf = slice.to_vec();
        buf.reverse();
        buf.resize(required_length, 0);
        buf.reverse();

        repr.read_be(&buf[..])?;
        Ok(Fr::from_repr(repr)?)
    }

    fn to_u32(&self) -> u32 {
        Self::to_decimal_string(self).parse::<u32>().unwrap()
    }

    fn to_i64(&self) -> i64 {
        Self::to_decimal_string(self).parse::<i64>().unwrap()
    }

    fn to_bigint(&self) -> BigInt {
        BigInt::parse_bytes(ff::to_hex(self).as_bytes(), 16).unwrap()
    }

    fn to_decimal_string(&self) -> String {
        Self::to_bigint(self).to_str_radix(10)
    }

    fn to_decimal(&self, scale: u32) -> Decimal {
        Decimal::new(Self::to_i64(self), scale)
    }

    fn to_vec_be(&self) -> Vec<u8> {
        let repr = self.into_repr();
        let required_length = repr.as_ref().len() * 8;
        let mut buf: Vec<u8> = Vec::with_capacity(required_length);
        repr.write_be(&mut buf).unwrap();
        buf
    }

    fn to_bool(&self) -> Result<bool> {
        if self.is_zero() {
            Ok(false)
        } else if self == &Fr::one() {
            Ok(true)
        } else {
            Err(FrExtError::InvalidBool)
        }
    }
}

pub trait DecimalExt {
    fn to_u64(&self, prec: u32) -> u64;
    fn to_fr(&self, prec: u32) -> Fr;
}

impl DecimalExt for Decimal {
    fn to_u64(&self, prec: u32) -> u64 {
        let prec_mul = Decimal::new(10, 0).pow(prec as u64);
        let adjusted = self * prec_mul;
        ToPrimitive::to_u64(&adjusted.floor()).unwrap()
    }

    fn to_fr(&self, prec: u32) -> Fr {
        Fr::from_u64(DecimalExt::to_u64(self, prec))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PubkeyExtError {
    #[error(transparent)]
    HexDecode(#[from] hex::FromHexError),
    #[error("invalid pubkey packed length {0} instead of 32")]
    InvalidLength(usize),
    #[error("{0}")]
    InvalidPoint(String),
}

pub trait PubkeyExt: Sized {
    fn from_str(pubkey: &str) -> std::result::Result<Self, PubkeyExtError>;
}

impl PubkeyExt for Pubkey {
    fn from_str(pubkey: &str) -> std::result::Result<Self, PubkeyExtError> {
        let pubkey = pubkey.trim_start_matches("0x");
        let pubkey_packed = hex::decode(pubkey)?;
        let arr: [u8; 32] = pubkey_packed
            .try_into()
            .map_err(|v: Vec<u8>| PubkeyExtError::InvalidLength(v.len()))?;
        decompress_point(arr).map_err(PubkeyExtError::InvalidPoint)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SignatureExtError {
    #[error(transparent)]
    HexDecode(#[from] hex::FromHexError),
    #[error("invalid signature packed length {0} instead of 64")]
    InvalidLength(usize),
    #[error("{0}")]
    InvalidPoint(String),
}

pub trait SignatureExt: Sized {
    fn from_str(signature: &str) -> std::result::Result<Self, SignatureExtError>;
}

impl SignatureExt for Signature {
    fn from_str(signature: &str) -> std::result::Result<Self, SignatureExtError> {
        let signature = signature.trim_start_matches("0x");
        let sig_packed_vec = hex::decode(signature)?;
        let arr: [u8; 64] = sig_packed_vec
            .try_into()
            .map_err(|v: Vec<u8>| SignatureExtError::InvalidLength(v.len()))?;
        decompress_signature(&arr).map_err(SignatureExtError::InvalidPoint)
    }
}
