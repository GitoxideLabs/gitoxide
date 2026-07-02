use bstr::{BStr, BString, ByteSlice};

/// Removes quotes, if any, from the provided inputs, and transforms
/// the 3 escape sequences `\n`, `\t` and `\b` into newline and tab
/// respectively, while `\b` will remove the previous character.
///
/// It assumes the input contains a even number of unescaped quotes,
/// and will unescape escaped quotes and everything else (even though the latter
/// would have been rejected in the parsing stage).
///
/// The return values should be safe for value interpretation.
///
/// This has optimizations for fully-quoted values, where the returned value
/// will be a borrowed reference if the only mutation necessary is to unquote
/// the value.
///
/// This is the function used to normalize raw values from higher level
/// abstractions. Generally speaking these
/// high level abstractions will handle normalization for you, and you do not
/// need to call this yourself. However, if you're directly handling events
/// from the parser, you may want to use this to help with value interpretation.
///
/// Generally speaking, you'll want to use one of the variants of this function,
/// such as [`normalize_bstr`] or [`normalize_bstring`].
///
/// # Examples
///
/// Internally quoted values are turned into owned variant with quotes removed.
///
/// ```
/// # use bstr::{BStr, BString};
/// # use gix_config::value::{normalize_bstr};
/// assert_eq!(normalize_bstr("hello \"world\""), BString::from("hello world"));
/// ```
///
/// Escaped quotes are unescaped.
///
/// ```
/// # use bstr::{BStr, BString};
/// # use gix_config::value::normalize_bstr;
/// assert_eq!(normalize_bstr(r#"hello "world\"""#), BString::from(r#"hello world""#));
/// ```
#[must_use]
pub fn normalize(input: &BStr) -> BString {
    let mut input = input;
    if input == "\"\"" {
        return BString::default();
    }
    // An optimization to strip enclosing quotes without producing a new value/copy it.
    while input.len() >= 3 && input[0] == b'"' && input[input.len() - 1] == b'"' && input[input.len() - 2] != b'\\' {
        input = input[1..input.len() - 1].as_ref();
        if input == "\"\"" {
            return BString::default();
        }
    }

    if input.find_byteset(br#"\""#).is_none() {
        return input.into();
    }
    let mut out: BString = Vec::with_capacity(input.len()).into();
    let mut bytes = input.iter().copied();
    while let Some(c) = bytes.next() {
        match c {
            b'\\' => match bytes.next() {
                Some(b'n') => out.push(b'\n'),
                Some(b't') => out.push(b'\t'),
                Some(b'b') => {
                    out.pop();
                }
                Some(c) => {
                    out.push(c);
                }
                None => break,
            },
            b'"' => {}
            _ => out.push(c),
        }
    }
    out
}

/// `&[u8]` variant of [`normalize`].
#[must_use]
pub fn normalize_bstr<'a>(input: impl Into<&'a BStr>) -> BString {
    normalize(input.into())
}

/// `Vec[u8]` variant of [`normalize`].
#[must_use]
pub fn normalize_bstring(input: impl Into<BString>) -> BString {
    normalize(input.into().as_ref())
}
