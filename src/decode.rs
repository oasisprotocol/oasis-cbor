//! CBOR decoding.
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    convert::TryInto,
};

use impl_trait_for_tuples::impl_for_tuples;

use crate::{DecodeError, SimpleValue, Value};

/// Trait for types that can be decoded from CBOR.
pub trait Decode {
    fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError>
    where
        Self: Sized;
}

#[impl_for_tuples(1, 10)]
impl Decode for Tuple {
    fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
        match value {
            Value::Array(mut values) => {
                Ok((for_tuples!( #( Tuple::try_from_cbor_value(values.remove(0))? ),* )))
            }
            _ => Err(DecodeError::UnexpectedType),
        }
    }
}

macro_rules! impl_uint {
    ($name:ty) => {
        impl Decode for $name {
            fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
                match value {
                    Value::Unsigned(v) => {
                        v.try_into().map_err(|_| DecodeError::UnexpectedIntegerSize)
                    }
                    _ => Err(DecodeError::UnexpectedType),
                }
            }
        }
    };
}

macro_rules! impl_int {
    ($name:ty) => {
        impl Decode for $name {
            fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
                match value {
                    Value::Unsigned(v) => {
                        v.try_into().map_err(|_| DecodeError::UnexpectedIntegerSize)
                    }
                    Value::Negative(v) => {
                        v.try_into().map_err(|_| DecodeError::UnexpectedIntegerSize)
                    }
                    _ => Err(DecodeError::UnexpectedType),
                }
            }
        }
    };
}

impl_uint!(u8);
impl_uint!(u16);
impl_uint!(u32);
impl_uint!(u64);
impl_int!(i8);
impl_int!(i16);
impl_int!(i32);
impl_int!(i64);

impl Decode for u128 {
    fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
        match value {
            Value::ByteString(v) => {
                const SIZE: usize = std::mem::size_of::<u128>();

                match v.len().cmp(&SIZE) {
                    Ordering::Greater => {
                        // We only support what can be represented in u128. For all practical cases,
                        // this should be fine.
                        Err(DecodeError::UnexpectedIntegerSize)
                    }
                    Ordering::Less => {
                        // Fill any leading bytes with zeros.
                        let mut data = [0u8; SIZE];
                        data[SIZE - v.len()..].copy_from_slice(&v);
                        Ok(u128::from_be_bytes(data))
                    }
                    Ordering::Equal => {
                        // Exactly the right size.
                        Ok(u128::from_be_bytes(v.try_into().unwrap()))
                    }
                }
            }
            _ => Err(DecodeError::UnexpectedType),
        }
    }
}

impl Decode for bool {
    fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
        match value {
            Value::Simple(SimpleValue::FalseValue) => Ok(false),
            Value::Simple(SimpleValue::TrueValue) => Ok(true),
            _ => Err(DecodeError::UnexpectedType),
        }
    }
}

impl Decode for String {
    fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
        match value {
            Value::TextString(v) => Ok(v),
            _ => Err(DecodeError::UnexpectedType),
        }
    }
}

impl<T: Decode> Decode for Vec<T> {
    default fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
        match value {
            Value::Array(v) => v.into_iter().map(T::try_from_cbor_value).collect(),
            _ => Err(DecodeError::UnexpectedType),
        }
    }
}

impl Decode for Vec<u8> {
    fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
        match value {
            Value::ByteString(v) => Ok(v),
            _ => Err(DecodeError::UnexpectedType),
        }
    }
}

impl<T: Decode> Decode for Option<T> {
    fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
        match value {
            Value::Simple(SimpleValue::NullValue) => Ok(None),
            _ => Ok(Some(T::try_from_cbor_value(value)?)),
        }
    }
}

impl Decode for Value {
    fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
        Ok(value)
    }
}

impl<K: Decode + Ord, V: Decode> Decode for BTreeMap<K, V> {
    fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
        match value {
            Value::Map(v) => {
                let result: Result<Vec<_>, DecodeError> = v
                    .into_iter()
                    .map(|(k, v)| Ok((K::try_from_cbor_value(k)?, V::try_from_cbor_value(v)?)))
                    .collect();
                Ok(result?.into_iter().collect())
            }
            _ => Err(DecodeError::UnexpectedType),
        }
    }
}

impl<T: Decode + Ord> Decode for BTreeSet<T> {
    fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
        match value {
            Value::Array(v) => v.into_iter().map(T::try_from_cbor_value).collect(),
            _ => Err(DecodeError::UnexpectedType),
        }
    }
}

impl<K: Decode + Eq + std::hash::Hash, V: Decode> Decode for HashMap<K, V> {
    fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
        match value {
            Value::Map(v) => {
                let result: Result<Vec<_>, DecodeError> = v
                    .into_iter()
                    .map(|(k, v)| Ok((K::try_from_cbor_value(k)?, V::try_from_cbor_value(v)?)))
                    .collect();
                Ok(result?.into_iter().collect())
            }
            _ => Err(DecodeError::UnexpectedType),
        }
    }
}

impl<T: Decode + Eq + std::hash::Hash> Decode for HashSet<T> {
    fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
        match value {
            Value::Array(v) => v.into_iter().map(T::try_from_cbor_value).collect(),
            _ => Err(DecodeError::UnexpectedType),
        }
    }
}

impl Decode for () {
    fn try_from_cbor_value(value: Value) -> Result<Self, DecodeError> {
        match value {
            Value::Simple(SimpleValue::NullValue) => Ok(()),
            _ => Err(DecodeError::UnexpectedType),
        }
    }
}
