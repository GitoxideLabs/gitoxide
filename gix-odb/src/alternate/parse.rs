/// Returned as part of [`crate::alternate::Error::Parse`]
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    PathConversion(Vec<u8>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::PathConversion(bytes) => write!(
                f,
                "Could not obtain an object path for the alternate directory '{}'",
                String::from_utf8_lossy(bytes)
            ),
        }
    }
}

impl std::error::Error for Error {}

pub(super) mod function {
    use super::Error;
    use std::{borrow::Cow, path::PathBuf};

    use gix_object::bstr::ByteSlice;

    /// Parse the raw contents of an `objects/info/alternates` file from `input` into paths.
    ///
    /// Empty entries and comments are ignored. Entries beginning with `"` use Git's C-style quoting,
    /// which permits literal newlines in paths. Invalid quoting falls back to the raw entry.
    pub fn parse(mut input: &[u8]) -> Result<Vec<PathBuf>, Error> {
        let mut out = Vec::new();
        while !input.is_empty() {
            let entry = input.as_bstr();
            let end_of_line = || entry.find_byte(b'\n').unwrap_or(entry.len());
            let (path, consumed) = if entry.starts_with(b"#") {
                (None, end_of_line())
            } else {
                // Like Git, try unquoting before treating a newline as the next separator.
                match entry.starts_with(b"\"").then(|| gix_quote::ansi_c::undo(entry)) {
                    Some(Ok((unquoted, consumed))) => (Some(unquoted), consumed),
                    _ => {
                        let consumed = end_of_line();
                        (Some(Cow::Borrowed(entry[..consumed].as_bstr())), consumed)
                    }
                }
            };
            let original = &entry[..consumed];
            let maybe_nl = usize::from(consumed < input.len());
            input = &input[consumed + maybe_nl..];

            let Some(path) = path.filter(|path| !path.is_empty()) else {
                continue;
            };
            out.push(
                gix_path::try_from_bstr(path)
                    .map_err(|_| Error::PathConversion(original.to_vec()))?
                    .into_owned(),
            );
        }
        Ok(out)
    }
}
