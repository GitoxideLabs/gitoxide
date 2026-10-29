use std::borrow::Cow;

use gix_error::{ErrorExt, Result};
use smallvec::SmallVec;

use crate::{BStr, ByteSlice, CommitRef, parse};

/// Parse the commit message after the header/message separator.
///
/// Typical input starts with the blank-line separator before the message, for
/// example `\nsubject\n\nbody\n`. The returned message excludes that first
/// separator newline and borrows all remaining bytes from `i`.
///
/// On success, `i` is advanced to the empty suffix as commits end with a message.
pub fn message<'a>(i: &mut &'a [u8]) -> Result<&'a BStr> {
    if let Some(rest) = i.strip_prefix(parse::NL) {
        *i = &[];
        Ok(rest.as_bstr())
    } else {
        Err(crate::decode::empty_error().raise())
    }
}

/// Parse a complete commit object body.
///
/// Typical input starts with `tree <hex>\n`, followed by zero or more
/// `parent <hex>\n` headers, then `author <signature>\n` and
/// `committer <signature>\n`. An optional `encoding <name>\n` header and any
/// number of extra single-line or multi-line headers may follow. The headers
/// are terminated by a blank line, after which all remaining bytes are the
/// commit message.
///
/// On success, the returned [`CommitRef`] borrows fields from `i` where
/// possible, and `i` is advanced to the empty suffix. Extra single-line header
/// values are borrowed, while multi-line header values are unfolded into owned
/// buffers.
///
/// This parser is not transactional as a whole: if a later required field or
/// the final message parse fails, `i` may already have been advanced past
/// earlier successfully parsed fields.
pub fn commit<'a>(i: &mut &'a [u8], object_hash: gix_hash::Kind) -> Result<CommitRef<'a>> {
    commit_with_signed_data(i, object_hash).map(|(commit, _)| commit)
}

pub(super) fn commit_with_signed_data<'a>(
    i: &mut &'a [u8],
    object_hash: gix_hash::Kind,
) -> Result<(CommitRef<'a>, Option<crate::signature::SignedData<'a>>)> {
    let data = *i;
    let mut signature_range = None;
    let tree = parse::header_field(i, b"tree", |value| parse::hex_hash(value, object_hash))?;

    let mut parents = SmallVec::new();
    loop {
        let before = *i;
        match parse::header_field(i, b"parent", |value| parse::hex_hash(value, object_hash)) {
            Ok(parent) => parents.push(parent),
            Err(_) => {
                *i = before;
                break;
            }
        }
    }

    let author = parse::header_field(i, b"author", parse::signature_raw)?;
    let committer = parse::header_field(i, b"committer", parse::signature_raw)?;

    let before = *i;
    let encoding = match parse::header_field(i, b"encoding", Ok) {
        Ok(encoding) => Some(encoding.as_bstr()),
        Err(_) => {
            *i = before;
            None
        }
    };

    let mut extra_headers = Vec::new();
    loop {
        let before = *i;
        match parse::any_header_field_multi_line(i)
            .map(|(k, v)| (k.as_bstr(), Cow::Owned(v)))
            .or_else(|_| {
                *i = before;
                parse::any_header_field(i).map(|(k, v)| (k.as_bstr(), Cow::Borrowed(v.as_bstr())))
            }) {
            Ok(header) => {
                if signature_range.is_none() && header.0 == super::signature_field_name(object_hash) {
                    signature_range = Some(data.len() - before.len()..data.len() - i.len());
                }
                extra_headers.push(header);
            }
            Err(_) => {
                *i = before;
                break;
            }
        }
    }

    let message = message(i)?;
    gix_error::ensure!(i.is_empty(), crate::decode::empty_error());

    Ok((
        CommitRef {
            tree,
            parents,
            author,
            committer,
            encoding,
            message,
            extra_headers,
        },
        signature_range.map(|range| crate::signature::SignedData::new(data, range)),
    ))
}
