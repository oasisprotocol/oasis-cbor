use std::collections::{BTreeMap, HashMap};

use oasis_cbor as cbor;

#[derive(Debug, Clone, Eq, PartialEq, cbor::Encode, cbor::Decode)]
struct A {
    foo: u64,
    bar: String,
    nested: B,
    #[cbor(optional)]
    optional: Option<bool>,
    always: Option<bool>,
    #[cbor(rename = "different")]
    renamed: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, cbor::Encode, cbor::Decode)]
struct B {
    foo: u64,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Eq, PartialEq, cbor::Encode, cbor::Decode)]
enum C {
    One = 1,
    Two = 2,
    Three = 3,
    Four,
    #[cbor(rename = "five")]
    Five,
}

#[derive(Debug, Clone, Eq, PartialEq, cbor::Encode, cbor::Decode)]
enum D {
    One,
    Two(u64),
    Three(u64, u64),
    #[cbor(rename = "four")]
    Four {
        foo: u64,
        #[cbor(rename = "ren")]
        bar: String,
        nested: B,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, cbor::Encode, cbor::Decode)]
struct E(u64, String, bool);

#[derive(Debug, Clone, Eq, PartialEq, cbor::Encode, cbor::Decode)]
#[cbor(transparent)]
struct Transparent(u64);

#[derive(Debug, Clone, Eq, PartialEq, cbor::Encode, cbor::Decode)]
struct NonTransparent(u64);

#[derive(Debug, Clone, Eq, PartialEq, cbor::Encode, cbor::Decode)]
struct WithOptionalDefault {
    #[cbor(optional, default, skip_serializing_if = "String::is_empty")]
    bar: String,
}

#[derive(Debug, Clone, Eq, PartialEq, cbor::Encode, cbor::Decode)]
struct WithOptional {
    #[cbor(optional)]
    bar: String,
}

#[derive(Debug, Clone, Eq, PartialEq, cbor::Encode)]
#[cbor(untagged)]
enum Untagged {
    First { a: u64, b: u64 },
}

#[derive(Debug, Clone, Eq, PartialEq, cbor::Encode, cbor::Decode)]
#[cbor(as_array)]
struct AsArray {
    foo: u64,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Eq, PartialEq, cbor::Encode, cbor::Decode)]
enum AlwaysEncodesAsMap {
    Two(u64),
    Three(u64, u64),
    #[cbor(rename = "four")]
    Four {
        foo: u64,
        #[cbor(rename = "ren")]
        bar: String,
        nested: B,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, cbor::Encode, cbor::Decode)]
enum NonStringKeys {
    #[cbor(rename = 1)]
    One(u64, u64),
    #[cbor(rename = 2)]
    Two,
    #[cbor(rename = 3)]
    Three { foo: u64 },
}

#[test]
fn test_round_trip_complex() {
    let a = A {
        foo: 42,
        bar: "hello world".to_owned(),
        nested: B {
            foo: 10,
            bytes: b"here".to_vec(),
        },
        optional: None,
        always: None,
        renamed: false,
    };

    let enc = cbor::to_vec(a.clone());
    assert_eq!(
        enc,
        vec![
            0xA5, // map(5)
            0x63, // text(3)
            0x62, 0x61, 0x72, // "bar"
            0x6B, // text(11)
            0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x77, 0x6F, 0x72, 0x6C, 0x64, // "hello world"
            0x63, // text(3)
            0x66, 0x6F, 0x6F, // "foo"
            0x18, 0x2A, // unsigned(42)
            0x66, // text(6)
            0x61, 0x6C, 0x77, 0x61, 0x79, 0x73, // "always"
            0xF6, // primitive(22)
            0x66, // text(6)
            0x6E, 0x65, 0x73, 0x74, 0x65, 0x64, // "nested"
            0xA2, // map(2)
            0x63, // text(3)
            0x66, 0x6F, 0x6F, // "foo"
            0x0A, // unsigned(10)
            0x65, // text(5)
            0x62, 0x79, 0x74, 0x65, 0x73, // "bytes"
            0x44, // bytes(4)
            0x68, 0x65, 0x72, 0x65, // "here"
            0x69, // text(9)
            0x64, 0x69, 0x66, 0x66, 0x65, 0x72, 0x65, 0x6E, 0x74, // "different"
            0xF4, // primitive(20)
        ],
        "should encode as expected"
    );

    let dec: A = cbor::from_slice(&enc).expect("serialization should round-trip");
    assert_eq!(dec, a, "serialization should round-trip");
}

