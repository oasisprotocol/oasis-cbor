//! Rust part of the differential fuzzer.

#[no_mangle]
pub extern "C" fn cbor_from_slice(data: *const u8, len: usize) -> usize {
    let data = unsafe {
        std::slice::from_raw_parts(data, len)
    };

    let value: Result<oasis_cbor::Value, _> = oasis_cbor::from_slice_non_strict(data);
    if value.is_ok() {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_decode() {
        let tcs: Vec<&[u8]> = vec![
            &[0xa2, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00],
            &[0xA2, 0x30, 0x30, 0x31, 0xFB, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30],
            &[0xA3, 0x30, 0x30, 0x38, 0x30, 0x30, 0x31, 0xE8],
            &[0xA2, 0x65, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x63, 0x30, 0x30, 0x30, 0x38, 0x0B],
            &[0xA2, 0x31, 0x3B, 0xFB, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30],
        ];

        for tc in tcs {
            oasis_cbor::reader::read_nested_non_strict(tc, Some(64)).unwrap();
        }
    }
}
