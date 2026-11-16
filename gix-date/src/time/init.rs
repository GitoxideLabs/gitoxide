use crate::{OffsetInSeconds, SecondsSinceUnixEpoch, Time};

/// Instantiation
impl Time {
    /// Create a new instance from seconds and offset.
    pub fn new(seconds: SecondsSinceUnixEpoch, offset: OffsetInSeconds) -> Self {
        Time { seconds, offset }
    }

    /// Return the current time without figuring out a timezone offset
    pub fn now_utc() -> Self {
        let seconds = jiff::Timestamp::now().as_second();
        Self { seconds, offset: 0 }
    }

    /// Return the current local time, or `None` if the local time wasn't available.
    pub fn now_local() -> Option<Self> {
        Some(Self::now_local_or_utc())
    }

    /// Return the current local time, or the one at UTC if the local time wasn't available.
    pub fn now_local_or_utc() -> Self {
        let zdt = jiff::ZonedDateTime::now();
        let seconds = zdt.timestamp().as_second();
        let offset = zdt.offset().seconds();
        Self { seconds, offset }
    }
}
