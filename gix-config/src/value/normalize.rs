use std::borrow::Cow;

use bstr::{BStr, BString, ByteSlice};

/// Removes quotes, if any, from the provided inputs, and transforms the
/// escape sequences `\n`, `\t` and `\b` into newline, tab and backspace
/// (byte `0x08`) respectively, matching how `git` interprets them.
///
/// It assumes the input contains a even number of unescaped quotes,
/// and will unescape escaped quotes and everything else (even though the latter
/// would have been rejected in the parsing stage).
///
/// The return values should be safe for value interpretation.
/// If normalization requires no byte changes, including when it only trims enclosing quotes,
/// the returned value borrows from `input`. Transforming escapes or embedded quotes produces an
/// owned value instead.
///
/// This is the function used to normalize raw values from higher level
/// abstractions. Generally speaking these
/// high level abstractions will handle normalization for you, and you do not
/// need to call this yourself. However, if you're directly handling events
/// from the parser, you may want to use this to help with value interpretation.
///
/// # Examples
///
/// Internally quoted values are turned into an owned variant with quotes removed.
///
/// ```
/// # use gix_config::value::normalize;
/// assert_eq!(&*normalize("hello \"world\""), "hello world");
/// ```
///
/// Escaped quotes are unescaped.
///
/// ```
/// # use gix_config::value::normalize;
/// assert_eq!(&*normalize(r#"hello "world\"""#), r#"hello world""#);
/// ```
#[must_use]
pub fn normalize(input: &(impl crate::AsBStr + ?Sized)) -> Cow<'_, BStr> {
    normalize_inner(input.as_bstr())
}

fn normalize_inner(mut input: &BStr) -> Cow<'_, BStr> {
    if input == "\"\"" {
        return Cow::Borrowed(BStr::new(&input[..0]));
    }
    // An optimization to strip enclosing quotes without producing a new value/copy it.
    while input.len() >= 3 && input[0] == b'"' && input[input.len() - 1] == b'"' && input[input.len() - 2] != b'\\' {
        input = input[1..input.len() - 1].as_ref();
        if input == "\"\"" {
            return Cow::Borrowed(BStr::new(&input[..0]));
        }
    }

    if input.find_byteset(br#"\""#).is_none() {
        return Cow::Borrowed(input);
    }
    let mut out: BString = Vec::with_capacity(input.len()).into();
    let mut bytes = input.iter().copied();
    while let Some(c) = bytes.next() {
        match c {
            b'\\' => match bytes.next() {
                Some(b'n') => out.push(b'\n'),
                Some(b't') => out.push(b'\t'),
                Some(b'b') => out.push(b'\x08'),
                Some(c) => {
                    out.push(c);
                }
                None => break,
            },
            b'"' => {}
            _ => out.push(c),
        }
    }
    Cow::Owned(out)
}
