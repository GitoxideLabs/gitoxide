use super::MAX_DATA_LEN;

/// The error returned by most functions in the [`encode`](crate::encode) module
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    DataLengthLimitExceeded { length_in_bytes: usize },
    DataIsEmpty,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DataLengthLimitExceeded { length_in_bytes } => {
                write!(f, "Cannot encode more than {MAX_DATA_LEN} bytes, got {length_in_bytes}")
            }
            Error::DataIsEmpty => f.write_str("Empty lines are invalid"),
        }
    }
}

impl std::error::Error for Error {}

pub(crate) fn u16_to_hex(value: u16) -> [u8; 4] {
    let mut buf = [0u8; 4];
    faster_hex::hex_encode(&value.to_be_bytes(), &mut buf).expect("two bytes to 4 hex chars never fails");
    buf
}
