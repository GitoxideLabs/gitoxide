use std::{borrow::Cow, path::PathBuf};

use gix_object::bstr::ByteSlice;

/// Returned as part of [`crate::alternate::Error::Parse`]
#[derive(thiserror::Error, Debug)]
#[expect(missing_docs)]
pub enum Error {
    #[error("Could not obtain an object path for the alternate directory '{}'", String::from_utf8_lossy(.0))]
    PathConversion(Vec<u8>),
}

pub(crate) fn content(mut input: &[u8]) -> Result<Vec<PathBuf>, Error> {
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

#[cfg(test)]
mod tests {
    use super::content;
    use std::path::PathBuf;

    #[test]
    fn a_quote_that_is_never_closed_is_used_as_a_literal_path() {
        assert_eq!(
            content(br#""unterminated"#).expect("no path conversion issue"),
            vec![PathBuf::from(r#""unterminated"#)],
            "broken quoting falls back to the raw line"
        );
    }

    #[test]
    fn a_properly_quoted_path_is_unquoted() {
        assert_eq!(
            content(br#""quoted\tpath""#).expect("no path conversion issue"),
            vec![PathBuf::from("quoted\tpath")]
        );
    }

    #[test]
    fn a_quoted_path_may_contain_the_line_separator() {
        assert_eq!(
            content(b"\"quoted\npath\"\nnext").expect("no path conversion issue"),
            vec![PathBuf::from("quoted\npath"), PathBuf::from("next")],
            "Git looks for a closing quote before treating a newline as the next separator"
        );
    }
}
