use bstr::ByteSlice;

use crate::{BStr, Kind, TagRef, parse, parse::ParseResult};

/// Parse a complete annotated tag object body.
///
/// Typical input starts with `object <hex>\n`, followed by `type <kind>\n`,
/// `tag <name>\n`, an optional `tagger <signature>\n`, and a message separated
/// from the headers by a blank line. On success, the returned [`TagRef`] borrows
/// all fields from `i` and `i` is advanced to the empty suffix, and it expects
/// to see an entire, fully consumable tag in `i` without any unconsumed content
/// after parsing.
///
/// This parser is not transactional as a whole: if a later field fails, `i` may
/// already have been advanced past earlier successfully parsed fields. Individual
/// field parsers document their own cursor behaviour.
pub fn git_tag<'a>(i: &mut &'a [u8], hash_kind: gix_hash::Kind) -> ParseResult<TagRef<'a>> {
    let target = target(i, hash_kind)?;
    let kind = kind(i)?;
    let tag_version = name(i)?;
    let tagger = tagger_raw(i)?;

    let (message, signature) = message(i)?;
    if !i.is_empty() {
        return Err(crate::decode::empty_error());
    }

    Ok(TagRef {
        target,
        name: tag_version.as_bstr(),
        target_kind: kind,
        message,
        tagger,
        signature,
    })
}

/// Parse the `object <hex>\n` header and return the object id as bytes.
///
/// Typical input is `object 0123456789012345678901234567890123456789\n`.
/// The hash must match `object_hash`. Uppercase ASCII hex is also valid.
/// On success, `i` is advanced past the entire header line.
pub(crate) fn target<'a>(i: &mut &'a [u8], hash_kind: gix_hash::Kind) -> ParseResult<&'a BStr> {
    parse::header_field(i, b"object", |value| parse::hex_hash(value, hash_kind))
}

/// Parse the `type <kind>\n` header and return the object kind.
///
/// Typical inputs are `type commit\n`, `type tree\n`, `type blob\n`, and
/// `type tag\n`. On success, `i` is advanced past the entire header line.
pub(crate) fn kind(i: &mut &[u8]) -> ParseResult<Kind> {
    parse::header_field(i, b"type", Kind::from_bytes)
}

/// Parse the `tag <name>\n` header and return the tag name.
///
/// A typical input is `tag v1.0.0\n`. The returned name excludes both the
/// `tag ` prefix and the trailing newline, and must be non-empty. On success,
/// `i` is advanced past the entire header line.
pub(crate) fn name<'a>(i: &mut &'a [u8]) -> ParseResult<&'a BStr> {
    parse::header_field(i, b"tag", |value| {
        (!value.is_empty())
            .then(|| value.as_bstr())
            .ok_or_else(crate::decode::empty_error)
    })
}

/// Parse an optional `tagger <signature>\n` header and return its raw signature.
///
/// A typical input is `tagger Name <name@example.com> 1700000000 +0000\n`. If
/// the `tagger ` prefix is absent, this returns `Ok(None)`. On success, it
/// returns the signature bytes without the prefix or newline and advances `i`
/// past the entire header line.
pub(crate) fn tagger_raw<'a>(i: &mut &'a [u8]) -> ParseResult<Option<&'a BStr>> {
    if !i.starts_with(b"tagger ") {
        return Ok(None);
    }
    parse::header_field(i, b"tagger", |raw| {
        let mut sig = raw;
        gix_actor::SignatureRef::from_bytes_consuming(&mut sig)?;
        Ok(raw.as_bstr())
    })
    .map(Some)
}

/// Parse an optional `tagger <signature>\n` header and return the decoded signature.
///
/// A typical input is `tagger Name <name@example.com> 1700000000 +0000\n`. If
/// the `tagger ` prefix is absent, this returns `Ok(None)`. On success, it
/// returns the parsed [`gix_actor::SignatureRef`] and advances `i` past the
/// entire header line.
pub(crate) fn tagger<'a>(i: &mut &'a [u8]) -> ParseResult<Option<gix_actor::SignatureRef<'a>>> {
    if !i.starts_with(b"tagger ") {
        return Ok(None);
    }
    parse::header_field(i, b"tagger", |i| {
        let mut sig = i;
        let signature = gix_actor::SignatureRef::from_bytes_consuming(&mut sig)?;
        Ok(signature)
    })
    .map(Some)
}

/// Parse the tag message and its optional cryptographic signature block.
///
/// On success, `i` is always advanced to the empty suffix. The returned tuple
/// contains the message and, if a Git-supported armor marker is found at the
/// beginning of a line, all bytes from the last such marker through end of input.
/// An end-of-signature marker isn't required.
///
/// An input consisting only of newlines is accepted as an empty-header message
/// and consumed entirely. In that case, the newlines are returned as part of
/// the message to preserve roundtrips for tags whose body is only the
/// header/message separator.
pub fn message<'a>(i: &mut &'a [u8]) -> ParseResult<(&'a BStr, Option<&'a BStr>)> {
    if i.iter().all(|b| *b == b'\n') {
        let message = i.as_bstr();
        *i = &[];
        return Ok((message, None));
    }

    let Some(rest) = i.strip_prefix(parse::NL) else {
        return Err(crate::decode::empty_error());
    };

    *i = &[];
    if let Some((sig_start, _format)) = crate::signature::find(rest) {
        // Truncate newline off the message end.
        let message_end = if sig_start > 0 && rest[sig_start - 1] == b'\n' {
            sig_start - 1
        } else {
            sig_start
        };
        let message = rest[..message_end].as_bstr();
        let signature = &rest[sig_start..];
        return Ok((message, (!signature.is_empty()).then(|| signature.as_bstr())));
    }

    Ok((rest.as_bstr(), None))
}
