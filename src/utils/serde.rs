use core::fmt;
use core::marker::PhantomData;
use serde::de::{Deserializer, Error, Unexpected, Visitor};
use serde::ser::Serializer;
use std::convert::TryInto;

pub trait HexArray<'de>: Sized {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

impl<'de, const N: usize> HexArray<'de> for [u8; N] {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(hex::encode(&self).as_str())
    }

    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HexArrayVisitor<T> {
            value: PhantomData<T>,
        }

        impl<'de, const N: usize> Visitor<'de> for HexArrayVisitor<[u8; N]> {
            type Value = [u8; N];

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "an hex encoded array of length {}", N)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                hex::decode(v)
                    .ok()
                    .and_then(|v| v.try_into().ok())
                    .ok_or_else(|| Error::invalid_type(Unexpected::Str(v), &self))
            }
        }

        let visitor = HexArrayVisitor { value: PhantomData };
        deserializer.deserialize_str(visitor)
    }
}
