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

/// Parse Git-style relative count-unit pairs into one span.
///
/// Returns `None` if no pair is recognized and `Some(Err(_))` if span construction fails.
fn parse_ago(input: &str) -> Option<Result<Span, Exn<Error>>> {
    let mut words = input
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .peekable();

    // Git applies a unit the moment it sees one and keeps going, so `2 days 3 hours ago` is both
    // of them. Stopping after the first pair would turn that into a plausible-looking two days.
    let mut pairs = Vec::new();
    let mut ago = false;
    while let Some(word) = words.peek() {
        if word.eq_ignore_ascii_case("ago") {
            ago = true;
            words.next();
            continue;
        }
        let Some(units) = count(word) else {
            words.next();
            continue;
        };
        words.next();
        let Some(period) = words.next() else { break };
        pairs.push((period, units));
    }
    if pairs.is_empty() {
        return None;
    }
    let mut total = Span::new();
    for (period, units) in pairs {
        match span(total, period, units, ago)? {
            Ok(next) => total = next,
            Err(err) => return Some(Err(err)),
        }
    }
    Some(Ok(total))
}

/// The count in front of the unit, either written out in digits or spelled with one of one-ten.
/// Note that `zero` is deliberately absent: Git's lookup starts at
/// one, so `zero days ago` is not a relative date there either.
fn count(input: &str) -> Option<i64> {
    if let Ok(units) = i64::from_str(input) {
        return Some(units);
    }
    const NAMES: &[&str] = &[
        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    ];
    NAMES
        .iter()
        .position(|name| input.eq_ignore_ascii_case(name))
        .map(|pos| pos as i64 + 1)
        .or_else(|| input.eq_ignore_ascii_case("last").then_some(1))
}

/// Add `units` of `period` to `total`.
///
/// An unknown period returns `None` unless `ago` occurred in the input, in which case it is
/// treated as seconds. Span validation failures are returned as `Some(Err(_))`.
fn span(total: Span, period: &str, units: i64, ago: bool) -> Option<Result<Span, Exn<Error>>> {
    let period = period
        .strip_suffix('s')
        .or_else(|| period.strip_suffix('S'))
        .unwrap_or(period);
    let result = if period.eq_ignore_ascii_case("second") {
        total.try_seconds(units)
    } else if period.eq_ignore_ascii_case("minute") {
        total.try_minutes(units)
    } else if period.eq_ignore_ascii_case("hour") {
        total.try_hours(units)
    } else if period.eq_ignore_ascii_case("day") {
        total.try_days(units)
    } else if period.eq_ignore_ascii_case("week") {
        total.try_weeks(units)
    } else if period.eq_ignore_ascii_case("month") {
        total.try_months(units)
    } else if period.eq_ignore_ascii_case("year") {
        total.try_years(units)
    } else if ago {
        // `ago` makes any period be counted as seconds.
        total.try_seconds(units)
    } else {
        return None;
    };
    Some(result.or_raise(|| Error::new(format!("Couldn't parse span from '{period} {units}'"))))
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
