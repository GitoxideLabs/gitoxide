use std::{str::FromStr, time::SystemTime};

use jiff::{civil::Date, fmt::rfc2822, tz::TimeZone, Zoned};

use crate::parse::git::parse_git_date_format;
use crate::parse::raw::parse_raw;
use crate::{
    parse::relative,
    time::format::{DEFAULT, GITOXIDE, ISO8601, ISO8601_STRICT, SHORT},
    Error, OffsetInSeconds, SecondsSinceUnixEpoch, Time,
};
use gix_error::{Exn, ResultExt};

/// Parse `input` as any time that Git can parse when inputting a date.
///
/// ## Examples
///
/// ### 1. SHORT Format
///
/// *   `2018-12-24`
/// *   `1970-01-01`
/// *   `1950-12-31`
/// *   `2024-12-31`
///
/// ### 2. RFC2822 Format
///
/// *   `Thu, 18 Aug 2022 12:45:06 +0800`
/// *   `Mon Oct 27 10:30:00 2023 -0800`
///
/// ### 3. GIT_RFC2822 Format
///
/// *   `Thu, 8 Aug 2022 12:45:06 +0800`
/// *   `Mon Oct 27 10:30:00 2023 -0800` (Note the single-digit day)
///
/// ### 4. ISO8601 Format
///
/// *   `2022-08-17 22:04:58 +0200`
/// *   `1970-01-01 00:00:00 -0500`
///
/// ### 5. ISO8601_STRICT Format
///
/// *   `2022-08-17T21:43:13+08:00`
///
/// ### 6. UNIX Timestamp (Seconds Since Epoch)
///
/// *   `123456789`
/// *   `0` (January 1, 1970 UTC)
/// *   `-1000`
/// *   `1700000000`
///
/// ### 7. Commit Header Format
///
/// *   `1745582210 +0200`
/// *   `1660874655 +0800`
/// *   `-1660874655 +0800`
///
/// See also the [`parse_header()`].
///
/// ### 8. GITOXIDE Format
///
/// *   `Thu Sep 04 2022 10:45:06 -0400`
/// *   `Mon Oct 27 2023 10:30:00 +0000`
///
/// ### 9. DEFAULT Format
///
/// *   `Thu Sep 4 10:45:06 2022 -0400`
/// *   `Mon Oct 27 10:30:00 2023 +0000`
///
/// ### 10. Relative Dates (e.g., "2 minutes ago", "1 hour from now")
///
/// These dates are parsed *relative to a `now` timestamp*. The examples depend entirely on the value of `now`.
/// If `now` is October 27, 2023 at 10:00:00 UTC:
///     *   `2 minutes ago` (October 27, 2023 at 09:58:00 UTC)
///     *   `3 hours ago` (October 27, 2023 at 07:00:00 UTC)
pub fn parse(input: &str, now: Option<SystemTime>) -> Result<Time, Exn<Error>> {
    Ok(if let Ok(val) = Date::strptime(SHORT.0, input) {
        let val = val
            .to_zoned(TimeZone::UTC)
            .or_raise(|| Error::new_with_input("Timezone conversion failed", input))?;
        Time::new(val.timestamp().as_second(), val.offset().seconds())
    } else if let Ok(val) = rfc2822_relaxed(input) {
        Time::new(val.timestamp().as_second(), val.offset().seconds())
    } else if let Ok(val) = strptime_relaxed(ISO8601.0, input) {
        Time::new(val.timestamp().as_second(), val.offset().seconds())
    } else if let Ok(val) = strptime_relaxed(ISO8601_STRICT.0, input) {
        Time::new(val.timestamp().as_second(), val.offset().seconds())
    } else if let Ok(val) = strptime_relaxed(GITOXIDE.0, input) {
        Time::new(val.timestamp().as_second(), val.offset().seconds())
    } else if let Ok(val) = strptime_relaxed(DEFAULT.0, input) {
        Time::new(val.timestamp().as_second(), val.offset().seconds())
    } else if let Ok(val) = SecondsSinceUnixEpoch::from_str(input) {
        Time::new(val, 0)
    } else if let Some(val) = parse_git_date_format(input) {
        val
    } else if let Some(val) = relative::parse(input, now).transpose()? {
        Time::new(val.timestamp().as_second(), val.offset().seconds())
    } else if let Some(val) = parse_raw(input) {
        // Format::Raw
        val
    } else {
        return Err(Error::new_with_input("Unknown date format", input))?;
    })
}

/// Unlike [`parse()`] which handles all kinds of input, this function only parses the commit-header format
/// like `1745582210 +0200`.
///
/// Note that failure to parse the time zone isn't fatal, instead it will default to `0`. To know if
/// the time is wonky, serialize the return value to see if it matches the `input.`
pub fn parse_header(input: &str) -> Option<Time> {
    pub enum Sign {
        Plus,
        Minus,
    }
    fn parse_offset(offset: &str) -> Option<OffsetInSeconds> {
        if (offset.len() != 5) && (offset.len() != 7) {
            return None;
        }
        let sign = match offset.get(..1)? {
            "-" => Some(Sign::Minus),
            "+" => Some(Sign::Plus),
            _ => None,
        }?;
        if offset.as_bytes().get(1).is_some_and(|b| !b.is_ascii_digit()) {
            return None;
        }
        let hours: i32 = offset.get(1..3)?.parse().ok()?;
        let minutes: i32 = offset.get(3..5)?.parse().ok()?;
        let offset_seconds: i32 = if offset.len() == 7 {
            offset.get(5..7)?.parse().ok()?
        } else {
            0
        };
        let mut offset_in_seconds = hours * 3600 + minutes * 60 + offset_seconds;
        if matches!(sign, Sign::Minus) {
            offset_in_seconds *= -1;
        }
        Some(offset_in_seconds)
    }

    if input.contains(':') {
        return None;
    }
    let mut split = input.split_whitespace();
    let seconds = split.next()?;
    let seconds = match seconds.parse::<SecondsSinceUnixEpoch>() {
        Ok(s) => s,
        Err(_err) => {
            // Inefficient, but it's not the common case.
            let first_digits: String = seconds.chars().take_while(char::is_ascii_digit).collect();
            first_digits.parse().ok()?
        }
    };
    let offset = match split.next() {
        None => 0,
        Some(offset) => {
            if split.next().is_some() {
                0
            } else {
                parse_offset(offset).unwrap_or_default()
            }
        }
    };
    let time = Time { seconds, offset };
    Some(time)
}

/// This is just like `Zoned::strptime`, but it allows parsing datetimes
/// whose weekdays are inconsistent with the date. While the day-of-week
/// still must be parsed, it is otherwise ignored. This seems to be
/// consistent with how `git` behaves.
fn strptime_relaxed(fmt: &str, input: &str) -> std::result::Result<Zoned, jiff::Error> {
    let mut tm = jiff::fmt::strtime::parse(fmt, input)?;
    tm.set_weekday(None);
    tm.to_zoned()
}

/// This is just like strptime_relaxed, except for RFC 2822 parsing.
/// Namely, it permits the weekday to be inconsistent with the date.
fn rfc2822_relaxed(input: &str) -> std::result::Result<Zoned, jiff::Error> {
    static P: rfc2822::DateTimeParser = rfc2822::DateTimeParser::new().relaxed_weekday(true);
    P.parse_zoned(input)
}
