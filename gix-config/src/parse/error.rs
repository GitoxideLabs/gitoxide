use std::fmt::Display;

use crate::parse::Error;

#[derive(PartialEq, Debug)]
pub(crate) enum Kind {
    Parse {
        line_number: usize,
        last_attempted_parser: ParseNode,
        parsed_until: bstr::BString,
    },
    InputTooLarge {
        actual: usize,
    },
}

/// A list of parsers that parsing can fail on. This is used for pretty-printing errors
#[derive(PartialEq, Debug, Clone, Copy)]
pub(crate) enum ParseNode {
    SectionHeader,
    Name,
    Value,
}

impl Display for ParseNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SectionHeader => write!(f, "section header"),
            Self::Name => write!(f, "name"),
            Self::Value => write!(f, "value"),
        }
    }
}

impl Error {
    pub(crate) fn parse(line_number: usize, last_attempted_parser: ParseNode, parsed_until: bstr::BString) -> Self {
        Self {
            kind: Kind::Parse {
                line_number,
                last_attempted_parser,
                parsed_until,
            },
        }
    }

    pub(crate) fn input_too_large(actual: usize) -> Self {
        Self {
            kind: Kind::InputTooLarge { actual },
        }
    }

    /// The one-indexed line number where the error occurred. This is determined
    /// by the number of newlines that were successfully parsed.
    #[must_use]
    pub const fn line_number(&self) -> usize {
        match self.kind {
            Kind::Parse { line_number, .. } => line_number + 1,
            Kind::InputTooLarge { .. } => 1,
        }
    }

    /// The data that was left unparsed, which contains the cause of the parse error.
    #[must_use]
    pub fn remaining_data(&self) -> &[u8] {
        match &self.kind {
            Kind::Parse { parsed_until, .. } => parsed_until,
            Kind::InputTooLarge { .. } => &[],
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (line_number, last_attempted_parser, parsed_until) = match &self.kind {
            Kind::InputTooLarge { actual } => {
                return write!(
                    f,
                    "Configuration input is {actual} bytes large, but at most {} bytes are supported",
                    u32::MAX
                );
            }
            Kind::Parse {
                line_number,
                last_attempted_parser,
                parsed_until,
            } => (line_number, last_attempted_parser, parsed_until),
        };
        write!(
            f,
            "Got an unexpected token on line {} while trying to parse a {}: ",
            line_number + 1,
            last_attempted_parser,
        )?;

        let data_size = parsed_until.len();
        let data = std::str::from_utf8(parsed_until);
        match (data, data_size) {
            (Ok(data), _) if data_size > 10 => {
                write!(
                    f,
                    "'{}' ... ({} characters omitted)",
                    data.chars().take(10).collect::<String>(),
                    data_size - 10
                )
            }
            (Ok(data), _) => write!(f, "'{data}'"),
            (Err(_), _) => parsed_until.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(const { &gix_error::ClassificationMarker::VALIDATION })
    }
}
