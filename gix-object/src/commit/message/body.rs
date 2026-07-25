use std::{borrow::Cow, ops::Deref};

use crate::{
    bstr::{BStr, BString, ByteSlice, ByteVec},
    commit::message::BodyRef,
};

/// An iterator over trailers as parsed from a commit message body.
///
/// lines with parsing failures will be skipped
pub struct Trailers<'a> {
    pub(crate) cursor: &'a [u8],
}

/// An iterator over raw message portions and the contiguous trailers following each one.
pub struct MessageBlocks<'a> {
    cursor: &'a [u8],
    trailer_start: usize,
}

/// A raw message portion and the contiguous trailers following it.
pub struct MessageBlock<'a> {
    /// The message bytes, including their original whitespace and line endings.
    pub message: &'a BStr,
    trailers: &'a [u8],
}

/// A trailer as parsed from the commit message body.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TrailerRef<'a> {
    /// The name of the trailer, like "Signed-off-by", up to the separator `: `.
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub token: &'a BStr,
    /// The value right after the separator `: `, with leading and trailing whitespace trimmed.
    /// Multi-line values are unfolded to match `git interpret-trailers --parse`, which is when
    /// this field is [`Cow::Owned`].
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub value: Cow<'a, BStr>,
}

// Git treats these as built-in, recognized trailer prefixes when deciding whether a
// trailing paragraph is a trailer block at all. The cherry-pick marker is special in
// that it is not a `token: value` trailer, but it still contributes to Git's
// recognized-prefix / 25% heuristic in `interpret-trailers`.
const GIT_GENERATED_PREFIXES: [&[u8]; 2] = [b"Signed-off-by: ", b"(cherry picked from commit "];

#[derive(Clone, Copy)]
/// A physical line in the original message body.
///
/// `text` has its trailing line ending removed for parsing, while `start`
/// points to the first byte of that line in the original `body` slice.
struct Line<'a> {
    /// The line contents without a trailing `\n` or `\r\n`.
    text: &'a [u8],
    /// Byte offset of the start of this line in the original body buffer.
    start: usize,
}

/// Windows or linux line endings are supported here.
fn trim_line_ending(mut line: &[u8]) -> &[u8] {
    if let Some(stripped) = line.strip_suffix(b"\n") {
        line = stripped;
        if let Some(stripped) = line.strip_suffix(b"\r") {
            line = stripped;
        }
    } else if let Some(stripped) = line.strip_suffix(b"\r") {
        line = stripped;
    }
    line
}

/// Split `input` into physical lines while keeping enough information to map
/// parser decisions back to the original byte slice.
///
/// This is different from using plain `.lines()` because trailer block detection
/// needs normalized line contents for parsing *and* exact byte offsets to slice
/// the original body at the eventual trailer boundary.
fn lines(input: &[u8]) -> Vec<Line<'_>> {
    let mut start = 0;
    input
        .lines_with_terminator()
        .map(|raw| {
            let line = Line {
                text: trim_line_ending(raw),
                start,
            };
            start += raw.len();
            line
        })
        .collect()
}

/// Find the byte position of a Git trailer separator in `line`.
///
/// This recognizes the `:` that terminates a trailer token like `Acked-by: Alice`
/// as well as the looser Git form with optional whitespace before the separator,
/// like `Acked-by : Alice`.
fn find_separator(line: &[u8]) -> Option<usize> {
    let mut whitespace_found = false;
    for (idx, byte) in line.iter().copied().enumerate() {
        if byte == b':' {
            return Some(idx);
        }
        if !whitespace_found && (byte.is_ascii_alphanumeric() || byte == b'-') {
            continue;
        }
        if idx != 0 && matches!(byte, b' ' | b'\t') {
            whitespace_found = true;
            continue;
        }
        break;
    }
    None
}

