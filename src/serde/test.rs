use oasis_cbor_derive::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::{SimpleValue, Value};

macro_rules! str {
    ($s:expr) => {
        Value::TextString($s.to_owned())
    };
}

/// Asserts that `oasis_cbor::serde` and bare `oasis_cbor` produce compatible serializations of `v`.
/// Specifically, asserts that
///   - The `Value` representation is the same between the two libraries, and equal to `expected_intermediate`.
///   - `v` can be encoded to bytes with `oasis_cbor::serde` and decoded with bare `oasis_cbor` back to `v`.
///   - Same as above, with the two (de)serializers swapped.
fn assert_compat_roundtrip<T>(v: T, expected_intermediate: Value)
where
    T: Clone,
    T: serde::Serialize + crate::Decode, // for round-tripping
    T: crate::Encode + serde::de::DeserializeOwned, // for round-tripping the other way
    T: std::cmp::PartialEq + std::fmt::Debug, // for asserting equality after round-trip
{
    assert_serde_roundtrip(v.clone(), expected_intermediate);

    // For debugging/development purposes; shown only when a test fails.
    println!(
        "oasis_cbor representation of {:?}: {:?}",
        v,
        v.clone().into_cbor_value(),
    );

    // Check that `Value` representations match.
    let intermediate = crate::serde::to_value(&v).unwrap();
    assert_eq!(
        intermediate,
        v.clone().into_cbor_value(),
        "intermediate representation (left) does not match oasis_cbor (right)"
    );

    // Round-trip: encode with oasis_cbor, decode with serde.
    let bytes = crate::to_vec(v.clone());
    let reconstructed: T = crate::serde::from_slice(&bytes).expect("serde decoding should succeed");
    assert_eq!(
        v, reconstructed,
        "bad round-trip when encoding with oasis_cbor and decoding with serde"
    );

    // Round-trip: encode with serde, decode with oasis_cbor.
    let bytes = crate::serde::to_vec(&v).unwrap();
    let reconstructed: T = crate::from_slice(&bytes).expect("oasis_cbor decoding should succeed");
    assert_eq!(
        v, reconstructed,
        "bad round-trip when encoding with serde and decoding with oasis_cbor"
    );
}

/// Asserts that serde produces the `expected_intermediate` representation for `v`,
/// and that `v` roundtrips to bytes and back using the `serde` compatibility layer.
fn assert_serde_roundtrip<T>(v: T, expected_intermediate: Value)
where
    T: serde::Serialize + serde::de::DeserializeOwned, // for round-tripping
    T: std::cmp::PartialEq + std::fmt::Debug,          // for asserting equality after round-trip
{
    let intermediate = crate::serde::to_value(&v).unwrap();
    println!("serde representation of {:?}: {:?}", v, intermediate);
    assert_eq!(
        intermediate, expected_intermediate,
        "intermediate representation (left) does not match expected value (right)"
    );

    let bytes = crate::serde::to_vec(&v).unwrap();
    let reconstructed: T = crate::serde::from_slice(&bytes).expect("serde decoding should succeed");
    assert_eq!(
        v, reconstructed,
        "bad round-trip when (de)serializing with oasis_cbor::serde"
    );
}

#[test]
fn test_simple_types() {
    // bool
    assert_compat_roundtrip(false, Value::Simple(SimpleValue::FalseValue));

    // int
    assert_compat_roundtrip(0u64, Value::Unsigned(0));
    assert_compat_roundtrip(1_000_000i32, Value::Unsigned(1_000_000));
    assert_compat_roundtrip(1_000_000i64, Value::Unsigned(1_000_000));
    assert_compat_roundtrip(0u128, Value::ByteString(vec![]));
    assert_compat_roundtrip(1_000_000u128, Value::ByteString(vec![15, 66, 64]));

    // char
    assert_compat_roundtrip('A', Value::Unsigned(65));

    // str
    assert_compat_roundtrip("foo".to_string(), str!("foo"));
}

#[test]
fn test_float() {
    // Floats are not supported by sk_cbor; make sure we enforce that at the serde layer already.

    // Encoding.
    let err = crate::serde::to_value(&1.0)
        .err()
        .expect("encoding f32 should fail");
    assert!(
        matches!(err, crate::serde::Error::UnsupportedType(_)),
        "f32 should be marked as unsupported"
    );

    // Decoding.
    let one = [0xf9, 0x3c, 0x00]; // CBOR encoding for float(1.0)
    let err = crate::serde::from_slice::<f32>(&one)
        .err()
        .expect("decoding f32 should fail");
    assert!(
        matches!(
            err,
            crate::serde::Error::ByteDecoder(
                sk_cbor::reader::DecoderError::UnsupportedFloatingPointValue
            )
        ),
        "f32 should be marked as unsupported, but error was {:?}",
        err
    );
}

#[test]
fn test_vec() {
    assert_compat_roundtrip(
        vec![30u16, 10],
        Value::Array(vec![Value::Unsigned(30), Value::Unsigned(10)]),
    );
}

#[test]
fn test_tuple() {
    // Large tuple.
    assert_compat_roundtrip(
        (101, 102, 103, 104, 105, 106, 107, 108, 109, 110),
        Value::Array((101..=110).map(Value::Unsigned).collect::<Vec<_>>()),
    );

    // Elements of different types.
    assert_compat_roundtrip(
        (1, "one".to_string(), ()),
        Value::Array(vec![
            Value::Unsigned(1),
            str!("one"),
            Value::Simple(SimpleValue::NullValue),
        ]),
    );
}

