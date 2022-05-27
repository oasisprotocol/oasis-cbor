use std::fmt::Display;

use serde::{de, ser};

/// Errors during serde-compatible (de)serialization.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to serialize value to bytes: {0:?}")]
    ByteEncoder(sk_cbor::writer::EncoderError),
    #[error("failed to deserialize bytes to sk_cbor::Value: {0:?}")]
    ByteDecoder(sk_cbor::reader::DecoderError),
    #[error("failed to decode sk_cbor::Value into rust struct or enum: {0}")]
    ValueDecoder(#[from] crate::DecodeError),
    #[error("values of type {0} are not supported by this implementation")]
    UnsupportedType(&'static str),
    #[error("serde error: {0}")]
    Other(String),
}

impl From<sk_cbor::writer::EncoderError> for Error {
    fn from(e: sk_cbor::writer::EncoderError) -> Self {
        Self::ByteEncoder(e)
    }
}

impl From<sk_cbor::reader::DecoderError> for Error {
    fn from(e: sk_cbor::reader::DecoderError) -> Self {
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
pub(crate) fn unexpected(v: &sk_cbor::Value) -> de::Unexpected {
    match v {
        sk_cbor::Value::Unsigned(n) => de::Unexpected::Unsigned(*n),
        sk_cbor::Value::Negative(n) => de::Unexpected::Signed(*n),
        sk_cbor::Value::ByteString(s) => de::Unexpected::Bytes(s),
        sk_cbor::Value::TextString(s) => de::Unexpected::Str(s),
        sk_cbor::Value::Array(_) => de::Unexpected::Seq,
        sk_cbor::Value::Map(_) => de::Unexpected::Map,
        sk_cbor::Value::Tag(_, _) => de::Unexpected::Other("tag"),
        sk_cbor::Value::Simple(v) => match v {
            sk_cbor::SimpleValue::FalseValue => de::Unexpected::Bool(false),
            sk_cbor::SimpleValue::TrueValue => de::Unexpected::Bool(true),
            sk_cbor::SimpleValue::NullValue => de::Unexpected::Other("null"),
            sk_cbor::SimpleValue::Undefined => de::Unexpected::Other("undefined"),
        },
    }
}
