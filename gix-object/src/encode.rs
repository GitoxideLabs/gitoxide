//! Encoding utilities
use std::io::{self, Write};

use bstr::ByteSlice;

macro_rules! check {
    ($e: expr) => {
        $e.expect("Writing to a Vec should never fail.")
    };
}
/// Generates a loose header buffer
pub fn loose_header(kind: crate::Kind, size: u64) -> smallvec::SmallVec<[u8; 28]> {
    let mut v = smallvec::SmallVec::new();
    check!(v.write_all(kind.as_bytes()));
    check!(v.write_all(SPACE));
    check!(v.write_all(itoa::Buffer::new().format(size).as_bytes()));
    check!(v.write_all(b"\0"));
    v
}

pub(crate) fn header_field_multi_line(name: &[u8], value: &[u8], out: &mut dyn io::Write) -> io::Result<()> {
    let mut lines = value.as_bstr().lines_with_terminator();
    out.write_all(name)?;
    out.write_all(SPACE)?;
    if let Some(line) = lines.next() {
        out.write_all(line)?;
    }
    for line in lines {
        out.write_all(SPACE)?;
        out.write_all(line)?;
    }
    if !value.ends_with_str(b"\n") {
        out.write_all(NL)?;
    }
    Ok(())
}

pub(crate) fn trusted_header_field(name: &[u8], value: &[u8], out: &mut dyn io::Write) -> io::Result<()> {
    out.write_all(name)?;
    out.write_all(SPACE)?;
    out.write_all(value)?;
    out.write_all(NL)
}

pub(crate) fn trusted_header_signature(
    name: &[u8],
    value: &gix_actor::SignatureRef<'_>,
    out: &mut dyn io::Write,
) -> io::Result<()> {
    out.write_all(name)?;
    out.write_all(SPACE)?;
    value.write_to(out)?;
    out.write_all(NL)
}

pub(crate) fn trusted_header_id(
    name: &[u8],
    value: &gix_hash::ObjectId,
    mut out: &mut dyn io::Write,
) -> io::Result<()> {
    out.write_all(name)?;
    out.write_all(SPACE)?;
    value.write_hex_to(&mut out)?;
    out.write_all(NL)
}

/// Serialize a header field. Values containing newlines are retained as `input` bytes in the I/O error.
/// After [wrapping](gix_error::Error::from_error()), inspect them with [metadata](gix_error::Error::metadata()).
pub(crate) fn header_field(name: &[u8], value: &[u8], out: &mut dyn io::Write) -> io::Result<()> {
    if value.is_empty() {
        return Err(io::Error::other(gix_error::validation(
            "Header values must not be empty",
        )));
    }
    if value.find(NL).is_some() {
        return Err(io::Error::other(
            gix_error::validation("Newlines are not allowed in header values").with("input", value),
        ));
    }
    trusted_header_field(name, value, out)
}

pub(crate) const NL: &[u8; 1] = b"\n";
pub(crate) const SPACE: &[u8; 1] = b" ";
