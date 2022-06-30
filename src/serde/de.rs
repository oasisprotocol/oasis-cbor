use std::iter::Peekable;

use serde::{
    de::{self, Error as _, Visitor},
    Deserializer as _,
};

use super::error::{unexpected, Error};
use crate::{Decode as _, SimpleValue, Value};

pub(crate) struct Deserializer(Value);

impl Deserializer {
    pub fn from_cbor_value(input: Value) -> Self {
        Deserializer(input)
    }
}

impl<'de> de::Deserializer<'de> for Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = match self.0 {
            Value::Unsigned(n) => visitor.visit_u64(n),
            Value::Negative(n) => visitor.visit_i64(n as i64),
            Value::ByteString(bytes) => visitor.visit_byte_buf(bytes),
            Value::TextString(s) => visitor.visit_string(s),
            Value::Array(_) => self.deserialize_seq(visitor),
            Value::Map(_) => self.deserialize_map(visitor),
            Value::Tag(_, _) => Err(Error::UnsupportedType("cbor tag")),
            Value::Simple(v) => match v {
                SimpleValue::FalseValue => visitor.visit_bool(false),
                SimpleValue::TrueValue => visitor.visit_bool(true),
                SimpleValue::NullValue => visitor.visit_unit(),
                SimpleValue::Undefined => visitor.visit_unit(),
            },
        };
        value
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64
        str string identifier
        bytes byte_buf
        newtype_struct tuple_struct
        tuple
        unit unit_struct
        ignored_any
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let n = u128::try_from_cbor_value(self.0).map_err(Error::from)?;
        visitor.visit_u128(n)
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::UnsupportedType("f32"))
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::UnsupportedType("f64"))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let c = char::try_from_cbor_value(self.0).map_err(Error::from)?;
        visitor.visit_char(c)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.0 {
            Value::Simple(SimpleValue::NullValue) => visitor.visit_none(),
            x => visitor.visit_some(Self(x)),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        struct SeqAccessImpl<I>(I);
        impl<'de, I: Iterator<Item = Value>> de::SeqAccess<'de> for SeqAccessImpl<I> {
            type Error = Error;

            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
            where
                T: de::DeserializeSeed<'de>,
            {
                match self.0.next() {
                    Some(item) => seed
                        .deserialize(Deserializer::from_cbor_value(item))
                        .map(Some),
                    None => Ok(None),
                }
            }
        }

        let items = match self.0 {
            Value::Array(a) => a,
            _ => return Err(Error::invalid_type(unexpected(&self.0), &"array")),
        };
        visitor.visit_seq(SeqAccessImpl(items.into_iter()))
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        struct MapAccessImpl<I: Iterator<Item = (Value, Value)>>(Peekable<I>);
        impl<'de, I: Iterator<Item = (Value, Value)>> de::MapAccess<'de> for MapAccessImpl<I> {
            type Error = Error;
            fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
            where
                K: de::DeserializeSeed<'de>,
            {
                match self.0.peek_mut() {
                    Some((k, _v)) => {
                        let k = std::mem::replace(k, Value::Simple(SimpleValue::Undefined));
                        Ok(Some(seed.deserialize(Deserializer(k))?))
                    }
                    None => Ok(None),
                }
            }

            fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
            where
                V: de::DeserializeSeed<'de>,
            {
                match self.0.next() {
                    Some((_k, v)) => seed.deserialize(Deserializer(v)),
                    None => Err(de::Error::custom("unexpected end of map")),
                }
            }
        }

        let pairs = match self.0 {
            Value::Map(pairs) => pairs,
            _ => return Err(Error::invalid_type(unexpected(&self.0), &"map")),
        };
        visitor.visit_map(MapAccessImpl(pairs.into_iter().peekable()))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        struct EnumAccessImpl((Value, Value)); // The pair represents (variant name, variant value).
        impl<'de> de::EnumAccess<'de> for EnumAccessImpl {
            type Error = Error;
            type Variant = VariantAccessImpl;

            fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
            where
                V: de::DeserializeSeed<'de>,
            {
                Ok((
                    seed.deserialize(Deserializer(self.0 .0))?,
                    VariantAccessImpl(self.0 .1),
                ))
            }
        }

        struct VariantAccessImpl(Value);
        impl<'de> de::VariantAccess<'de> for VariantAccessImpl {
            type Error = Error;

            fn unit_variant(self) -> Result<(), Self::Error> {
                match self.0 {
                    Value::Simple(SimpleValue::NullValue) => Ok(()),
                    _ => Err(de::Error::invalid_type(unexpected(&self.0), &"null")),
                }
            }

            fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(
                self,
                seed: T,
            ) -> Result<T::Value, Self::Error> {
                seed.deserialize(Deserializer(self.0))
            }

            fn tuple_variant<V: Visitor<'de>>(
                self,
                _len: usize,
                visitor: V,
            ) -> Result<V::Value, Self::Error> {
                Deserializer(self.0).deserialize_seq(visitor)
            }

            fn struct_variant<V: Visitor<'de>>(
                self,
                _fields: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value, Self::Error> {
                Deserializer(self.0).deserialize_map(visitor)
            }
        }

        match self.0 {
            Value::Map(mut pairs) if pairs.len() == 1 => {
                let pair = pairs.swap_remove(0);
                visitor.visit_enum(EnumAccessImpl(pair))
            }
            variant_name @ Value::TextString(_) => visitor.visit_enum(EnumAccessImpl((
                variant_name,
                Value::Simple(SimpleValue::NullValue),
            ))),
            _ => Err(de::Error::invalid_type(
                unexpected(&self.0),
                &"string or map with one key",
            )),
        }
    }
}
