use std::ffi::OsString;

use bstr::{BStr, BString};

/// The result of [`command_line()`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Leading environment assignments, without the separating `=`. Names are ASCII shell identifiers.
    pub env: Vec<(String, OsString)>,
    /// The command to execute.
    pub command: OsString,
    /// The arguments to pass to the command.
    pub args: Vec<OsString>,
}

/// The error returned when a command line cannot be parsed into a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// A quote was opened but never closed.
    MissingClosingQuote,
    /// An unquoted backslash was not followed by a byte to escape.
    MissingEscapedByte,
    /// The input contains no command to execute.
    MissingCommand,
    /// The command, an argument, or an environment value cannot be represented as an OS string on this platform.
    UnrepresentableOsString,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Error::MissingClosingQuote => "missing closing quote",
            Error::MissingEscapedByte => "missing byte after escape",
            Error::MissingCommand => "missing command",
            Error::UnrepresentableOsString => {
                "command, argument, or environment value cannot be represented as an OS string"
            }
        })
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Quote {
    Single,
    Double,
}

/// A parsed shell word.
struct Word {
    value: BString,
    /// The byte offset of the unquoted `=` within `value`, if it follows a valid shell identifier.
    assignment_separator: Option<usize>,
}

/// Split `input` into leading environment assignments and the command with its arguments.
///
/// Whitespace, quotes, escapes, line continuations, and comments follow POSIX `sh`ell word-splitting rules. Shell
/// expansions and operators are not interpreted. An assignment is recognized only when its name is an unquoted
/// shell identifier. Assignment-only input is rejected because it contains no command to execute. Environment
/// assignment names are strings, while their values, the command, and arguments are converted losslessly to OS
/// strings or rejected if the platform cannot represent them.
pub fn command_line(input: &BStr) -> Result<Outcome, Error> {
    let mut words = parse_words(input)?;
    let assignment_count = words
        .iter()
        .take_while(|word| word.assignment_separator.is_some())
        .count();
    let mut args = words.split_off(assignment_count).into_iter().map(|word| word.value);
    let command = into_os_string(args.next().ok_or(Error::MissingCommand)?)?;
    let env = words
        .into_iter()
        .map(|word| {
            let separator = word.assignment_separator.expect("only recognized assignments remain");
            Ok((
                String::from_utf8(word.value[..separator].to_owned())
                    .expect("shell assignment names contain only ASCII bytes"),
                into_os_string(word.value[separator + 1..].to_owned().into())?,
            ))
        })
        .collect::<Result<_, Error>>()?;
    Ok(Outcome {
        env,
        command,
        args: args.map(into_os_string).collect::<Result<_, _>>()?,
    })
}

pub(crate) fn arguments(input: &BStr) -> Result<Vec<OsString>, Error> {
    parse_words(input)?
        .into_iter()
        .map(|word| into_os_string(word.value))
        .collect()
}

fn parse_words(input: &BStr) -> Result<Vec<Word>, Error> {
    let mut words = Vec::new();
    let mut value = BString::default();
    let mut assignment_possible = true;
    let mut assignment_separator = None;
    let mut word_started = false;
    let mut quote = None;
    let mut bytes = input.iter().copied();

    while let Some(byte) = bytes.next() {
        match quote {
            Some(Quote::Single) => {
                if byte == b'\'' {
                    quote = None;
                } else {
                    value.push(byte);
                }
            }
            Some(Quote::Double) => match byte {
                b'"' => quote = None,
                b'\\' => match bytes.next() {
                    Some(b'\n') => {}
                    Some(next @ (b'$' | b'`' | b'"' | b'\\')) => value.push(next),
                    Some(next) => {
                        value.push(b'\\');
                        value.push(next);
                    }
                    None => return Err(Error::MissingClosingQuote),
                },
                _ => value.push(byte),
            },
            None => match byte {
                b' ' | b'\t' | b'\n' => {
                    if word_started {
                        words.push(Word {
                            value: std::mem::take(&mut value),
                            assignment_separator,
                        });
                        (assignment_possible, assignment_separator, word_started) = (true, None, false);
                    }
                }
                b'#' if !word_started => {
                    bytes.by_ref().find(|byte| *byte == b'\n');
                }
                b'\'' => (assignment_possible, quote, word_started) = (false, Some(Quote::Single), true),
                b'"' => (assignment_possible, quote, word_started) = (false, Some(Quote::Double), true),
                b'\\' => match bytes.next() {
                    Some(b'\n') => {}
                    Some(next) => {
                        (assignment_possible, word_started) = (false, true);
                        value.push(next);
                    }
                    None => return Err(Error::MissingEscapedByte),
                },
                _ => {
                    word_started = true;
                    push_unquoted(&mut value, &mut assignment_possible, &mut assignment_separator, byte);
                }
            },
        }
    }
    if quote.is_some() {
        return Err(Error::MissingClosingQuote);
    }
    if word_started {
        words.push(Word {
            value,
            assignment_separator,
        });
    }
    Ok(words)
}

/// Append an unquoted byte while tracking whether the word starts with a valid shell
/// assignment name and where its `=` occurs.
fn push_unquoted(
    value: &mut BString,
    assignment_possible: &mut bool,
    assignment_separator: &mut Option<usize>,
    byte: u8,
) {
    if assignment_separator.is_none() && *assignment_possible {
        if value.is_empty() {
            *assignment_possible = byte == b'_' || byte.is_ascii_alphabetic();
        } else if byte == b'=' {
            *assignment_separator = Some(value.len());
        } else if byte != b'_' && !byte.is_ascii_alphanumeric() {
            *assignment_possible = false;
        }
    }
    value.push(byte);
}

fn into_os_string(value: BString) -> Result<OsString, Error> {
    gix_path::try_from_bstring(value)
        .map(std::path::PathBuf::into_os_string)
        .map_err(|_| Error::UnrepresentableOsString)
}
