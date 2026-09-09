//! Reformat a git-config file with normalized, sanitized whitespace.
//!
//! This operates purely on the syntactic [event stream](crate::parse::Events) of a single
//! file. `include`/`includeIf` directives are *never* resolved here - those are only acted upon
//! when constructing a [`File`](crate::File) - so the formatter is "flat" by construction.
//!
//! Values, comments and section headers are reproduced verbatim; only insignificant whitespace,
//! newlines and the `=` separator are rewritten according to [`Options`](crate::parse::format::Options).

use bstr::BString;

use crate::parse::{self, EventRef};

/// How key/value lines beneath a section header are indented.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Indentation {
    /// A single horizontal tab per line - git's de-facto writer style.
    Tab,
    /// The given number of spaces per line.
    Spaces(usize),
    /// No indentation at all.
    None,
}

/// Which newline sequence to write between lines.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Newline {
    /// Use the first newline sequence found in the input, falling back to `\n` if none is present.
    Detect,
    /// Always use a Unix newline (`\n`).
    Lf,
    /// Always use a Windows newline (`\r\n`).
    CrLf,
}

/// Options controlling [`normalize()`].
///
/// The defaults are intentionally conservative: they tidy the common sources of noise (stray
/// indentation, spacing around `=`, trailing whitespace, missing final newline) while leaving
/// blank lines and the substance of the file untouched.
///
/// Note that trailing whitespace at the end of a line is always removed - it is never significant
/// in git-config syntax - so there is no option to retain it.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Options {
    /// How to indent key/value (and comment) lines beneath a section header.
    pub indentation: Indentation,
    /// If `true`, place a single space on each side of the `=` separator (`a = b`);
    /// if `false`, emit a bare `=` (`a=b`).
    pub spaces_around_separator: bool,
    /// Which newline sequence to emit between lines.
    pub newline: Newline,
    /// If `true`, ensure a non-empty file ends with exactly one newline.
    pub ensure_trailing_newline: bool,
    /// If `Some(n)`, cap runs of consecutive blank lines at `n`. `None` (the default) leaves
    /// blank lines exactly as they are.
    pub max_consecutive_blank_lines: Option<usize>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            indentation: Indentation::Spaces(2),
            spaces_around_separator: true,
            newline: Newline::Detect,
            ensure_trailing_newline: true,
            max_consecutive_blank_lines: None,
        }
    }
}

/// Parse `input` as a single git-config file and return it with whitespace normalized per `options`.
///
/// Includes are never resolved. Values, comments and section headers are preserved byte-for-byte;
/// only insignificant whitespace, newlines and the `=` separator are rewritten.
///
/// # Errors
///
/// Returns a [`gix_error::ValidationError`] if `input` is not a syntactically valid git-config file.
pub fn normalize(input: &[u8], options: &Options) -> Result<BString, gix_error::ValidationError> {
    let parsed = parse::Events::from_bytes(input, None)?;
    let events: Vec<_> = parsed.iter().collect();
    Ok(normalize_events(&events, options))
}

fn detect_newline(events: &[EventRef<'_>]) -> &'static [u8] {
    for (index, event) in events.iter().enumerate() {
        if let EventRef::Newline(n) = event {
            let cr_is_attached_to_comment = index.checked_sub(1).is_some_and(
                |previous| matches!(events[previous], EventRef::Comment { text, .. } if text.ends_with(b"\r")),
            );
            return if n.contains(&b'\r') || cr_is_attached_to_comment {
                b"\r\n"
            } else {
                b"\n"
            };
        }
    }
    b"\n"
}

fn normalize_events(events: &[EventRef<'_>], opts: &Options) -> BString {
    let newline: &[u8] = match opts.newline {
        Newline::Detect => detect_newline(events),
        Newline::Lf => b"\n",
        Newline::CrLf => b"\r\n",
    };
    let indent: Vec<u8> = match opts.indentation {
        Indentation::Tab => vec![b'\t'],
        Indentation::Spaces(n) => vec![b' '; n],
        Indentation::None => Vec::new(),
    };

    let mut out: Vec<u8> = Vec::with_capacity(events.len() * 8);
    let mut in_section = false;
    let mut line_has_content = false;
    let mut consecutive_blank_lines = 0usize;
    let mut i = 0;

    while i < events.len() {
        match events[i] {
            // Standalone, insignificant whitespace is dropped; we synthesize whitespace
            // deterministically around the structural events below.
            EventRef::Whitespace(_) => {
                i += 1;
            }
            EventRef::SectionHeader { .. } => {
                events[i].write_to(&mut out).expect("write to Vec is infallible");
                in_section = true;
                line_has_content = true;
                consecutive_blank_lines = 0;
                i += 1;
            }
            EventRef::SectionValueName(_) => {
                if in_section && !line_has_content {
                    out.extend_from_slice(&indent);
                }
                events[i].write_to(&mut out).expect("write to Vec is infallible");
                line_has_content = true;
                consecutive_blank_lines = 0;
                i += 1;
            }
            EventRef::KeyValueSeparator => {
                if opts.spaces_around_separator {
                    out.extend_from_slice(b" = ");
                } else {
                    out.push(b'=');
                }
                line_has_content = true;
                consecutive_blank_lines = 0;
                i += 1;
            }
            EventRef::Value(_) | EventRef::ValueNotDone(_) | EventRef::ValueDone(_) => {
                events[i].write_to(&mut out).expect("write to Vec is infallible");
                line_has_content = true;
                consecutive_blank_lines = 0;
                i += 1;
            }
            EventRef::Comment { tag, text } => {
                if line_has_content {
                    // Inline comment trailing a value/header: one space before the marker.
                    out.push(b' ');
                } else if in_section {
                    out.extend_from_slice(&indent);
                }
                out.push(tag);
                let text: &[u8] = text.as_ref();
                let newline_follows = events
                    .get(i + 1)
                    .is_some_and(|event| matches!(event, EventRef::Newline(_)));
                out.extend_from_slice(if newline_follows {
                    text.strip_suffix(b"\r").unwrap_or(text)
                } else {
                    text
                });
                line_has_content = true;
                consecutive_blank_lines = 0;
                i += 1;
            }
            EventRef::Newline(n) => {
                for _ in 0..n.iter().filter(|&&b| b == b'\n').count() {
                    let is_blank_line = !line_has_content;
                    let should_emit = opts
                        .max_consecutive_blank_lines
                        .is_none_or(|max_blank| !is_blank_line || consecutive_blank_lines < max_blank);
                    if should_emit {
                        out.extend_from_slice(newline);
                    }
                    if is_blank_line {
                        consecutive_blank_lines = consecutive_blank_lines.saturating_add(1);
                    } else {
                        consecutive_blank_lines = 0;
                    }
                    line_has_content = false;
                }
                i += 1;
            }
        }
    }

    if opts.ensure_trailing_newline && !out.is_empty() {
        while out.last() == Some(&b'\n') || out.last() == Some(&b'\r') {
            out.pop();
        }
        out.extend_from_slice(newline);
    }

    out.into()
}
