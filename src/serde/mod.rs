//! Compatibility layer for serde. It allows to (de)serialize any serde-compatible type as CBOR.
//! Although it provides convenience methods for (de)serializing directly into/from bytes,
//! this module only implements converts between native rust types and `sk_cbor::Value`. The
//! conversion between `sk_cbor::Value` and bytes is handled by the `sk_cbor` library.
//!
//! NOTE: CBOR encoding is not strictly defined for non-primitive types. For any given type T,
//! serializations produced by `#[derive(Encode)]` (from core `oasis-cbor`) and those produced
//! by `#[derive(Deserialize)]` (from this module) are NOT GUARANTEED TO BE COMPATIBLE.
//! See notes in the test suite for examples of incompatibilities.

mod de;
mod error;
mod ser;
#[cfg(test)]
mod test;

pub use self::error::Error;
use crate::Value;

/// Deserialize CBOR-encoded bytes into `T`.
pub fn from_slice<T>(data: &[u8]) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let value =
        sk_cbor::reader::read_nested(data, Some(crate::MAX_NESTING_LEVEL)).map_err(Error::from)?;
    from_value(value)
}

/// Deserialize intermediate-representation `sk_cbor::Value` into `T`.
pub fn from_value<T>(value: Value) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let deserializer = de::Deserializer::from_cbor_value(value);
    T::deserialize(deserializer)
}

/// Serialize `value` into CBOR bytes.
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, Error>
where
    T: ?Sized + serde::Serialize,
{
    let mut data = vec![];
    sk_cbor::writer::write(to_value(value)?, &mut data).map_err(Error::from)?;
    Ok(data)
}

/// Serialize `value` into intermediate-representation `sk_cbor::Value`.
pub fn to_value<T>(value: &T) -> Result<Value, Error>
where
    T: ?Sized + serde::Serialize,
{
    let mut ser = ser::Serializer;
    value.serialize(&mut ser)
}