/// Parse a single physical trailer line.
///
/// Returns `None` if `line` is not a valid trailer line at all.
///
/// Returns `Some((token, separator_offset))` if parsing succeeds, where `token`
/// is the normalized trailer token as a `BStr` and `separator_offset` is the
/// byte offset of the `:` separator in the original `line`. Callers use that
/// offset to slice out the raw value bytes, potentially including following
/// continuation lines.
fn parse_trailer_line(line: &[u8]) -> Option<(&BStr, usize)> {
    if line.first().is_some_and(u8::is_ascii_whitespace) {
        return None;
    }
    let separator = find_separator(line)?;
    (separator > 0).then_some((line[..separator].trim().as_bstr(), separator))
}

fn is_blank_line(line: &[u8]) -> bool {
    line.iter().all(u8::is_ascii_whitespace)
}

fn is_recognized_prefix(line: &[u8]) -> bool {
    GIT_GENERATED_PREFIXES.iter().any(|prefix| line.starts_with(prefix))
}

/// Turn a raw trailer value, possibly spanning multiple physical lines, into
/// the unfolded value Git would expose for parsing.
///
/// A single-line value is returned borrowed. If continuation lines are present,
/// embedded newlines and leading continuation whitespace are collapsed into
/// single spaces and the unfolded value is returned owned.
fn unfold_value(value: &[u8]) -> Cow<'_, BStr> {
    let mut physical_lines = value.lines().peekable();
    let Some(first_line) = physical_lines.next() else {
        return Cow::Borrowed(b"".as_bstr());
    };

    if physical_lines.peek().is_none() {
        return Cow::Borrowed(first_line.trim().as_bstr());
    }

    let mut out = BString::from(first_line.trim());
    for line in physical_lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push_byte(b' ');
        }
        out.extend_from_slice(line);
    }
    Cow::Owned(out)
}

struct TrailerLine<'a> {
    token: &'a BStr,
    separator: usize,
    len: usize,
}

fn trailer_at_start(cursor: &[u8]) -> Option<TrailerLine<'_>> {
    let line = cursor.lines_with_terminator().next()?;
    let (token, separator) = parse_trailer_line(trim_line_ending(line))?;
    let mut len = line.len();
    let mut rest = &cursor[len..];
    while let Some(next_line) = rest.lines_with_terminator().next() {
        let next_text = trim_line_ending(next_line);
        if is_blank_line(next_text) || !next_text.first().is_some_and(u8::is_ascii_whitespace) {
            break;
        }
        len += next_line.len();
        rest = &rest[next_line.len()..];
    }
    Some(TrailerLine { token, separator, len })
}

/// Find the byte offset at which the trailer block begins in `body`.
///
/// Returns `None` if the trailing paragraph does not satisfy Git's trailer-block
/// heuristic. Returns `Some(offset)`  if it does, where `offset` points into the
/// original `body` slice at the first byte that belongs to the trailer block,
/// including the separating blank line when one is present.
///
/// Internally this mirrors Git's backward scan: count trailer lines,
/// non-trailer lines, continuation lines, and whether a recognized built-in
/// prefix was seen, then apply the "all trailers" or recognized-prefix / 25%
/// rule to the last paragraph of the body.
fn trailer_block_start(body: &[u8]) -> Option<usize> {
    /// Git accepts the trailing paragraph either if it is made entirely of
    /// trailers, or if it contains at least one recognized built-in trailer
    /// prefix and at least 25% of the paragraph consists of trailer lines.
    fn accepts_as_trailer_block(recognized_prefix: bool, trailer_lines: usize, non_trailer_lines: usize) -> bool {
        (trailer_lines > 0 && non_trailer_lines == 0) || (recognized_prefix && trailer_lines * 3 >= non_trailer_lines)
    }

    let lines = lines(body);
    let mut recognized_prefix = false;
    let mut trailer_lines = 0usize;
    let mut non_trailer_lines = 0usize;
    let mut possible_continuation_lines = 0usize;
    let mut saw_non_blank_line = false;

    for idx in (0..lines.len()).rev() {
        let line = &lines[idx];
        if is_blank_line(line.text) {
            if !saw_non_blank_line {
                continue;
            }
            non_trailer_lines += possible_continuation_lines;
            return accepts_as_trailer_block(recognized_prefix, trailer_lines, non_trailer_lines).then_some(
                idx.checked_sub(1)
                    .map_or(0, |prev| lines[prev].start + lines[prev].text.len()),
            );
        }

        saw_non_blank_line = true;
        if is_recognized_prefix(line.text) {
            trailer_lines += 1;
            possible_continuation_lines = 0;
            recognized_prefix = true;
            continue;
        }

        if parse_trailer_line(line.text).is_some() {
            trailer_lines += 1;
            possible_continuation_lines = 0;
            continue;
        }

        if line.text.first().is_some_and(u8::is_ascii_whitespace) {
            possible_continuation_lines += 1;
            continue;
        }

        non_trailer_lines += 1 + possible_continuation_lines;
        possible_continuation_lines = 0;
    }

    non_trailer_lines += possible_continuation_lines;
    accepts_as_trailer_block(recognized_prefix, trailer_lines, non_trailer_lines).then_some(0)
}

