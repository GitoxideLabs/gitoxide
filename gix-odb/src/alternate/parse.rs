use std::{borrow::Cow, path::PathBuf};

use gix_object::bstr::ByteSlice;

/// Returned as part of [`crate::alternate::Error::Parse`]
#[derive(thiserror::Error, Debug)]
#[expect(missing_docs)]
pub enum Error {
    #[error("Could not obtain an object path for the alternate directory '{}'", String::from_utf8_lossy(.0))]
    PathConversion(Vec<u8>),
}

pub(crate) fn content(input: &[u8]) -> Result<Vec<PathBuf>, Error> {
    let mut out = Vec::new();
    for line in input.split(|b| *b == b'\n') {
        let line = line.as_bstr();
        if line.is_empty() || line.starts_with(b"#") {
            continue;
        }
        out.push(
            // Broken quoting, like an entry that doesn't end with a quote, falls back to the raw
            // line - a case that Git's alternates parsing in `odb.c` calls out in its own comment.
            gix_path::try_from_bstr(match line.starts_with(b"\"").then(|| gix_quote::ansi_c::undo(line)) {
                Some(Ok((unquoted, _consumed))) => unquoted,
                _ => Cow::Borrowed(line),
            })
            .map_err(|_| Error::PathConversion(line.to_vec()))?
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
            "broken quoting falls back to the raw line, like Git's alternates parsing does"
        );
    }

    #[test]
    fn a_properly_quoted_path_is_unquoted() {
        assert_eq!(
            content(br#""quoted\tpath""#).expect("no path conversion issue"),
            vec![PathBuf::from("quoted\tpath")],
            "…while quoting that is intact still decodes its escapes"
        );
    }
}
