//! Convenience functions for dealing with CBOR encodings.
//!
//! This is currently a thin wrapper around the `sk-cbor` crate.
#![feature(min_specialization)]

pub mod decode;
pub mod encode;
#[doc(hidden)]
pub mod macros;

pub use sk_cbor::*;
use thiserror::Error;

// Re-export the support proc-macros.
pub use oasis_cbor_derive::*;

// Re-export traits.
pub use crate::{
    decode::Decode,
    encode::{Encode, EncodeAsMap},
};

/// Maximum nesting level allowed when decoding from CBOR.
const MAX_NESTING_LEVEL: i8 = 10;

/// Error encountered during decoding.
#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("parsing failed")]
    ParsingFailed,
    #[error("unexpected type")]
    UnexpectedType,
    #[error("missing field")]
    MissingField,
    #[error("unknown field")]
    UnknownField,
    #[error("unexpected integer size")]
    UnexpectedIntegerSize,
}

impl From<reader::DecoderError> for DecodeError {
    fn from(_e: reader::DecoderError) -> Self {
        DecodeError::ParsingFailed
    }
}

/// Convert CBOR-encoded data into the given type.
pub fn from_slice<T>(data: &[u8]) -> Result<T, DecodeError>
where
    T: Decode,
{
    let value = reader::read_nested(data, Some(MAX_NESTING_LEVEL))?;
    T::try_from_cbor_value(value)
}

/// Convert high-level CBOR representation into the given type.
///
/// This is the same as calling `T::try_from_cbor_value(value)`.
pub fn from_value<T>(value: Value) -> Result<T, DecodeError>
where
    T: Decode,
{
    T::try_from_cbor_value(value)
}

/// Convert the given type into its CBOR-encoded representation.
pub fn to_vec<T>(value: T) -> Vec<u8>
where
    T: Encode,
{
    let mut data = vec![];
    writer::write(value.into_cbor_value(), &mut data).unwrap();
    data
}

/// Convert the given type into its high-level CBOR representation.
///
/// This is the same as calling `value.into_cbor_value()`.
pub fn to_value<T>(value: T) -> Value
where
    T: Encode,
{
    value.into_cbor_value()
}
