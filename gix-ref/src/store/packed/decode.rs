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
pub fn reference<'a>(input: &mut &'a [u8], object_hash: gix_hash::Kind) -> Result<packed::Reference<'a>, ()> {
    let target = parse::hex_hash(input, object_hash)?;
    let Some(rest) = input.strip_prefix(b" ") else {
        return Err(());
    };
    *input = rest;
    let name = until_line_end_without_separator(input)?.try_into().map_err(|_| ())?;

    let object = if let Some(rest) = input.strip_prefix(b"^") {
        *input = rest;
        let object = parse::hex_hash(input, object_hash)?;
        parse::newline(input)?;
        Some(object)
    } else {
        None
    };

    Ok(packed::Reference { name, target, object })
}

#[cfg(test)]
mod tests;
