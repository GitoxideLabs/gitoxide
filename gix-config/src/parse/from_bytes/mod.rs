use std::borrow::Cow;

use bstr::{BStr, ByteSlice};

use crate::parse::{Comment, Error, Event, error::ParseNode, section};

type ParseResult<T> = Result<T, ()>;

/// Attempt to zero-copy parse the provided `input`, passing results to `dispatch`.
///
/// The `input` is a complete Git config file. A UTF BOM is skipped, leading
/// comments, whitespace, newlines, and Git-compatible key/value pairs before
/// the first section are emitted first via `dispatch`, and then one or more
/// sections are parsed until EOF.
///
/// On success, all input is consumed.
/// On failure, the returned [`Error`] reports the line number, the parser node
/// that was active, and the remaining bytes at the point where parsing stopped.
pub fn from_bytes<'i>(mut input: &'i [u8], dispatch: &mut dyn FnMut(Event<'i>)) -> Result<(), Error> {
    let original = input;

    let bom = unicode_bom::Bom::from(input);
    input = &input[bom.len()..];

    loop {
        let before = input;
        if let Ok(comment) = comment(&mut input) {
            dispatch(Event::Comment(comment));
        } else if let Ok(whitespace) = take_spaces1(&mut input) {
            dispatch(Event::Whitespace(Cow::Borrowed(whitespace)));
        } else if let Ok(newline) = take_newlines1(&mut input) {
            dispatch(Event::Newline(Cow::Borrowed(newline)));
        } else if !input.starts_with(b"[") {
            let mut node = ParseNode::SectionHeader;
            key_value_pair(&mut input, &mut node, dispatch).map_err(|_| Error {
                line_number: newlines_from(original, input),
                last_attempted_parser: node,
                parsed_until: input.as_bstr().into(),
            })?;
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
        section(&mut input, &mut node, dispatch).map_err(|_| Error {
            line_number: newlines_from(original, input),
            last_attempted_parser: node,
            parsed_until: input.as_bstr().into(),
        })?;
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

/// Parse a single Git config comment.
///
/// A comment starts with `;` or `#` and continues until, but not including, the
/// next `\n` or EOF. On success, `i` is advanced to the newline or empty suffix
/// and the returned [`Comment`] borrows the marker and text from the input.
fn comment<'i>(i: &mut &'i [u8]) -> ParseResult<Comment<'i>> {
    let Some((&tag, rest)) = i.split_first() else {
        return Err(());
    };
    if tag != b';' && tag != b'#' {
        return Err(());
    }
    let end = rest.find_byte(b'\n').unwrap_or(rest.len());
    let text = rest[..end].as_bstr();
    *i = &rest[end..];
    Ok(Comment {
        tag,
        text: Cow::Borrowed(text),
    })
}

/// Parse a section header and all following items until the next section or EOF.
///
/// A section starts with a [`section_header`]. The body may contain whitespace,
/// newlines, key/value pairs, and comments in sequence. Parsed items are emitted
/// to `dispatch`, `node` is updated before parsing key names and values for
/// error reporting, and `i` is advanced past all consumed section content.
fn section<'i>(i: &mut &'i [u8], node: &mut ParseNode, dispatch: &mut dyn FnMut(Event<'i>)) -> ParseResult<()> {
    let header = section_header(i)?;
    dispatch(Event::SectionHeader(header));

    loop {
        let before = *i;

        if let Ok(v) = take_spaces1(i) {
            dispatch(Event::Whitespace(Cow::Borrowed(v.as_bstr())));
        }
        if let Ok(v) = take_newlines1(i) {
            dispatch(Event::Newline(Cow::Borrowed(v.as_bstr())));
        }

        key_value_pair(i, node, dispatch)?;

        if let Ok(comment) = comment(i) {
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
/// success, `i` is advanced past the closing `]`.
fn section_header<'i>(i: &mut &'i [u8]) -> ParseResult<section::Header<'i>> {
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
            Some(index) => Ok(section::Header {
                name: section::Name(Cow::Borrowed(name[..index].as_bstr())),
                separator: name.get(index..=index).map(|s| Cow::Borrowed(s.as_bstr())),
                subsection_name: name.get(index + 1..).map(|s| Cow::Borrowed(s.as_bstr())),
            }),
            None => Ok(section::Header {
                name: section::Name(Cow::Borrowed(name.as_bstr())),
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
    let subsection_name = quoted_sub_section(&mut c)?;
    let Some(rest) = c.strip_prefix(b"\"]") else {
        return Err(());
    };
    *i = rest;
    Ok(section::Header {
        name: section::Name(Cow::Borrowed(name)),
        separator: Some(Cow::Borrowed(whitespace)),
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
/// byte into an owned buffer; otherwise the returned value borrows from the
/// input. Newlines are rejected. On success, `i` is advanced to the closing
/// quote.
/// NUL byte are explicitly allowed.
fn quoted_sub_section<'i>(i: &mut &'i [u8]) -> ParseResult<Cow<'i, BStr>> {
    let mut c = *i;
    let input = c;
    let mut out: Option<Vec<u8>> = None;
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
    Ok(match out {
        Some(out) => Cow::Owned(out.into()),
        None => Cow::Borrowed(input[..borrowed_len].as_bstr()),
    })
}

/// Parse a config key or value name.
///
/// Names must start with an ASCII alphabetic byte and may continue with ASCII
/// alphanumeric bytes or `-`. On success, `i` is advanced past the name and the
/// returned value borrows the consumed bytes.
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
/// key name is present, this succeeds without emitting anything.
/// `node` is updated to distinguish name and value parse errors.
fn key_value_pair<'i>(i: &mut &'i [u8], node: &mut ParseNode, dispatch: &mut dyn FnMut(Event<'i>)) -> ParseResult<()> {
    *node = ParseNode::Name;
    let Ok(name) = config_name(i) else { return Ok(()) };

    dispatch(Event::SectionValueName(section::ValueName(Cow::Borrowed(name))));

    if let Ok(whitespace) = take_spaces1(i) {
        dispatch(Event::Whitespace(Cow::Borrowed(whitespace)));
    }

    *node = ParseNode::Value;
    config_value(i, dispatch)
}

/// Parse the value portion of a key/value pair.
///
/// If `=` is present, this emits [`Event::KeyValueSeparator`], optional
/// whitespace, and delegates to [`value`]. If `=` is absent, the key is an
/// implicit boolean and an empty [`Event::Value`] is emitted.
fn config_value<'i>(i: &mut &'i [u8], dispatch: &mut dyn FnMut(Event<'i>)) -> ParseResult<()> {
    if let Some(rest) = i.strip_prefix(b"=") {
        *i = rest;
        dispatch(Event::KeyValueSeparator);
        if let Ok(whitespace) = take_spaces1(i) {
            dispatch(Event::Whitespace(Cow::Borrowed(whitespace)));
        }
        value(i, dispatch)
    } else {
        dispatch(Event::Value(Cow::Borrowed("".into())));
        Ok(())
    }
}

/// Parse a config value and emit value-related events.
///
/// Values run until newline, EOF, or an unquoted `;` or `#` comment marker.
/// Double quotes toggle quoted mode for comment handling. Supported escapes are
/// backslash followed by `n`, `t`, `\`, `b`, `"`, LF, or CRLF. Line continuations
/// emit [`Event::ValueNotDone`], the continuation newline, and finally [`Event::ValueDone`].
/// If the value ends with a trailing backslash at EOF, it is emitted as
/// [`Event::ValueNotDone`] followed directly by an empty [`Event::ValueDone`].
/// Otherwise a single [`Event::Value`] is emitted with trailing ASCII whitespace
/// trimmed from the logical value.
/// On success, `i` is advanced to the first unconsumed delimiter or EOF.
fn value<'i>(i: &mut &'i [u8], dispatch: &mut dyn FnMut(Event<'i>)) -> ParseResult<()> {
    let input = *i;
    let mut cursor = 0usize;
    let mut value_start = 0usize;
    let mut value_end = None;
    // While quoted, `;` and `#` remain part of the value instead of starting a comment.
    let mut is_in_quotes = false;
    // Set after a line continuation so the final chunk is emitted as `ValueDone`.
    let mut partial_value_found = false;

    while cursor < input.len() {
        match input[cursor] {
            b'\n' => {
                value_end = Some(cursor);
                break;
            }
            b';' | b'#' if !is_in_quotes => {
                value_end = Some(cursor);
                break;
            }
            b'\\' => {
                let escape_index = cursor;
                cursor += 1;
                let mut consumed = 1usize;
                let Some(mut b) = input.get(cursor).copied() else {
                    let value = input[value_start..escape_index].as_bstr();
                    dispatch(Event::ValueNotDone(Cow::Borrowed(value)));
                    dispatch(Event::ValueDone(Cow::Borrowed("".into())));
                    *i = &[];
                    return Ok(());
                };
                if b == b'\r' {
                    cursor += 1;
                    b = *input.get(cursor).ok_or(())?;
                    if b != b'\n' {
                        return Err(());
                    }
                    consumed += 1;
                }
                match b {
                    b'\n' => {
                        partial_value_found = true;
                        let value = input[value_start..escape_index].as_bstr();
                        dispatch(Event::ValueNotDone(Cow::Borrowed(value)));
                        let nl_start = escape_index + 1;
                        let nl = input[nl_start..nl_start + consumed].as_bstr();
                        dispatch(Event::Newline(Cow::Borrowed(nl)));
                        cursor += 1;
                        value_start = cursor;
                        value_end = None;
                    }
                    b'n' | b't' | b'\\' | b'b' | b'"' => cursor += 1,
                    _ => return Err(()),
                }
            }
            b'"' => {
                is_in_quotes = !is_in_quotes;
                cursor += 1;
            }
            _ => cursor += 1,
        }
    }
    if is_in_quotes {
        return Err(());
    }

    let end = value_end.unwrap_or(cursor);
    if end == value_start {
        dispatch(Event::Value(Cow::Borrowed("".into())));
        *i = &input[cursor..];
        return Ok(());
    }

    let value_end_no_trailing_whitespace = input[value_start..end]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(idx, b)| (!b.is_ascii_whitespace()).then_some(value_start + idx + 1))
        .unwrap_or(value_start);
    let value = input[value_start..value_end_no_trailing_whitespace].as_bstr();
    if partial_value_found {
        dispatch(Event::ValueDone(Cow::Borrowed(value)));
    } else {
        dispatch(Event::Value(Cow::Borrowed(value)));
    }
    *i = &input[value_end_no_trailing_whitespace..];
    Ok(())
}

/// Parse one or more spaces or horizontal tabs.
///
/// At least one space or horizontal tab must be present at the current cursor.
/// On success, `i` is advanced past the whitespace run and the returned
/// [`BStr`] borrows the consumed bytes.
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
/// the returned [`BStr`] borrows the consumed bytes.
fn take_newlines1<'i>(i: &mut &'i [u8]) -> ParseResult<&'i BStr> {
    let mut c = *i;
    let input = c;
    let mut cursor = 0usize;
    while cursor < input.len() {
        if input[cursor..].starts_with(b"\r\n") {
            cursor += 2;
        } else if input[cursor] == b'\n' {
            cursor += 1;
        } else {
            break;
        }
    }
    if cursor == 0 {
        return Err(());
    }
    c = &input[cursor..];
    *i = c;
    Ok(input[..cursor].as_bstr())
}

#[cfg(test)]
mod tests;
