//! CBOR encoding.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use impl_trait_for_tuples::impl_for_tuples;

use crate::{SimpleValue, Value};

/// Trait for types that can be encoded into CBOR.
pub trait Encode {
    /// Encode the type into a CBOR Value.
    fn into_cbor_value(self) -> Value;
}

/// Trait for types that always encode as CBOR maps.
pub trait EncodeAsMap: Encode {
    /// Encode the type into a CBOR Map, returning the map items.
    fn into_cbor_map(self) -> Vec<(Value, Value)>
    where
        Self: Sized,
    {
        match self.into_cbor_value() {
            Value::Map(items) => items,
            _ => vec![],
        }
    }
}

#[impl_for_tuples(1, 10)]
impl Encode for Tuple {
    #[allow(clippy::vec_init_then_push)]
    fn into_cbor_value(self) -> Value {
        let mut values = vec![];
        for_tuples!( #( values.push(Tuple.into_cbor_value()); )* );
        Value::Array(values)
    }
}

macro_rules! impl_uint {
    ($name:ty) => {
        impl Encode for $name {
            fn into_cbor_value(self) -> Value {
                Value::Unsigned(self as u64)
            }
        }

        impl Encode for &$name {
            fn into_cbor_value(self) -> Value {
                Encode::into_cbor_value(*self)
            }
        }
    };
}

macro_rules! impl_int {
    ($name:ty) => {
        impl Encode for $name {
            fn into_cbor_value(self) -> Value {
                Value::integer(self as i64)
            }
        }

        impl Encode for &$name {
            fn into_cbor_value(self) -> Value {
                Encode::into_cbor_value(*self)
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

impl Encode for u128 {
    fn into_cbor_value(self) -> Value {
        Value::ByteString(self.to_be_bytes()[self.leading_zeros() as usize / 8..].to_vec())
    }
}

impl Encode for &u128 {
    fn into_cbor_value(self) -> Value {
        Encode::into_cbor_value(*self)
    }
}

impl Encode for bool {
    fn into_cbor_value(self) -> Value {
        if self {
            Value::Simple(SimpleValue::TrueValue)
        } else {
            Value::Simple(SimpleValue::FalseValue)
        }
    }
}

impl Encode for String {
    fn into_cbor_value(self) -> Value {
        Value::TextString(self)
    }
}

impl Encode for &str {
    fn into_cbor_value(self) -> Value {
        Value::TextString(self.to_string())
    }
}

impl<T: Encode> Encode for Vec<T> {
    default fn into_cbor_value(self) -> Value {
        Value::Array(self.into_iter().map(Encode::into_cbor_value).collect())
    }
}

impl Encode for Vec<u8> {
    fn into_cbor_value(self) -> Value {
        Value::ByteString(self)
    }
}

impl<T: Encode> Encode for Option<T> {
    fn into_cbor_value(self) -> Value {
        match self {
            Some(v) => Encode::into_cbor_value(v),
            None => Value::Simple(SimpleValue::NullValue),
        }
    }
}

impl Encode for Value {
    fn into_cbor_value(self) -> Value {
        self
    }
}

impl<K: Encode, V: Encode> Encode for BTreeMap<K, V> {
    fn into_cbor_value(self) -> Value {
        Value::Map(
            self.into_iter()
                .map(|(k, v)| (k.into_cbor_value(), v.into_cbor_value()))
                .collect(),
        )
    }
}

impl<K: Encode, V: Encode> EncodeAsMap for BTreeMap<K, V> {}

impl<V: Encode> Encode for BTreeSet<V> {
    fn into_cbor_value(self) -> Value {
        Value::Array(self.into_iter().map(Encode::into_cbor_value).collect())
    }
}

impl<K: Encode, V: Encode> Encode for HashMap<K, V> {
    fn into_cbor_value(self) -> Value {
        Value::Map(
            self.into_iter()
                .map(|(k, v)| (k.into_cbor_value(), v.into_cbor_value()))
                .collect(),
        )
    }
}

impl<K: Encode, V: Encode> EncodeAsMap for HashMap<K, V> {}

impl<V: Encode> Encode for HashSet<V> {
    fn into_cbor_value(self) -> Value {
        Value::Array(self.into_iter().map(Encode::into_cbor_value).collect())
    }
}

impl Encode for () {
    fn into_cbor_value(self) -> Value {
        Value::Simple(SimpleValue::NullValue)
    }
}
