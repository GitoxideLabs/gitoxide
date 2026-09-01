use bstr::{BStr, ByteSlice};

use crate::{
    parse::{Comment, Error, Event, MaybeDecoded, Span, error::ParseNode, section},
    value::is_git_whitespace,
};

type ParseResult<T> = Result<T, ()>;

/// Parse `input` into span-delineated events, passing results to `dispatch`.
///
/// `input` is the backing buffer for every emitted span and the parser does not copy it. A caller
/// retaining events after this function returns must also retain `input` unchanged, or provide an
/// identical copy when resolving their spans.
///
/// The `input` is a complete Git config file. A UTF BOM is skipped, leading
/// comments, whitespace, newlines, and Git-compatible key/value pairs before
/// the first section are emitted first via `dispatch`, and then one or more
/// sections are parsed until EOF.
///
/// On success, all input is consumed.
/// Inputs larger than [`u32::MAX`] bytes are rejected as spans use compact 32-bit offsets. On other
/// failures, the returned [`Error`] reports the line number, the parser node that was active, and
/// the remaining bytes at the point where parsing stopped.
pub(crate) fn from_bytes(mut input: &[u8], dispatch: &mut dyn FnMut(Event)) -> Result<(), Error> {
    ensure_supported_input_size(input.len())?;
    let backing = input;

    let bom = unicode_bom::Bom::from(input);
    input = &input[bom.len()..];

    loop {
        let before = input;
        if let Ok(comment) = comment(backing, &mut input) {
            dispatch(Event::Comment(comment));
        } else if let Ok(whitespace) = take_spaces1(&mut input) {
            dispatch(Event::Whitespace(Span::new(backing, whitespace)));
        } else if let Ok(newline) = take_newlines1(&mut input) {
            dispatch(Event::Newline(Span::new(backing, newline)));
        } else if !input.starts_with(b"[") {
            let mut node = ParseNode::SectionHeader;
            key_value_pair(backing, &mut input, &mut node, dispatch)
                .map_err(|_| Error::parse(newlines_from(backing, input), node, input.as_bstr().into()))?;
        }
        if input.len() == before.len() {
            break;
        }
    }

    if input.is_empty() {
        return Ok(());
    }

    let mut node = ParseNode::SectionHeader;
    while !input.is_empty() {
        section(backing, &mut input, &mut node, dispatch)
            .map_err(|_| Error::parse(newlines_from(backing, input), node, input.as_bstr().into()))?;
    }
    Ok(())
}

fn ensure_supported_input_size(len: usize) -> Result<(), Error> {
    if len > u32::MAX as usize {
        return Err(Error::input_too_large(len));
    }
    Ok(())
}

/// Count newline bytes in `original` up to the beginning of `input`.
///
/// `rest` is expected to be a suffix of `original`, i.e. the unconsumed input,
/// and the returned count is used as the zero-based parse error line offset.
fn newlines_from(original: &[u8], rest: &[u8]) -> usize {
    let consumed = original.len().saturating_sub(rest.len());
    original[..consumed].iter().filter(|c| **c == b'\n').count()
}

/// Parse a single Git config comment from `i`, computing slices from `backing`.
///
/// A comment starts with `;` or `#` and continues until, but not including, the
/// next `\n` or `\r\n` line ending or EOF. On success, `i` is advanced to the line ending or empty suffix
/// and the returned [`Comment`] stores its text as a span into `backing`.
fn comment(backing: &[u8], i: &mut &[u8]) -> ParseResult<Comment> {
    let Some((&tag, rest)) = i.split_first() else {
        return Err(());
    };
    if tag != b';' && tag != b'#' {
        return Err(());
    }
    let end = rest.find_byte(b'\n').unwrap_or(rest.len());
    let line_ending_start = if end < rest.len() && rest[..end].ends_with(b"\r") {
        end - 1
    } else {
        end
    };
    let text = rest[..line_ending_start].as_bstr();
    *i = &rest[line_ending_start..];
    Ok(Comment {
        tag,
        text: Span::new(backing, text),
    })
}

