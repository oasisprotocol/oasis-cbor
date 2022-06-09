use serde::{
    ser::{self, SerializeStruct},
    Serialize,
};

use crate::{serde::error::Error, Encode, Value};

pub(crate) struct Serializer;

/// Derives impls for serialize_X() methods for all types X that trivially
/// forward serialization to core oasis-cbor.
macro_rules! trivial_serialize_fns {
    ($(fn $name:ident($ty:ty);)*) => {
        $(
        fn $name(self, v: $ty) -> Result<Self::Ok, Self::Error> {
            Ok(v.into_cbor_value())
        }
        )*
    };
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SeqSerializer;
    type SerializeTuple = SeqSerializer;
    type SerializeTupleStruct = TupleStructSerializer;
    type SerializeTupleVariant = TupleVariantSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = StructSerializer;
    type SerializeStructVariant = StructVariantSerializer;

    trivial_serialize_fns! {
        fn serialize_bool(bool);
        fn serialize_i8(i8);
        fn serialize_i16(i16);
        fn serialize_i32(i32);
        fn serialize_i64(i64);
        fn serialize_u8(u8);
        fn serialize_u16(u16);
        fn serialize_u32(u32);
        fn serialize_u64(u64);
        fn serialize_u128(u128);
        fn serialize_char(char);
        fn serialize_str(&str);
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        Err(Self::Error::UnsupportedType("f32"))
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        Err(Self::Error::UnsupportedType("f64"))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Value::ByteString(v.to_owned()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(().into_cbor_value())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<Self::Ok, Self::Error> {
        v.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(().into_cbor_value())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(().into_cbor_value())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        (&[value]).serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(vec![(
            variant.serialize(&mut *self)?,
            value.serialize(self)?,
        )]))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer { items: vec![] })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SeqSerializer { items: vec![] })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(TupleStructSerializer { fields: vec![] })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TupleVariantSerializer {
            variant,
            fields: vec![],
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            key: None,
            items: vec![],
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructSerializer { fields: vec![] })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructVariantSerializer {
            variant,
            fields: vec![],
        })
    }
}

pub(crate) struct SeqSerializer {
    items: Vec<Value>,
}

impl<'a> ser::SerializeTuple for SeqSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.items.push(value.serialize(&mut Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Array(self.items))
    }
}

/// The implementation of `SerializeSeq` just forwards to `SerializeTuple`.
impl<'a> ser::SerializeSeq for SeqSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        <Self as ser::SerializeTuple>::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as ser::SerializeTuple>::end(self)
    }
}

pub(crate) struct StructVariantSerializer {
    variant: &'static str,
    fields: Vec<(Value, Value)>,
}
impl<'a> ser::SerializeStructVariant for StructVariantSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let mut s = Serializer;
        self.fields
            .push((key.serialize(&mut s)?, value.serialize(&mut s)?));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(vec![(
            self.variant.serialize(&mut Serializer)?,
            Value::Map(self.fields),
        )]))
    }
}

pub(crate) struct TupleVariantSerializer {
    variant: &'static str,
    fields: Vec<Value>,
}
impl<'a> ser::SerializeTupleVariant for TupleVariantSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.fields.push(value.serialize(&mut Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(vec![(
            self.variant.serialize(&mut Serializer)?,
            Value::Array(self.fields),
        )]))
    }
}

pub(crate) struct TupleStructSerializer {
    fields: Vec<Value>,
}
impl ser::SerializeTupleStruct for TupleStructSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.fields.push(value.serialize(&mut Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Array(self.fields))
    }
}

pub(crate) struct MapSerializer {
    key: Option<Value>,
    items: Vec<(Value, Value)>,
}
impl ser::SerializeMap for MapSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.key = Some(key.serialize(&mut Serializer)?);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.items.push((
            self.key
                .take()
                .expect("serde tried to encode a map value without a key"),
            value.serialize(&mut Serializer)?,
        ));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.items))
    }
}

pub(crate) struct StructSerializer {
    fields: Vec<(Value, Value)>,
}
impl SerializeStruct for StructSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.fields.push((
            key.serialize(&mut Serializer)?,
            value.serialize(&mut Serializer)?,
        ));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.fields))
    }
}
