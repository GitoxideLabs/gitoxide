use std::{borrow::Cow, path::PathBuf};

use gix_object::bstr::ByteSlice;

/// Returned as part of [`crate::alternate::Error::Parse`]
// TODO(review): `Unquote` wraps `gix_quote::ansi_c::undo::Error`, which is a `gix_error::Exn` and does
//                not implement `std::error::Error`; `source()` dereferences it to the inner error, just
//                as the `thiserror` `#[from]` did via `as_dyn_error()` (which auto-derefs the `Exn`).
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    PathConversion(Vec<u8>),
    Unquote(gix_quote::ansi_c::undo::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::PathConversion(bytes) => write!(
                f,
                "Could not obtain an object path for the alternate directory '{}'",
                String::from_utf8_lossy(bytes)
            ),
            Error::Unquote(_) => f.write_str("Could not unquote alternate path"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Unquote(err) => Some(&**err),
            Error::PathConversion(_) => None,
        }
    }
}

impl From<gix_quote::ansi_c::undo::Error> for Error {
    fn from(err: gix_quote::ansi_c::undo::Error) -> Self {
        Error::Unquote(err)
    }
}

pub(crate) fn content(input: &[u8]) -> Result<Vec<PathBuf>, Error> {
    let mut out = Vec::new();
    for line in input.split(|b| *b == b'\n') {
        let line = line.as_bstr();
        if line.is_empty() || line.starts_with(b"#") {
            continue;
        }
        out.push(
            gix_path::try_from_bstr(if line.starts_with(b"\"") {
                gix_quote::ansi_c::undo(line)?.0
            } else {
                Cow::Borrowed(line)
            })
            .map_err(|_| Error::PathConversion(line.to_vec()))?
            .into_owned(),
        );
    }
    Ok(out)
}
