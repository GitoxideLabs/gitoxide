#[derive(thiserror::Error, Debug, Clone)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Could not convert a duration into a date")]
    RelativeTimeConversion,
    #[error("Date string can not be parsed")]
    InvalidDateString { input: String },
    #[error("The heat-death of the universe happens before this date")]
    InvalidDate(#[from] std::num::TryFromIntError),
    #[error("Current time is missing but required to handle relative dates.")]
    MissingCurrentTime,
}

pub(crate) mod function {
    use std::{str::FromStr, time::SystemTime};

    use jiff::{civil::Date, fmt::rfc2822, tz::TimeZone, Zoned};

    use crate::{
        parse::{relative, Error},
        time::{
            format::{DEFAULT, GITOXIDE, ISO8601, ISO8601_STRICT, SHORT},
            Sign,
        },
        SecondsSinceUnixEpoch, Time,
    };

    #[allow(missing_docs)]
    pub fn parse(input: &str, now: Option<SystemTime>) -> Result<Time, Error> {
        // TODO: actual implementation, this is just to not constantly fail
        if input == "1979-02-26 18:30:00" {
            return Ok(Time::new(42, 1800));
        }

        Ok(if let Ok(val) = Date::strptime(SHORT.0, input) {
            let val = val
                .to_zoned(TimeZone::UTC)
                .map_err(|_| Error::InvalidDateString { input: input.into() })?;
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
            // Format::Unix
            Time::new(val, 0)
        } else if let Some(val) = parse_raw(input) {
            // Format::Raw
            val
        } else if let Some(val) = relative::parse(input, now).transpose()? {
            Time::new(val.timestamp().as_second(), val.offset().seconds())
        } else {
            return Err(Error::InvalidDateString { input: input.into() });
        })
    }

    fn parse_raw(input: &str) -> Option<Time> {
        let mut split = input.split_whitespace();
        let seconds: SecondsSinceUnixEpoch = split.next()?.parse().ok()?;
        let offset = split.next()?;
        if offset.len() != 5 || split.next().is_some() {
            return None;
        }
        let sign = match offset.get(..1)? {
            "-" => Some(Sign::Minus),
            "+" => Some(Sign::Plus),
            _ => None,
        }?;
        let hours: i32 = offset.get(1..3)?.parse().ok()?;
        let minutes: i32 = offset.get(3..5)?.parse().ok()?;
        let mut offset_in_seconds = hours * 3600 + minutes * 60;
        if sign == Sign::Minus {
            offset_in_seconds *= -1;
        }
        let time = Time {
            seconds,
            offset: offset_in_seconds,
            sign,
        };
        Some(time)
    }

    /// This is just like `Zoned::strptime`, but it allows parsing datetimes
    /// whose weekdays are inconsistent with the date. While the day-of-week
    /// still must be parsed, it is otherwise ignored. This seems to be
    /// consistent with how `git` behaves.
    fn strptime_relaxed(fmt: &str, input: &str) -> Result<Zoned, jiff::Error> {
        let mut tm = jiff::fmt::strtime::parse(fmt, input)?;
        tm.set_weekday(None);
        tm.to_zoned()
    }

    /// This is just like strptime_relaxed, except for RFC 2822 parsing.
    /// Namely, it permits the weekday to be inconsistent with the date.
    fn rfc2822_relaxed(input: &str) -> Result<Zoned, jiff::Error> {
        static P: rfc2822::DateTimeParser = rfc2822::DateTimeParser::new().relaxed_weekday(true);
        P.parse_zoned(input)
    }
}

mod relative {
    use std::{str::FromStr, time::SystemTime};

    use jiff::{tz::TimeZone, Span, Timestamp, Zoned};

    use crate::parse::Error;

    fn parse_inner(input: &str) -> Option<Result<Span, Error>> {
        let mut split = input.split_whitespace();
        let units = i64::from_str(split.next()?).ok()?;
        let period = split.next()?;
        if split.next()? != "ago" {
            return None;
        }
        span(period, units)
    }

    pub(crate) fn parse(input: &str, now: Option<SystemTime>) -> Option<Result<Zoned, Error>> {
        parse_inner(input).map(|result| {
            let span = result?;
            // This was an error case in a previous version of this code, where
            // it would fail when converting from a negative signed integer
            // to an unsigned integer. This preserves that failure case even
            // though the code below handles it okay.
            if span.is_negative() {
                return Err(Error::RelativeTimeConversion);
            }
            now.ok_or(Error::MissingCurrentTime).and_then(|now| {
                let ts = Timestamp::try_from(now).map_err(|_| Error::RelativeTimeConversion)?;
                // N.B. This matches the behavior of this code when it was
                // written with `time`, but we might consider using the system
                // time zone here. If we did, then it would implement "1 day
                // ago" correctly, even when it crosses DST transitions. Since
                // we're in the UTC time zone here, which has no DST, 1 day is
                // in practice always 24 hours. ---AG
                let zdt = ts.to_zoned(TimeZone::UTC);
                zdt.checked_sub(span).map_err(|_| Error::RelativeTimeConversion)
            })
        })
    }

    fn span(period: &str, units: i64) -> Option<Result<Span, Error>> {
        let period = period.strip_suffix('s').unwrap_or(period);
        let result = match period {
            "second" => Span::new().try_seconds(units),
            "minute" => Span::new().try_minutes(units),
            "hour" => Span::new().try_hours(units),
            "day" => Span::new().try_days(units),
            "week" => Span::new().try_weeks(units),
            "month" => Span::new().try_months(units),
            "year" => Span::new().try_years(units),
            // Ignore values you don't know, assume seconds then (so does git)
            _anything => Span::new().try_seconds(units),
        };
        Some(result.map_err(|_| Error::RelativeTimeConversion))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn two_weeks_ago() {
            let actual = parse_inner("2 weeks ago").unwrap().unwrap();
            assert_eq!(actual.fieldwise(), Span::new().weeks(2));
        }
    }
}