#[test]
fn test_enum_unit_discriminant() {
    let tcs = vec![
        (C::One, vec![0x01]),
        (C::Two, vec![0x02]),
        (C::Three, vec![0x03]),
        (
            C::Four,
            vec![
                0x64, // text(4)
                0x46, 0x6F, 0x75, 0x72, // "Four"
            ],
        ),
        (
            C::Five,
            vec![
                0x64, // text(4)
                0x66, 0x69, 0x76, 0x65, // "five"
            ],
        ),
    ];
    for tc in tcs {
        let enc = cbor::to_vec(tc.0.clone());
        assert_eq!(enc, tc.1);
        let dec: C = cbor::from_slice(&enc).expect("serialization should round-trip");
        assert_eq!(dec, tc.0, "serialization should round-trip");
    }
}

#[test]
fn test_enum() {
    let tcs = vec![
        (
            D::One,
            vec![
                0x63, // text(3),
                0x4F, 0x6E, 0x65, // "One"
            ],
        ),
        (
            D::Two(42),
            vec![
                // {"Two": 42}
                0xA1, // map(1)
                0x63, // text(3)
                0x54, 0x77, 0x6F, // "Two"
                0x18, 0x2A, // unsigned(42)
            ],
        ),
        (
            D::Three(11, 32),
            vec![
                // {"Three": [11, 32]}
                0xA1, // map(1)
                0x65, // text(5)
                0x54, 0x68, 0x72, 0x65, 0x65, // "Three"
                0x82, // array(2)
                0x0B, // unsigned(11)
                0x18, 0x20, // unsigned(32)
            ],
        ),
        (
            D::Four {
                foo: 17,
                bar: "hello".to_owned(),
                nested: B {
                    foo: 20,
                    bytes: b"abc".to_vec(),
                },
            },
            vec![
                // {"four": {"foo": 17, "ren": "hello", "nested": {"foo": 20, "bytes": h'616263'}}}
                0xA1, // map(1)
                0x64, // text(4)
                0x66, 0x6F, 0x75, 0x72, // "four"
                0xA3, // map(3)
                0x63, // text(3)
                0x66, 0x6F, 0x6F, // "foo"
                0x11, // unsigned(17)
                0x63, // text(3)
                0x72, 0x65, 0x6E, // "ren"
                0x65, // text(5)
                0x68, 0x65, 0x6C, 0x6C, 0x6F, // "hello"
                0x66, // text(6)
                0x6E, 0x65, 0x73, 0x74, 0x65, 0x64, // "nested"
                0xA2, // map(2)
                0x63, // text(3)
                0x66, 0x6F, 0x6F, // "foo"
                0x14, // unsigned(20)
                0x65, // text(5)
                0x62, 0x79, 0x74, 0x65, 0x73, // "bytes"
                0x43, // bytes(3)
                0x61, 0x62, 0x63, // "abc"
            ],
        ),
    ];
    for tc in tcs {
        let enc = cbor::to_vec(tc.0.clone());
        assert_eq!(enc, tc.1);
        let dec: D = cbor::from_slice(&enc).expect("serialization should round-trip");
        assert_eq!(dec, tc.0, "serialization should round-trip");
    }
}

#[test]
fn test_tuple_struct() {
    let e = E(500, "string".to_owned(), true);

    let enc = cbor::to_vec(e.clone());
    assert_eq!(
        enc,
        vec![
            // [500, "string", true]
            0x83, // array(3)
            0x19, 0x01, 0xF4, // unsigned(500)
            0x66, // text(6)
            0x73, 0x74, 0x72, 0x69, 0x6E, 0x67, // "string"
            0xF5, // primitive(21)
        ],
        "should encode as expected"
    );

    let dec: E = cbor::from_slice(&enc).expect("serialization should round-trip");
    assert_eq!(dec, e, "serialization should round-trip");
}

#[test]
fn test_transparent() {
    let transparent = Transparent(42);
    let enc = cbor::to_vec(transparent);
    assert_eq!(
        enc,
        vec![
        0x18, 0x2a, // unsigned(42)
    ],
        "should encode directly as inner type"
    );

    let non_transparent = NonTransparent(42);
    let enc = cbor::to_vec(non_transparent);
    assert_eq!(
        enc,
        vec![
            0x81, // array(1)
            0x18, 0x2a, // unsigned(42)
        ],
        "should encode as array with inner type"
    );
}