/// Parse a section header and all following items until the next section or EOF.
///
/// A section starts with a [`section_header`]. The body may contain whitespace,
/// newlines, key/value pairs, and comments in sequence. Parsed items are emitted
/// to `dispatch` with their spans referring to `backing`; `node` is updated before parsing key names and values for
/// error reporting, and `i` is advanced past all consumed section content.
fn section(backing: &[u8], i: &mut &[u8], node: &mut ParseNode, dispatch: &mut dyn FnMut(Event)) -> ParseResult<()> {
    let header = section_header(backing, i)?;
    dispatch(Event::SectionHeader(header));

    loop {
        let before = *i;

        if let Ok(v) = take_spaces1(i) {
            dispatch(Event::Whitespace(Span::new(backing, v)));
        }
        if let Ok(v) = take_newlines1(i) {
            dispatch(Event::Newline(Span::new(backing, v)));
        }

        key_value_pair(backing, i, node, dispatch)?;

        if let Ok(comment) = comment(backing, i) {
            dispatch(Event::Comment(comment));
        }

        if i.len() == before.len() {
            break;
        }
    }

    Ok(())
}

/// Parse a Git config section header.
///
/// Accepted forms are `[name]`, `[name.subsection]`, and the legacy
/// `[name "subsection"]` form. Section names contain ASCII alphanumeric bytes,
/// `-`, and `.`, and may be empty only for compatibility with Git's quoted
/// subsection form. Quoted subsection names are parsed by [`sub_section`]. On
/// success, `i` is advanced past the closing `]`, and header spans refer to `backing`.
fn section_header(backing: &[u8], i: &mut &[u8]) -> ParseResult<section::HeaderData> {
    let mut c = *i;
    c = c.strip_prefix(b"[").ok_or(())?;
    let name = {
        let rest = c;
        let name_len = rest.iter().take_while(|b| is_section_char(**b)).count();
        c = &rest[name_len..];
        rest[..name_len].as_bstr()
    };

    if let Some(rest) = c.strip_prefix(b"]") {
        if name.is_empty() {
            return Err(());
        }
        *i = rest;
        return match name.find_byte(b'.') {
            Some(index) => Ok(section::HeaderData {
                name: Span::new(backing, name[..index].as_bstr()),
                separator: name.get(index..=index).map(|sep| Span::new(backing, sep)),
                subsection_name: name
                    .get(index + 1..)
                    .map(|name| crate::parse::MaybeDecoded::raw(Span::new(backing, name))),
            }),
            None => Ok(section::HeaderData {
                name: Span::new(backing, name.as_bstr()),
                separator: None,
                subsection_name: None,
            }),
        };
    }

    let whitespace = take_spaces1(&mut c)?;
    let Some(rest) = c.strip_prefix(b"\"") else {
        return Err(());
    };
    c = rest;
    let subsection_name = quoted_sub_section(backing, &mut c)?;
    let Some(rest) = c.strip_prefix(b"\"]") else {
        return Err(());
    };
    *i = rest;
    Ok(section::HeaderData {
        name: Span::new(backing, name),
        separator: Some(Span::new(backing, whitespace)),
        subsection_name: Some(subsection_name),
    })
}

/// Return true if `c` is accepted in an unquoted section header name.
///
/// Valid bytes are ASCII alphanumeric characters, `-`, and `.`.
fn is_section_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-' || c == b'.'
}

/// Parse the contents of a quoted legacy subsection name.
///
/// Input starts immediately after the opening quote in `[section "sub"]`.
/// Parsing stops before the closing quote. Backslash escapes copy the escaped
/// byte into an owned buffer; its raw spelling is retained as a span into `backing`.
/// Newlines are rejected. On success, `i` is advanced to the closing
/// quote.
/// NUL byte are explicitly allowed.
fn quoted_sub_section(backing: &[u8], i: &mut &[u8]) -> ParseResult<crate::parse::MaybeDecoded> {
    let mut c = *i;
    let input = c;
    let mut out: Option<Vec<u8>> = None;
    // Number of raw bytes consumed from `input`. It identifies the prefix to copy when the first
    // escape requires a decoded value, and the complete raw span once parsing finishes.
    let mut borrowed_len = 0usize;
    while let Some(&b) = c.first() {
        match b {
            b'"' => break,
            b'\n' => return Err(()),
            b'\\' => {
                let escaped = *c.get(1).ok_or(())?;
                if escaped == b'\n' {
                    return Err(());
                }
                let out = out.get_or_insert_with(|| input[..borrowed_len].to_vec());
                out.push(escaped);
                c = &c[2..];
                borrowed_len = input.len() - c.len();
            }
            _ => {
                if let Some(out) = out.as_mut() {
                    out.push(b);
                }
                c = &c[1..];
                borrowed_len = input.len() - c.len();
            }
        }
    }
    *i = c;
    let raw = Span::new(backing, input[..borrowed_len].as_bstr());
    let out = MaybeDecoded {
        raw,
        decoded: out.map(Into::into),
    };
    Ok(out)
}

