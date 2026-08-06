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
    // For the `<count> <unit>` shapes handled here, any byte that is neither a digit nor a letter
    // separates the parts, because `approxidate_alpha()` in Git's `date.c` ends a word at the
    // first byte that is not a letter. So `1.hour.ago` and `1 hour ago` are the same to it.
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
            break;
        }
        let Some(units) = count(word) else { break };
        words.next();
        let Some(period) = words.next() else { break };
        pairs.push((period, units));
    }
    if pairs.is_empty() {
        return None;
    }
    // `ago` may still be further along, past a word that is no count of ours.
    ago |= words.any(|word| word.eq_ignore_ascii_case("ago"));

    // The trailing `ago` is not required: `2 days` is `2 days ago` to Git. It is what permits an
    // unknown unit to count as seconds, though, or `1745582210 +0200` would read as a count of
    // `1745582210` in the unknown unit `0200` and never reach the raw-format parser that owns it.
    let mut total = Span::new();
    for (period, units) in pairs {
        match span(total, period, units, ago)? {
            Ok(next) => total = next,
            Err(err) => return Some(Err(err)),
        }
    }
    Some(Ok(total))
}

/// The count in front of the unit, either written out in digits or spelled with one of the names
/// Git keeps in `number_name[]`. Note that `zero` is deliberately absent: Git's lookup starts at
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
        // `last week` is `1 week ago` to Git, which sets the count to one for it.
        .or_else(|| input.eq_ignore_ascii_case("last").then_some(1))
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

fn span(total: Span, period: &str, units: i64, ago: bool) -> Option<Result<Span, Exn<Error>>> {
    let period = period
        .strip_suffix('s')
        .or_else(|| period.strip_suffix('S'))
        .unwrap_or(period);
    // Git compares unit names with `match_string()`, which folds case.
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
        // An unknown unit still counts as seconds, but only once `ago` has marked the input as a
        // relative date. Note that Git does *not* read it as seconds, despite what this comment
        // used to claim: it leaves the count pending, where it ends up standing in for a field of
        // the date itself, so how far `1 banana ago` lands from now depends on today's date.
        total.try_seconds(units)
    } else {
        return None;
    };
    Some(result.or_raise(|| Error::new(format!("Couldn't parse span from '{period} {units}'"))))
}