#[test]
fn test_missing_field() {
    let b_without_bytes = vec![
        // {"foo": 10}
        0xA1, // map(1)
        0x63, // text(3)
        0x66, 0x6F, 0x6F, // "foo"
        0x0A, // unsigned(10)
    ];
    let res: Result<B, _> = cbor::from_slice(&b_without_bytes);
    assert!(matches!(res, Err(cbor::DecodeError::MissingField)));
}

#[test]
fn test_invalid_type() {
    let b_invalid_type = vec![
        // {"foo": "boom"}
        0xA1, // map(1)
        0x63, // text(3)
        0x66, 0x6F, 0x6F, // "foo"
        0x64, // text(4)
        0x62, 0x6F, 0x6F, 0x6D, // "boom"
    ];
    let res: Result<B, _> = cbor::from_slice(&b_invalid_type);
    assert!(matches!(res, Err(cbor::DecodeError::UnexpectedType)));
}

#[test]
fn test_field_reorder() {
    let b_reorder = vec![
        // {"bytes": h'01', "foo": 42}
        0xA2, // map(2)
        0x65, // text(5)
        0x62, 0x79, 0x74, 0x65, 0x73, // "bytes"
        0x41, // bytes(1)
        0x01, // "\x01"
        0x63, // text(3)
        0x66, 0x6F, 0x6F, // "foo"
        0x18, 0x2A, // unsigned(42)
    ];
    let res: Result<B, _> = cbor::from_slice(&b_reorder);
    assert!(matches!(res, Err(cbor::DecodeError::ParsingFailed)));
}

#[test]
fn test_extra_fields() {
    // Extra field at the end.
    let b_extra = vec![
        // {"foo": 10, "bytes": h'00', "bytesextra": true}
        0xA3, // map(3)
        0x63, // text(3)
        0x66, 0x6F, 0x6F, // "foo"
        0x0A, // unsigned(10)
        0x65, // text(5)
        0x62, 0x79, 0x74, 0x65, 0x73, // "bytes"
        0x41, // bytes(1)
        0x00, // "\x00"
        0x6A, // text(10)
        0x62, 0x79, 0x74, 0x65, 0x73, 0x65, 0x78, 0x74, 0x72, 0x61, // "bytesextra"
        0xF5, // primitive(21)
    ];
    let res: Result<B, _> = cbor::from_slice(&b_extra);
    assert!(matches!(res, Err(cbor::DecodeError::UnknownField)));

    // Extra field in the middle.
    let b_extra = vec![
        // {"foo": 10, "fop": 10, "bytes": h'00'}
        0xA3, // map(3)
        0x63, // text(3)
        0x66, 0x6F, 0x6F, // "foo"
        0x0A, // unsigned(10)
        0x63, // text(3)
        0x66, 0x6F, 0x70, // "fop"
        0x0A, // unsigned(10)
        0x65, // text(5)
        0x62, 0x79, 0x74, 0x65, 0x73, // "bytes"
        0x41, // bytes(1)
        0x00, // "\x00"
    ];
    let res: Result<B, _> = cbor::from_slice(&b_extra);
    assert!(matches!(res, Err(cbor::DecodeError::UnknownField)));

    // Extra field at the start.
    let b_extra = vec![
        // {"fon": 10, "foo": 10, "bytes": h'00'}
        0xA3, // map(3)
        0x63, // text(3)
        0x66, 0x6F, 0x6E, // "fon"
        0x0A, // unsigned(10)
        0x63, // text(3)
        0x66, 0x6F, 0x6F, // "foo"
        0x0A, // unsigned(10)
        0x65, // text(5)
        0x62, 0x79, 0x74, 0x65, 0x73, // "bytes"
        0x41, // bytes(1)
        0x00, // "\x00"
    ];
    let res: Result<B, _> = cbor::from_slice(&b_extra);
    assert!(matches!(res, Err(cbor::DecodeError::UnknownField)));
}