/// Parse a config key or value name.
///
/// Names must start with an ASCII alphabetic byte and may continue with ASCII
/// alphanumeric bytes or `-`. On success, `i` is advanced past the name and the
/// returned value refers to the consumed bytes.
fn config_name<'i>(i: &mut &'i [u8]) -> ParseResult<&'i BStr> {
    if !i.first().is_some_and(u8::is_ascii_alphabetic) {
        return Err(());
    }
    let len = i
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric() || **c == b'-')
        .count();
    let (name, rest) = i.split_at(len);
    *i = rest;
    Ok(name.as_bstr())
}

/// Parse an optional key/value pair in a section body.
///
/// If a key name is present, this emits [`Event::SectionValueName`], optional
/// whitespace, and then the value events produced by [`config_value`]. If no
/// key name is present, this succeeds without emitting anything. Emitted spans refer to `backing`.
/// `node` is updated to distinguish name and value parse errors.
fn key_value_pair(
    backing: &[u8],
    i: &mut &[u8],
    node: &mut ParseNode,
    dispatch: &mut dyn FnMut(Event),
) -> ParseResult<()> {
    *node = ParseNode::Name;
    let Ok(name) = config_name(i) else { return Ok(()) };

    dispatch(Event::SectionValueName(Span::new(backing, name)));

    if let Ok(whitespace) = take_spaces1(i) {
        dispatch(Event::Whitespace(Span::new(backing, whitespace)));
    }

    *node = ParseNode::Value;
    config_value(backing, i, dispatch)
}

/// Parse the value portion of a key/value pair.
///
/// If `=` is present, this emits [`Event::KeyValueSeparator`], optional
/// whitespace, and delegates to [`value`]. If `=` is absent, the key is an
/// implicit boolean and an empty [`Event::Value`] is emitted. Emitted spans refer to `backing`.
fn config_value(backing: &[u8], i: &mut &[u8], dispatch: &mut dyn FnMut(Event)) -> ParseResult<()> {
    if let Some(rest) = i.strip_prefix(b"=") {
        *i = rest;
        dispatch(Event::KeyValueSeparator);
        value(backing, i, dispatch)
    } else {
        dispatch(Event::Value(Span::default()));
        Ok(())
    }
}