impl<'a> Iterator for Trailers<'a> {
    type Item = TrailerRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.cursor.is_empty() {
            if let Some(trailer) = trailer_at_start(self.cursor) {
                let value = unfold_value(&self.cursor[trailer.separator + 1..trailer.len]);
                self.cursor = &self.cursor[trailer.len..];
                return Some(TrailerRef {
                    token: trailer.token,
                    value,
                });
            }
            let consumed = self.cursor.lines_with_terminator().next()?.len();
            self.cursor = &self.cursor[consumed..];
        }
        None
    }
}

impl<'a> Iterator for MessageBlocks<'a> {
    type Item = MessageBlock<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor.is_empty() {
            return None;
        }

        let mut message_len = self.trailer_start;
        while message_len < self.cursor.len() && trailer_at_start(&self.cursor[message_len..]).is_none() {
            message_len += self.cursor[message_len..].lines_with_terminator().next()?.len();
        }

        let trailer_start = message_len;
        while let Some(trailer) = trailer_at_start(&self.cursor[message_len..]) {
            message_len += trailer.len;
        }
        let block_len = message_len;
        let block = MessageBlock {
            message: self.cursor[..trailer_start].as_bstr(),
            trailers: &self.cursor[trailer_start..block_len],
        };
        self.cursor = &self.cursor[block_len..];
        self.trailer_start = 0;
        Some(block)
    }
}

impl<'a> MessageBlock<'a> {
    /// Return the continuously parseable trailers immediately following [`message`](MessageBlock::message).
    pub fn trailers(&self) -> Trailers<'a> {
        Trailers { cursor: self.trailers }
    }
}

impl<'a> BodyRef<'a> {
    /// Parse `body` bytes into the trailer and the actual body.
    pub fn from_bytes(body: &'a [u8]) -> Self {
        BodyRef {
            body: body.as_bstr(),
            trailer_start: trailer_block_start(body).unwrap_or(body.len()),
        }
    }

    /// Returns the body with the trailers stripped.
    ///
    /// You can iterate trailers with the [`trailers()`][BodyRef::trailers()] method.
    pub fn without_trailer(&self) -> &'a BStr {
        self.body[..self.trailer_start].as_bstr()
    }

    /// Return an iterator over the trailers parsed from the last paragraph of the body. Maybe empty.
    pub fn trailers(&self) -> Trailers<'a> {
        Trailers {
            cursor: &self.body[self.trailer_start..],
        }
    }

    /// Return the entire body as raw message portions, each followed by a continuously parseable trailer run.
    ///
    /// Message bytes aren't trimmed or normalized. Trailer-like lines outside the block accepted by Git's
    /// trailer heuristic remain part of a message.
    pub fn message_blocks(&self) -> MessageBlocks<'a> {
        MessageBlocks {
            cursor: self.body,
            trailer_start: self.trailer_start,
        }
    }
}

impl AsRef<BStr> for BodyRef<'_> {
    fn as_ref(&self) -> &BStr {
        self.without_trailer()
    }
}

impl Deref for BodyRef<'_> {
    type Target = BStr;

    fn deref(&self) -> &Self::Target {
        self.without_trailer()
    }
}

