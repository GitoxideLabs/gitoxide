use std::str::FromStr;

use smallvec::SmallVec;

use crate::Time;

/// Errors that can occur when parsing dates.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Error {
    RelativeTimeConversion,
    InvalidDateString { input: String },
    InvalidDate(std::num::TryFromIntError),
    MissingCurrentTime,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RelativeTimeConversion => write!(f, "Could not convert a duration into a date"),
            Error::InvalidDateString { input } => write!(f, "Date string can not be parsed: {input}"),
            Error::InvalidDate(err) => write!(f, "The heat-death of the universe happens before this date: {err}"),
            Error::MissingCurrentTime => write!(f, "Current time is missing but required to handle relative dates."),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::InvalidDate(err) => Some(err),
            _ => None,
        }
    }
}

/// A wrapper around `exn::Exn<Error>` that implements `std::error::Error`.
///
/// This type is returned by functions that integrate with external APIs requiring `std::error::Error`,
/// while internally using exn for context-aware error tracking.
#[derive(Debug)]
pub struct ParseError(exn::Exn<Error>);

impl ParseError {
    /// Create a ParseError from an exn error.
    pub fn from_exn(exn: exn::Exn<Error>) -> Self {
        Self(exn)
    }

    /// Get a reference to the underlying error.
    pub fn as_error(&self) -> &Error {
        self.0.as_error()
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_error())
    }
}

impl From<exn::Exn<Error>> for ParseError {
    fn from(exn: exn::Exn<Error>) -> Self {
        Self(exn)
    }
}

/// A container for just enough bytes to hold the largest-possible [`time`](Time) instance.
/// It's used in conjunction with
#[derive(Default, Clone)]
pub struct TimeBuf {
    buf: SmallVec<[u8; Time::MAX.size()]>,
}

impl TimeBuf {
    /// Represent this instance as standard string, serialized in a format compatible with
    /// signature fields in Git commits, also known as anything parseable as [raw format](function::parse_header()).
    pub fn as_str(&self) -> &str {
        // SAFETY: We know that serialized times are pure ASCII, a subset of UTF-8.
        //         `buf` and `len` are written only by time-serialization code.
        let time_bytes = self.buf.as_slice();
        #[allow(unsafe_code)]
        unsafe {
            std::str::from_utf8_unchecked(time_bytes)
        }
    }

    /// Clear the previous content.
    fn clear(&mut self) {
        self.buf.clear();
    }
}

impl Time {
    /// Serialize this instance into `buf`, exactly as it would appear in the header of a Git commit,
    /// and return `buf` as `&str` for easy consumption.
    pub fn to_str<'a>(&self, buf: &'a mut TimeBuf) -> &'a str {
        buf.clear();
        self.write_to(&mut buf.buf)
            .expect("write to memory of just the right size cannot fail");
        buf.as_str()
    }
}

impl FromStr for Time {
    type Err = ParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        crate::parse_header(s)
            .ok_or_else(|| Error::InvalidDateString { input: s.into() })
            .map_err(|e| ParseError::from_exn(exn::Exn::from(e)))
    }
}

pub(crate) mod function;
mod git;
mod raw;
mod relative;
