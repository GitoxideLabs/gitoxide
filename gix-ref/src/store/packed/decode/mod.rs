use gix_error::{ExnMessageResult, ResultExt};
use gix_object::bstr::{BStr, ByteSlice};

use crate::{parse, store_impl::packed};

#[derive(Debug, PartialEq, Eq)]
enum Peeled {
    Unspecified,
    Partial,
    Fully,
}

/// Information parsed from the header of a packed ref file
#[derive(Debug, PartialEq, Eq)]
pub struct Header {
    peeled: Peeled,
    pub sorted: bool,
}

impl Default for Header {
    fn default() -> Self {
        Header {
            peeled: Peeled::Unspecified,
            sorted: false,
        }
    }
}

/// Return the bytes before the next line ending as a [`BStr`].
///
/// On success, `input` is advanced past the line ending. The returned slice
/// does not include the line ending.
fn until_line_end_without_separator<'a>(input: &mut &'a [u8]) -> Result<&'a BStr, ()> {
    let line_end = input.iter().position(|b| *b == b'\r' || *b == b'\n').ok_or(())?;
    let out = input[..line_end].as_bstr();
    let mut maybe_start_of_newline = &input[line_end..];
    parse::newline(&mut maybe_start_of_newline)?;
    *input = maybe_start_of_newline;
    Ok(out)
}

/// Parse a `packed-refs` header line.
///
/// A valid header starts with `# pack-refs with: ` and ends with a line ending.
/// Known space-separated traits after the prefix populate the returned
/// [`Header`]: `peeled`, `fully-peeled`, and `sorted`. Unknown traits are
/// ignored.
///
/// On success, `input` is advanced past the entire header line, including its
/// line ending.
pub fn header(input: &mut &[u8]) -> Result<Header, ()> {
    let Some(rest) = input.strip_prefix(b"# pack-refs with: ") else {
        return Err(());
    };
    *input = rest;
    let traits = until_line_end_without_separator(input)?;
    let mut peeled = Peeled::Unspecified;
    let mut sorted = false;
    for token in traits.split_str(b" ") {
        if token == b"fully-peeled" {
            peeled = Peeled::Fully;
        } else if token == b"peeled" {
            peeled = Peeled::Partial;
        } else if token == b"sorted" {
            sorted = true;
        }
    }
    Ok(Header { peeled, sorted })
}

/// Parse one packed reference entry and its optional peeled object line.
///
/// The reference line has the form `<hex-object-id> <ref-name>` followed by a
/// line ending. If the following line starts with `^`, it is parsed as the
/// peeled object id for the returned [`packed::Reference`].
/// Object ids are parsed according to `object_hash`.
///
/// On success, `input` is advanced past the reference line and, if present, the
/// peeled object line.
pub fn reference<'a>(input: &mut &'a [u8], object_hash: gix_hash::Kind) -> ExnMessageResult<packed::Reference<'a>> {
    let invalid = || gix_error::corruption("Malformed packed reference");
    let target = parse::hex_hash(input, object_hash).map_err(|()| invalid())?;
    *input = input.strip_prefix(b" ").ok_or_else(invalid)?;
    let name = <&crate::FullNameRef>::try_from(until_line_end_without_separator(input).map_err(|()| invalid())?)
        .or_raise_typed(invalid)?;

    let object = if let Some(rest) = input.strip_prefix(b"^") {
        *input = rest;
        let object = parse::hex_hash(input, object_hash).map_err(|()| invalid())?;
        parse::newline(input).map_err(|()| invalid())?;
        Some(object)
    } else {
        None
    };

    Ok(packed::Reference { name, target, object })
}

/// Extract the name bytes from a packed-refs record without validating the
/// name as a [`FullNameRef`][crate::FullNameRef] or the hash as hex.
///
/// Used by the binary search in [`super::Buffer::binary_search_by`], where
/// only the name bytes are needed for ordered comparison — the final match
/// is re-parsed through [`reference`] which validates fully.
///
/// Returns `None` when the record does not have the expected
/// `<hash><space><name><newline>` shape (including an empty name or input
/// shorter than the hash plus separator).
pub(crate) fn name_at_record_start(input: &[u8], object_hash: gix_hash::Kind) -> Option<&[u8]> {
    let hex_len = object_hash.len_in_hex();
    if input.get(hex_len) != Some(&b' ') {
        return None;
    }
    let after = &input[hex_len + 1..];
    let end = after.iter().position(|&b| b == b'\r' || b == b'\n')?;
    if end == 0 {
        return None;
    }
    if after[end] == b'\r' && after.get(end + 1) != Some(&b'\n') {
        return None;
    }
    Some(&after[..end])
}

/// Return the byte offset of the packed-refs record containing `offset`.
///
/// If `offset` points into a peeled object line, the returned offset points to
/// the owning reference record instead.
pub(crate) fn record_start_at_offset(input: &[u8], offset: usize) -> usize {
    input[..offset]
        .rfind(b"\n")
        .and_then(|pos| {
            let candidate = pos + 1;
            let b = input.get(candidate)?;
            if *b == b'^' {
                input[..pos].rfind(b"\n").map(|pos| pos + 1)
            } else {
                Some(candidate)
            }
        })
        .unwrap_or(0)
}

/// Return the packed-refs record containing `offset`.
///
/// The returned slice begins at the owning reference line. It includes the
/// remainder of `input`, which is enough for callers that only need to parse
/// the first record.
pub(crate) fn record_at_offset(input: &[u8], offset: usize) -> &[u8] {
    &input[record_start_at_offset(input, offset)..]
}

#[cfg(test)]
mod tests;
