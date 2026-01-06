use std::str::FromStr;

use smallvec::SmallVec;

use crate::Time;

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
            Error::InvalidDate(_) => write!(f, "The heat-death of the universe happens before this date"),
            Error::MissingCurrentTime => write!(f, "Current time is missing but required to handle relative dates."),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::InvalidDate(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(e: std::num::TryFromIntError) -> Self {
        Error::InvalidDate(e)
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
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crate::parse_header(s).ok_or_else(|| Error::InvalidDateString { input: s.into() })
    }
}

pub(crate) mod function;
mod git;
mod raw;
mod relative;
