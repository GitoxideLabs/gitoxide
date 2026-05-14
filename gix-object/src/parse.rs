use bstr::{BStr, BString, ByteSlice, ByteVec};

pub(crate) const NL: &[u8] = b"\n";
pub(crate) const SPACE: &[u8] = b" ";
const SPACE_OR_NL: &[u8] = b" \n";

/// The result type shared by object parsers.
pub(crate) type ParseResult<T> = Result<T, crate::decode::Error>;

/// Parse any multi-line object header field.
///
/// Typical input is `gpgsig -----BEGIN...\n <continued>\nnext ...`, where the
/// field name is followed by a space, an initial value line, and at least one
/// continuation line starting with a space.
///
/// The returned tuple contains the field name and the unfolded value,
/// with the leading space removed from each continuation line.
///
/// On success, `i` is advanced to the first byte after the final continuation
/// line.
pub(crate) fn any_header_field_multi_line<'a>(i: &mut &'a [u8]) -> ParseResult<(&'a [u8], BString)> {
    let mut c = *i;
    let input = c;
    let name_end = c
        .find_byteset(SPACE_OR_NL)
        .filter(|pos| *pos > 0)
        .ok_or(crate::decode::Error)?;
    if c.get(name_end) != Some(&b' ') {
        return Err(crate::decode::Error);
    }

    c = &c[name_end + 1..];
    let first_line_end = c.find_byte(b'\n').ok_or(crate::decode::Error)?;
    c = &c[first_line_end + 1..];

    let mut continuation_end = name_end + 1 + first_line_end + 1;
    let mut continuation_count = 0usize;
    while c.first() == Some(&b' ') {
        let line_end = c.find_byte(b'\n').ok_or(crate::decode::Error)?;
        continuation_end += line_end + 1;
        c = &c[line_end + 1..];
        continuation_count += 1;
    }
    if continuation_count == 0 {
        return Err(crate::decode::Error);
    }

    let bytes = input[name_end + 1..continuation_end].as_bstr();
    let mut out = BString::from(Vec::with_capacity(bytes.len()));
    let mut lines = bytes.lines_with_terminator();
    out.push_str(lines.next().expect("first line"));
    for line in lines {
        out.push_str(&line[1..]);
    }
    *i = &input[continuation_end..];
    Ok((input[..name_end].as_bstr(), out))
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
    parse_value: impl FnOnce(&'a [u8]) -> ParseResult<T>,
) -> ParseResult<T> {
    let c = *i;
    let Some(rest) = c.strip_prefix(name).and_then(|rest| rest.strip_prefix(SPACE)) else {
        return Err(crate::decode::Error);
    };
    let Some(nl) = rest.find_byte(b'\n') else {
        return Err(crate::decode::Error);
    };
    let value = parse_value(&rest[..nl])?;
    *i = &rest[nl + 1..];
    Ok(value)
}

/// Parse any single-line object header field and return its raw value.
///
/// Typical input is `<field> <value>\n...`. The returned tuple contains the
/// field name and the value bytes without the trailing newline.
///
/// On success, `i` is advanced past the newline.
pub(crate) fn any_header_field<'a>(i: &mut &'a [u8]) -> ParseResult<(&'a [u8], &'a [u8])> {
    let mut c = *i;
    let input = c;
    let name_end = c
        .find_byteset(SPACE_OR_NL)
        .filter(|pos| *pos > 0)
        .ok_or(crate::decode::Error)?;
    if c.get(name_end) != Some(&b' ') {
        return Err(crate::decode::Error);
    }
    c = &c[name_end + 1..];
    if let Some(value_end) = c.find_byte(b'\n') {
        let value = &c[..value_end];
        let rest = &c[value_end + 1..];
        *i = rest;
        Ok((&input[..name_end], value))
    } else {
        Err(crate::decode::Error)
    }
}

/// Parse a complete hexadecimal object id of the given `object_hash`.
///
/// Typical input is a 40-byte SHA-1 hex id or a 64-byte SHA-256 hex id,
/// depending on `object_hash`. The entire input slice must be ASCII hex and
/// match the expected object hash length.
pub fn hex_hash(i: &[u8], hash_kind: gix_hash::Kind) -> ParseResult<&BStr> {
    if i.len() != hash_kind.len_in_hex() || !i.iter().all(u8::is_ascii_hexdigit) {
        return Err(crate::decode::Error);
    }
    Ok(i.as_bstr())
}

/// Parse a complete actor signature.
///
/// Typical input is `Name <name@example.com> 1700000000 +0000`.
/// The entire input slice must be consumed by
/// `gix_actor`'s signature parser; trailing bytes cause an error.
pub(crate) fn signature(mut i: &[u8]) -> ParseResult<gix_actor::SignatureRef<'_>> {
    let signature = gix_actor::SignatureRef::from_bytes_consuming(&mut i).map_err(|_| crate::decode::Error)?;
    if i.is_empty() {
        Ok(signature)
    } else {
        Err(crate::decode::Error)
    }
}

/// Validate a complete actor signature and return its raw bytes.
///
/// Typical input is `Name <name@example.com> 1700000000 +0000`. On success, the
/// returned [`BStr`] borrows all of `i`.
pub(crate) fn signature_raw(i: &[u8]) -> ParseResult<&BStr> {
    signature(i).map(|_| i.as_bstr())
}

/// Parse a complete actor signature from a [`BStr`].
///
/// This is a convenience wrapper around [`signature`] for callers that already
/// hold byte-string data.
pub(crate) fn parse_signature(raw: &BStr) -> Result<gix_actor::SignatureRef<'_>, crate::decode::Error> {
    signature(raw.as_ref())
}