/// Convenience methods
impl TrailerRef<'_> {
    /// Check if this trailer is a `Signed-off-by` trailer (case-insensitive).
    pub fn is_signed_off_by(&self) -> bool {
        self.token.eq_ignore_ascii_case(b"Signed-off-by")
    }

    /// Check if this trailer is a `Co-authored-by` trailer (case-insensitive).
    pub fn is_co_authored_by(&self) -> bool {
        self.token.eq_ignore_ascii_case(b"Co-authored-by")
    }

    /// Check if this trailer is an `Acked-by` trailer (case-insensitive).
    pub fn is_acked_by(&self) -> bool {
        self.token.eq_ignore_ascii_case(b"Acked-by")
    }

    /// Check if this trailer is a `Reviewed-by` trailer (case-insensitive).
    pub fn is_reviewed_by(&self) -> bool {
        self.token.eq_ignore_ascii_case(b"Reviewed-by")
    }

    /// Check if this trailer is a `Tested-by` trailer (case-insensitive).
    pub fn is_tested_by(&self) -> bool {
        self.token.eq_ignore_ascii_case(b"Tested-by")
    }

    /// Check if this trailer represents any kind of authorship or attribution
    /// (`Signed-off-by`, `Co-authored-by`, etc.).
    pub fn is_attribution(&self) -> bool {
        self.is_signed_off_by()
            || self.is_co_authored_by()
            || self.is_acked_by()
            || self.is_reviewed_by()
            || self.is_tested_by()
    }
}

/// Convenience methods
impl<'a> Trailers<'a> {
    /// Filter trailers to only include `Signed-off-by` entries.
    pub fn signed_off_by(self) -> impl Iterator<Item = TrailerRef<'a>> {
        self.filter(TrailerRef::is_signed_off_by)
    }

    /// Filter trailers to only include `Co-authored-by` entries.
    pub fn co_authored_by(self) -> impl Iterator<Item = TrailerRef<'a>> {
        self.filter(TrailerRef::is_co_authored_by)
    }

    /// Filter trailers to only include attribution-related entries.
    /// (`Signed-off-by`, `Co-authored-by`, `Acked-by`, `Reviewed-by`, `Tested-by`).
    pub fn attributions(self) -> impl Iterator<Item = TrailerRef<'a>> {
        self.filter(TrailerRef::is_attribution)
    }

    /// Filter trailers to only include authors from `Signed-off-by` and `Co-authored-by` entries.
    pub fn authors(self) -> impl Iterator<Item = TrailerRef<'a>> {
        self.filter(|trailer| trailer.is_signed_off_by() || trailer.is_co_authored_by())
    }
}

#[cfg(test)]
mod test_parse_trailer {
    use super::*;

    fn parse(input: &str) -> TrailerRef<'_> {
        Trailers {
            cursor: input.as_bytes(),
        }
        .next()
        .expect("a trailer to be parsed")
    }

    #[test]
    fn simple_newline() {
        assert_eq!(
            parse("foo: bar\n"),
            TrailerRef {
                token: "foo".into(),
                value: b"bar".as_bstr().into()
            }
        );
    }

    #[test]
    fn whitespace_around_separator_is_normalized() {
        assert_eq!(
            parse("foo :  bar"),
            TrailerRef {
                token: "foo".into(),
                value: b"bar".as_bstr().into()
            }
        );
    }

    #[test]
    fn trailing_whitespace_after_value_is_trimmed() {
        assert_eq!(
            parse("hello-foo: bar there   \n"),
            TrailerRef {
                token: "hello-foo".into(),
                value: b"bar there".as_bstr().into()
            }
        );
    }

    #[test]
    fn invalid_token_is_not_a_trailer() {
        assert_eq!(
            Trailers {
                cursor: "🤗: 🎉".as_bytes()
            }
            .next(),
            None
        );
    }

    #[test]
    fn simple_newline_windows() {
        assert_eq!(
            parse("foo: bar\r\n"),
            TrailerRef {
                token: "foo".into(),
                value: b"bar".as_bstr().into()
            }
        );
    }

    #[test]
    fn folded_value_is_unfolded() {
        assert_eq!(
            parse("foo: bar\n continued\r\n  here"),
            TrailerRef {
                token: "foo".into(),
                value: b"bar continued here".as_bstr().into()
            }
        );
    }
}