#[test]
fn test_bigint() {
    let tcs = vec![
        // NOTE: Test cases from Oasis Core (go/common/quantity/quantity_test.go).
        (0, vec![0x40]),
        (1, vec![0x41, 0x01]),
        (10, vec![0x41, 0x0a]),
        (100, vec![0x41, 0x64]),
        (1000, vec![0x42, 0x03, 0xe8]),
        (1000000, vec![0x43, 0x0f, 0x42, 0x40]),
        (
            18446744073709551615,
            vec![0x48, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
    ];
    for tc in tcs {
        let v: u128 = tc.0;
        let enc = cbor::to_vec(v);
        assert_eq!(enc, tc.1, "serialization should match");

        let dec: u128 = cbor::from_slice(&enc).expect("decoding should succeed");
        assert_eq!(v, dec, "serialization should round-trip");
    }
}

#[test]
fn test_with_default() {
    let dec: WithOptionalDefault = cbor::from_slice(&[0xA0]).unwrap();
    assert_eq!(dec, WithOptionalDefault { bar: "".to_owned() });

    let dec: Result<WithOptional, _> = cbor::from_slice(&[0xA0]);
    assert!(matches!(dec, Err(cbor::DecodeError::UnexpectedType)));

    let wod = WithOptionalDefault { bar: "".to_owned() };
    let enc = cbor::to_vec(wod);
    assert_eq!(enc, vec![0xA0]);
}

#[test]
fn test_enum_untagged() {
    let untagged = Untagged::First { a: 10, b: 11 };
    let enc = cbor::to_vec(untagged);
    assert_eq!(
        enc,
        vec![
            // {"a": 10, "b": 11}
            0xA2, // map(2)
            0x61, // text(1)
            0x61, // "a"
            0x0A, // unsigned(10)
            0x61, // text(1)
            0x62, // "b"
            0x0B, // unsigned(11)
        ]
    );
}

#[test]
fn test_btree_map() {
    let mut map = BTreeMap::new();
    map.insert("a", 10);
    map.insert("b", 11);
    let enc = cbor::to_vec(map);
    assert_eq!(
        enc,
        vec![
            // {"a": 10, "b": 11}
            0xA2, // map(2)
            0x61, // text(1)
            0x61, // "a"
            0x0A, // unsigned(10)
            0x61, // text(1)
            0x62, // "b"
            0x0B, // unsigned(11)
        ]
    );
}

#[test]
fn test_hash_map() {
    let mut map = HashMap::new();
    map.insert("a", 10);
    map.insert("b", 11);
    let enc = cbor::to_vec(map);
    assert_eq!(
        enc,
        vec![
            // {"a": 10, "b": 11}
            0xA2, // map(2)
            0x61, // text(1)
            0x61, // "a"
            0x0A, // unsigned(10)
            0x61, // text(1)
            0x62, // "b"
            0x0B, // unsigned(11)
        ]
    );
}

#[test]
fn test_as_array() {
    let asa = AsArray {
        foo: 10,
        bytes: b"here".to_vec(),
    };
    let enc = cbor::to_vec(asa);
    assert_eq!(
        enc,
        vec![
            // [10, h'68657265']
            0x82, // array(2)
            0x0A, // unsigned(10)
            0x44, // bytes(4)
            0x68, 0x65, 0x72, 0x65, // "here"
        ]
    );
}

#[test]
fn test_encode_as_map() {
    fn validate<T: cbor::EncodeAsMap>(_x: T) {}
    validate(AlwaysEncodesAsMap::Two(12));
}

#[test]
fn test_tuples() {
    let t1 = (1u64, "two".to_string(), 3u64, 4u128);
    let enc = cbor::to_vec(t1.clone());
    assert_eq!(
        enc,
        vec![
            // [1, "two", 3, h'04']
            0x84, // array(4)
            0x01, // unsigned(1)
            0x63, // text(3)
            0x74, 0x77, 0x6F, // "two"
            0x03, // unsigned(3)
            0x41, // bytes(1)
            0x04, // "\x04"
        ]
    );

    let dec: (u64, String, u64, u128) = cbor::from_slice(&enc).unwrap();
    assert_eq!(dec, t1, "serialization should round-trip");
}

#[test]
fn test_non_string_keys() {
    let nsk = NonStringKeys::One(10, 20);
    let enc = cbor::to_vec(nsk.clone());
    assert_eq!(
        enc,
        vec![
            // {1: [10, 20]}
            0xA1, // map(1)
            0x01, // unsigned(1)
            0x82, // array(2)
            0x0A, // unsigned(10)
            0x14, // unsigned(20)
        ]
    );

    let dec: NonStringKeys = cbor::from_slice(&enc).expect("serialization should round-trip");
    assert_eq!(dec, nsk, "serialization should round-trip");
}
