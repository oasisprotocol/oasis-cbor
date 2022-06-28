use std::fmt::Display;

use serde::{de, ser};

use crate::{reader, writer, DecodeError, SimpleValue, Value};

/// Errors during serde-compatible (de)serialization.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to serialize value to bytes: {0:?}")]
    ByteEncoder(writer::EncoderError),
    #[error("failed to deserialize bytes to Value: {0:?}")]
    ByteDecoder(reader::DecoderError),
    #[error("failed to decode sk_cbor::Value into rust struct or enum: {0}")]
    ValueDecoder(#[from] DecodeError),
    #[error("values of type {0} are not supported by this implementation")]
    UnsupportedType(&'static str),
    #[error("serde error: {0}")]
    Other(String),
}

impl From<writer::EncoderError> for Error {
    fn from(e: writer::EncoderError) -> Self {
        Self::ByteEncoder(e)
    }
}

impl From<reader::DecoderError> for Error {
    fn from(e: reader::DecoderError) -> Self {
        Self::ByteDecoder(e)
    }
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Other(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Other(msg.to_string())
    }
}

/// Converts `v` into a serde::de enum indicating that `v` was encountered
/// during deserialization but was not expected.
pub(crate) fn unexpected(v: &Value) -> de::Unexpected {
    match v {
        Value::Unsigned(n) => de::Unexpected::Unsigned(*n),
        Value::Negative(n) => de::Unexpected::Signed(*n as i64),
        Value::ByteString(s) => de::Unexpected::Bytes(s),
        Value::TextString(s) => de::Unexpected::Str(s),
        Value::Array(_) => de::Unexpected::Seq,
        Value::Map(_) => de::Unexpected::Map,
        Value::Tag(_, _) => de::Unexpected::Other("tag"),
        Value::Simple(v) => match v {
            SimpleValue::FalseValue => de::Unexpected::Bool(false),
            SimpleValue::TrueValue => de::Unexpected::Bool(true),
            SimpleValue::NullValue => de::Unexpected::Other("null"),
            SimpleValue::Undefined => de::Unexpected::Other("undefined"),
        },
    }
}
