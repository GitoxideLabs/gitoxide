use std::borrow::Cow;

use bstr::{BStr, BString, ByteSlice, ByteVec};
use gix_error::{ErrorExt, OptionExt, Result, ResultExt, bail, ensure};

pub(crate) const NL: &[u8] = b"\n";
pub(crate) const SPACE: &[u8] = b" ";
const SPACE_OR_NL: &[u8] = b" \n";

pub(crate) enum HeaderValue<'a> {
    SingleLine(&'a BStr),
    MultiLine(&'a BStr),
}

impl<'a> HeaderValue<'a> {
    /// Return the header value without its continuation markers.
    ///
    /// A single-line value is borrowed unchanged, without its terminating newline.
    /// For a multiline value, allocate a buffer and strip exactly one leading space from each
    /// continuation line, preserving all other bytes, including the final newline.
    /// For example, `b"first\n  second\n"` becomes `b"first\n second\n"`.
    pub(crate) fn unfold(self) -> Cow<'a, BStr> {
        match self {
            HeaderValue::SingleLine(value) => Cow::Borrowed(value),
            HeaderValue::MultiLine(bytes) => {
                let mut lines = bytes.lines_with_terminator();
                let mut out = BString::from(Vec::with_capacity(bytes.len()));
                out.push_str(lines.next().expect("multiline values have a first line"));
                for line in lines {
                    out.push_str(&line[1..]);
                }
                Cow::Owned(out)
            }
        }
    }
}

/// Parse a specific single-line object header field.
///
/// `name` is the header name without its trailing space, for example `b"tree"`
/// or `b"author"`. Typical input is `<name> <value>\n...`. The value bytes,
/// excluding the header name, separator, and trailing newline, are passed to
/// `parse_value`.
///
/// On success, `i` is advanced past the entire header line and the parsed value
/// is returned.
pub(crate) fn header_field<'a, T>(
    i: &mut &'a [u8],
    name: &'static [u8],
    parse_value: impl FnOnce(&'a [u8]) -> Result<T>,
) -> Result<T> {
    let c = *i;
    let Some(rest) = c.strip_prefix(name).and_then(|rest| rest.strip_prefix(SPACE)) else {
        bail!(crate::decode::empty_error());
    };
    let Some(nl) = rest.find_byte(b'\n') else {
        bail!(crate::decode::empty_error());
    };
    let value = parse_value(&rest[..nl])?;
    *i = &rest[nl + 1..];
    Ok(value)
}

/// Parse an object header and borrow its raw value.
///
/// A single-line value excludes its newline. A multiline value includes the
/// line endings and the leading space of each continuation line.
pub(crate) fn any_header_field<'a>(i: &mut &'a [u8]) -> Result<(&'a BStr, HeaderValue<'a>)> {
    let input = *i;
    let name_end = input
        .find_byteset(SPACE_OR_NL)
        .filter(|pos| *pos > 0)
        .ok_or_raise(crate::decode::empty_error)?;
    ensure!(input[name_end] == b' ', crate::decode::empty_error());
    let value = &input[name_end + 1..];
    let first_line_end = value.find_byte(b'\n').ok_or_raise(crate::decode::empty_error)?;
    let after_first_line = &value[first_line_end + 1..];
    let mut rest = after_first_line;
    while rest.first() == Some(&b' ') {
        let Some(line_end) = rest.find_byte(b'\n') else {
            // Preserve the single-line token and let the next parse report the malformed continuation.
            rest = after_first_line;
            break;
        };
        rest = &rest[line_end + 1..];
    }
    *i = rest;
    let value = if rest.len() == after_first_line.len() {
        HeaderValue::SingleLine(value[..first_line_end].as_bstr())
    } else {
        HeaderValue::MultiLine(value[..value.len() - rest.len()].as_bstr())
    };
    Ok((input[..name_end].as_bstr(), value))
}

/// Parse a complete hexadecimal object id of the given `object_hash`.
///
/// Typical input is a 40-byte SHA-1 hex id or a 64-byte SHA-256 hex id,
/// depending on `object_hash`. The entire input slice must be ASCII hex and
/// match the expected object hash length.
pub fn hex_hash(i: &[u8], hash_kind: gix_hash::Kind) -> Result<&BStr> {
    // Check all bytes without stopping early, which lets the compiler vectorize the loop.
    if i.len() != hash_kind.len_in_hex() || !i.iter().fold(true, |all_hex, b| all_hex & b.is_ascii_hexdigit()) {
        bail!(crate::decode::empty_error());
    }
    Ok(i.as_bstr())
}

/// Parse a complete actor signature.
///
/// Typical input is `Name <name@example.com> 1700000000 +0000`.
/// The entire input slice must be consumed by
/// `gix_actor`'s signature parser; trailing bytes cause an error.
pub(crate) fn signature(mut i: &[u8]) -> Result<gix_actor::SignatureRef<'_>> {
    let signature = gix_actor::SignatureRef::from_bytes_consuming(&mut i)
        .or_raise(|| gix_error::validation("Invalid actor signature"))?;
    if i.is_empty() {
        Ok(signature)
    } else {
        Err(crate::decode::empty_error().raise())
    }
}

/// Validate a complete actor signature and return its raw bytes.
///
/// Typical input is `Name <name@example.com> 1700000000 +0000`. On success, the
/// returned [`BStr`] borrows all of `i`.
pub(crate) fn signature_raw(i: &[u8]) -> Result<&BStr> {
    signature(i).map(|_| i.as_bstr())
}

/// Parse a complete actor signature from a [`BStr`].
///
/// This is a convenience wrapper around [`signature`] for callers that already
/// hold byte-string data.
pub(crate) fn parse_signature(raw: &BStr) -> Result<gix_actor::SignatureRef<'_>> {
    signature(raw.as_ref())
}