#[test]
fn test_map() {
    // string keys
    let mut m = std::collections::HashMap::new();
    m.insert("foo".to_string(), "one".to_string());
    m.insert("bar".to_string(), "two".to_string());
    m.insert("baz".to_string(), "three".to_string());
    m.insert("quux".to_string(), "four".to_string());
    // There's no guarantee about the order of the keys inside the Value::Map that is produced
    // by serialization. The test is stable only because Value::Map's equality operator
    // ignores the order.
    assert_compat_roundtrip(
        m,
        Value::Map(vec![
            (str!("bar"), str!("two")),
            (str!("baz"), str!("three")),
            (str!("foo"), str!("one")),
            (str!("quux"), str!("four")),
        ]),
    );

    // int keys
    let mut m = std::collections::HashMap::new();
    m.insert(2u8, 4u8);
    m.insert(1, 5);
    m.insert(3, 3);
    assert_compat_roundtrip(
        m,
        Value::Map(vec![
            (Value::Unsigned(1), Value::Unsigned(5)),
            (Value::Unsigned(2), Value::Unsigned(4)),
            (Value::Unsigned(3), Value::Unsigned(3)),
        ]),
    );
}

#[test]
fn test_bytes() {
    // NOTE: oasis_cbor encodes [u8] as `Value::ByteString`. By contrast, this implementation
    // encodes it as `Value::Array` because the `serde` framework cannot special-case
    // the serialization of [u8] compared to [T]: https://github.com/serde-rs/serde/issues/309
    // So we cannot assert_compat_roundtrip() here.
    assert_serde_roundtrip(
        vec![31u8, 11],
        Value::Array(vec![Value::Unsigned(31), Value::Unsigned(11)]),
    );

    // To efficiently encode bytes slices with serde, use the serde_bytes wrapper type.
    // This encodes as a `Value::BytesString` (and has minimal overhead).
    let hello_bytes = vec![104, 101, 108, 108, 111];
    let wrapped_bytes = serde_bytes::ByteBuf::from(&*hello_bytes);
    assert_serde_roundtrip(wrapped_bytes, Value::ByteString(hello_bytes));
}

#[test]
fn test_option() {
    let n: Option<u16> = None;
    assert_compat_roundtrip(n, Value::Simple(SimpleValue::NullValue));

    let s = Some("foo".to_string());
    assert_compat_roundtrip(s, str!("foo"));
}

#[test]
fn test_unit() {
    let x = ();
    assert_compat_roundtrip(x, Value::Simple(SimpleValue::NullValue));
}

mod enums {
    use super::*;

    #[test]
    fn test_unit_variant() {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Encode, Decode, Clone)]
        enum Enum {
            One,
            Second,
            Three,
        }
        assert_compat_roundtrip(Enum::Second, str!("Second"));
    }

    #[test]
    fn test_newtype_variant() {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Encode, Decode, Clone)]
        enum E {
            M(String),
            N(u8),
        }
        assert_compat_roundtrip(E::N(10), Value::Map(vec![(str!("N"), Value::Unsigned(10))]));
    }

    #[test]
    fn test_tuple_variant() {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Encode, Decode, Clone)]
        enum E {
            M(String, u16),
            N(u8),
        }
        assert_compat_roundtrip(
            E::M("foo".to_string(), 10),
            Value::Map(vec![(
                str!("M"),
                Value::Array(vec![str!("foo"), Value::Unsigned(10)]),
            )]),
        );
    }

    #[test]
    fn test_struct_variant() {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Encode, Decode, Clone)]
        enum E {
            S { r: u8, g: u8, b: u8 },
        }
        assert_compat_roundtrip(
            E::S {
                r: 10,
                g: 20,
                b: 30,
            },
            Value::Map(vec![(
                str!("S"),
                Value::Map(vec![
                    (str!("b"), Value::Unsigned(30)),
                    (str!("g"), Value::Unsigned(20)),
                    (str!("r"), Value::Unsigned(10)),
                ]),
            )]),
        );
    }

    #[test]
    fn test_explicit_discriminants() {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Encode, Decode, Clone)]
        enum E {
            Two = 2,
            Three,
        }

        // NOTE: Because the discriminant (i.e. the number that represents the variant)
        // for E::Two is explicitly specified, non-serde oasis_cbor encodes it as a number.
        // The serde layer does not special-case and will encode it as a string.
        assert_serde_roundtrip(E::Two, str!("Two"));
        assert_compat_roundtrip(E::Three, str!("Three"));
    }
}

mod structs {
    use super::*;

    #[test]
    fn test_newtype_struct() {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Encode, Decode, Clone, Default)]
        struct Millimeters(u16);
        assert_compat_roundtrip(Millimeters(100), Value::Array(vec![Value::Unsigned(100)]));
    }

    #[test]
    fn test_tuple_struct() {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Encode, Decode, Clone, Default)]
        struct Rgb(u8, u8, u8);
        assert_compat_roundtrip(
            Rgb(10, 20, 30),
            Value::Array(vec![
                Value::Unsigned(10),
                Value::Unsigned(20),
                Value::Unsigned(30),
            ]),
        );
    }

    #[test]
    fn test_classic_struct() {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Encode, Decode, Clone, Default)]
        struct Color {
            r: u8,
            g: u8,
            b: u8,
        }
        assert_compat_roundtrip(
            Color {
                r: 10,
                g: 20,
                b: 30,
            },
            Value::Map(vec![
                (str!("b"), Value::Unsigned(30)),
                (str!("g"), Value::Unsigned(20)),
                (str!("r"), Value::Unsigned(10)),
            ]),
        );
    }

    #[test]
    fn test_unit_struct() {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Encode, Decode, Clone, Default)]
        struct Unit;
        let v = Unit {};
        assert_compat_roundtrip(v, Value::Simple(SimpleValue::NullValue));
    }
}