/// Parse a config value and emit value-related events.
///
/// Values run until newline, EOF, or an unquoted `;` or `#` comment marker.
/// Double quotes toggle quoted mode for comment handling. Supported escapes are
/// backslash followed by `n`, `t`, `\`, `b`, `"`, LF, or CRLF. Line continuations
/// emit [`Event::ValueNotDone`], the continuation newline, and finally [`Event::ValueDone`].
/// At the start of each physical value chunk, Git whitespace is emitted as [`Event::Whitespace`]
/// while the logical value is still empty and outside quotes.
/// If the value ends with a trailing backslash at EOF, it is emitted as
/// [`Event::ValueNotDone`] followed directly by an empty [`Event::ValueDone`].
/// Otherwise a single [`Event::Value`] is emitted with trailing Git whitespace
/// trimmed from the logical value.
/// On success, `i` is advanced to the first unconsumed delimiter or EOF, and emitted spans refer to `backing`.
fn value(backing: &[u8], i: &mut &[u8], dispatch: &mut dyn FnMut(Event)) -> ParseResult<()> {
    let mut remaining = *i;
    let mut value_start = remaining;
    // While quoted, `;` and `#` remain part of the value instead of starting a comment.
    let mut is_in_quotes = false;
    // Set after a line continuation so the final chunk is emitted as `ValueDone`.
    let mut partial_value_found = false;
    // Cleared once parsing contributes a logical byte, to know if leading whitespace is still insignificant.
    let mut value_is_empty_so_far = true;

    loop {
        if value_is_empty_so_far
            && !is_in_quotes
            && remaining.len() == value_start.len()
            && let Ok(whitespace) = take_git_whitespace1(&mut remaining)
        {
            dispatch(Event::Whitespace(Span::new(backing, whitespace)));
            value_start = remaining;
            continue;
        }

        let Some((&byte, rest)) = remaining.split_first() else {
            break;
        };
        match byte {
            b'\n' => break,
            b'\r' if rest.starts_with(b"\n") => break,
            b';' | b'#' if !is_in_quotes => break,
            b'\\' => {
                let Some((&escaped, after_escape)) = rest.split_first() else {
                    let value = value_start[..value_start.len() - remaining.len()].as_bstr();
                    dispatch(Event::ValueNotDone(Span::new(backing, value)));
                    dispatch(Event::ValueDone(Span::default()));
                    *i = rest;
                    return Ok(());
                };
                let after_newline = match escaped {
                    b'\n' => Some(after_escape),
                    b'\r' => Some(after_escape.strip_prefix(b"\n").ok_or(())?),
                    _ => None,
                };
                if let Some(after_newline) = after_newline {
                    partial_value_found = true;
                    let value = value_start[..value_start.len() - remaining.len()].as_bstr();
                    dispatch(Event::ValueNotDone(Span::new(backing, value)));
                    let newline = rest[..rest.len() - after_newline.len()].as_bstr();
                    dispatch(Event::Newline(Span::new(backing, newline)));
                    remaining = after_newline;
                    value_start = remaining;
                    continue;
                }
                if !matches!(escaped, b'n' | b't' | b'\\' | b'b' | b'"') {
                    return Err(());
                }
                // Every non-continuation escape contributes a byte to the logical value,
                // making whitespace after a later continuation significant.
                value_is_empty_so_far = false;
                remaining = after_escape;
            }
            b'"' => {
                is_in_quotes = !is_in_quotes;
                remaining = rest;
            }
            b if !is_git_whitespace(b) || is_in_quotes => {
                // Non-whitespace, and even whitespace inside quotes, is value content,
                // making whitespace after a later continuation significant.
                value_is_empty_so_far = false;
                remaining = rest;
            }
            _ => remaining = rest,
        }
    }
    if is_in_quotes {
        return Err(());
    }

    let untrimmed_value = &value_start[..value_start.len() - remaining.len()];
    let value_len = untrimmed_value
        .iter()
        .rposition(|b| !is_git_whitespace(*b))
        .map_or(0, |idx| idx + 1);
    let value = untrimmed_value[..value_len].as_bstr();
    if partial_value_found {
        dispatch(Event::ValueDone(Span::new(backing, value)));
    } else {
        dispatch(Event::Value(Span::new(backing, value)));
    }
    *i = &value_start[value_len..];
    Ok(())
}

/// Parse one or more Git whitespace bytes without consuming LF or CRLF line endings.
fn take_git_whitespace1<'i>(i: &mut &'i [u8]) -> ParseResult<&'i BStr> {
    let input = *i;
    let mut remaining = input;
    while let Some((&byte, rest)) = remaining.split_first() {
        if byte == b'\n' || byte == b'\r' && rest.starts_with(b"\n") || !is_git_whitespace(byte) {
            break;
        }
        remaining = rest;
    }
    let len = input.len() - remaining.len();
    if len == 0 {
        return Err(());
    }
    let whitespace = &input[..len];
    *i = remaining;
    Ok(whitespace.as_bstr())
}

/// Parse one or more spaces or horizontal tabs.
///
/// At least one space or horizontal tab must be present at the current cursor.
/// On success, `i` is advanced past the whitespace run and the returned
/// [`BStr`] refers to the consumed bytes.
fn take_spaces1<'i>(i: &mut &'i [u8]) -> ParseResult<&'i BStr> {
    let len = i.iter().take_while(|c| **c == b' ' || **c == b'\t').count();
    if len == 0 {
        return Err(());
    }
    let (spaces, rest) = i.split_at(len);
    *i = rest;
    Ok(spaces.as_bstr())
}

/// Parse one or more line endings.
///
/// Both `\n` and `\r\n` are accepted. At least one line ending must be present
/// at the current cursor. On success, `i` is advanced past the newline run and
/// the returned [`BStr`] refers to the consumed bytes.
fn take_newlines1<'i>(i: &mut &'i [u8]) -> ParseResult<&'i BStr> {
    let input = *i;
    let mut remaining = input;
    loop {
        if let Some(rest) = remaining.strip_prefix(b"\r\n") {
            remaining = rest;
        } else if let Some(rest) = remaining.strip_prefix(b"\n") {
            remaining = rest;
        } else {
            break;
        }
    }
    let len = input.len() - remaining.len();
    if len == 0 {
        return Err(());
    }
    *i = remaining;
    Ok(input[..len].as_bstr())
}

#[cfg(test)]
mod tests;
