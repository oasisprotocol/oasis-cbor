use std::{cmp::Ordering, iter::Peekable};

use crate::{values::Value, DecodeError};

/// This function is an internal detail of the Decode derive macro, but has public visibility so
/// that users of the macro can use it.
pub fn destructure_cbor_map_peek_value_strict(
    it: &mut Peekable<std::vec::IntoIter<(Value, Value)>>,
    needle: Value,
) -> Result<Option<Value>, DecodeError> {
    match it.peek() {
        None => Ok(None),
        Some(item) => {
            let key: &Value = &item.0;
            match key.cmp(&needle) {
                Ordering::Less => {
                    // Reject unexpected fields.
                    Err(DecodeError::UnknownField)
                }
                Ordering::Equal => {
                    let value: Value = it.next().unwrap().1;
                    Ok(Some(value))
                }
                Ordering::Greater => Ok(None),
            }
        }
    }
}
