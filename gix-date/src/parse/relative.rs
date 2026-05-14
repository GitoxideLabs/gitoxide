use std::{str::FromStr, time::SystemTime};

use crate::Error;
use gix_error::{Exn, ResultExt, ValidationError, ensure};
use jiff::{Span, Timestamp, Zoned, tz::TimeZone};

pub fn parse(input: &str, now: Option<SystemTime>) -> Option<Result<Zoned, Exn<Error>>> {
    // First try named dates
    if let Some(result) = parse_named(input, now) {
        return Some(result);
    }

    // Then try numeric relative dates
    parse_ago(input).map(|result| -> Result<Zoned, Exn<Error>> {
        let span = result?;
        // This was an error case in a previous version of this code, where
        // it would fail when converting from a negative signed integer
        // to an unsigned integer. This preserves that failure case even
        // though the code below handles it okay.
        ensure!(!span.is_negative(), ValidationError::new(""));
        subtract_span(now, span)
    })
}

/// Parse named relative dates like "now", "today", "yesterday".
fn parse_named(input: &str, now: Option<SystemTime>) -> Option<Result<Zoned, Exn<Error>>> {
    let input = input.trim();
    let span = if input.eq_ignore_ascii_case("now") {
        Span::new()
    } else if input.eq_ignore_ascii_case("today") {
        // "today" is treated the same as "now" (current time) for simplicity
        Span::new()
    } else if input.eq_ignore_ascii_case("yesterday") {
        Span::new().try_days(1).ok()?
    } else {
        return None;
    };

    Some(subtract_span(now, span))
}

fn parse_ago(input: &str) -> Option<Result<Span, Exn<Error>>> {
    let mut split = input.split_whitespace();
    let units = i64::from_str(split.next()?).ok()?;
    let period = split.next()?;
    if split.next()? != "ago" {
        return None;
    }
    span(period, units)
}

fn subtract_span(now: Option<SystemTime>, span: Span) -> Result<Zoned, Exn<ValidationError>> {
    let now = now.ok_or(ValidationError::new("Missing current time"))?;
    let ts: Timestamp = Timestamp::try_from(now).or_raise(|| Error::new("Could not convert current time"))?;
    // N.B. This matches the behavior of this code when it was
    // written with `time`, but we might consider using the system
    // time zone here. If we did, then it would implement "1 day
    // ago" correctly, even when it crosses DST transitions. Since
    // we're in the UTC time zone here, which has no DST, 1 day is
    // in practice always 24 hours. ---AG
    let zdt = ts.to_zoned(TimeZone::UTC);
    zdt.checked_sub(span)
        .or_raise(|| Error::new(format!("Failed to subtract {zdt} from {span}")))
}

fn span(period: &str, units: i64) -> Option<Result<Span, Exn<Error>>> {
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
    Some(result.or_raise(|| Error::new(format!("Couldn't parse span from '{period} {units}'"))))
}
