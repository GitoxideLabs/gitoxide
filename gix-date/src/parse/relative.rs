use std::str::FromStr;

use crate::Error;
use gix_error::{Exn, ResultExt, ValidationError};
use jiff::{SignedDuration, Zoned, civil, tz::TimeZone};

pub fn parse(input: &str, now: Option<Zoned>) -> Option<Result<Zoned, Exn<Error>>> {
    // First try named dates
    if let Some(result) = parse_named(input, now.as_ref()) {
        return Some(result);
    }

    // Then try numeric relative dates
    Some(subtract_pairs(now, &parse_ago(input)?))
}

/// Parse named relative dates like "now", "today", "yesterday".
fn parse_named(input: &str, now: Option<&Zoned>) -> Option<Result<Zoned, Exn<Error>>> {
    let input = input.trim();
    let duration = if input.eq_ignore_ascii_case("now") {
        SignedDuration::ZERO
    } else if input.eq_ignore_ascii_case("today") {
        // "today" is treated the same as "now" (current time) for simplicity
        SignedDuration::ZERO
    } else if input.eq_ignore_ascii_case("yesterday") {
        SignedDuration::from_hours(24)
    } else {
        return None;
    };

    Some(subtract_duration(now, duration))
}

/// Parse Git-style relative count-unit pairs, in input order.
///
/// Returns `None` if no pair is recognized.
fn parse_ago(input: &str) -> Option<Vec<Pair<'_>>> {
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
    pairs
        .into_iter()
        .map(|(period, count)| unit(period, ago).map(|(period, unit)| Pair { period, count, unit }))
        .collect()
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

/// A single `<count> <unit>` occurrence, in input order.
struct Pair<'a> {
    /// The unit name, in singular form, for error messages.
    period: &'a str,
    /// The count in front of the unit.
    count: i64,
    /// How the pair is subtracted.
    unit: Unit,
}

/// How a unit is subtracted: Git's `date.c` counts `second` through `week` as fixed numbers of
/// seconds, while `month` and `year` step down the calendar fields and leave the day alone.
enum Unit {
    /// One unit is this many seconds, to be taken off the timestamp.
    Seconds(i64),
    /// One unit is this many months, to be taken off the year and month fields.
    Months(i64),
}

/// Classify `period`, also returning it in singular form.
///
/// An unknown period returns `None` unless `ago` occurred in the input, in which case it is
/// treated as seconds.
fn unit(period: &str, ago: bool) -> Option<(&str, Unit)> {
    let period = period
        .strip_suffix('s')
        .or_else(|| period.strip_suffix('S'))
        .unwrap_or(period);
    let unit = if period.eq_ignore_ascii_case("second") {
        Unit::Seconds(1)
    } else if period.eq_ignore_ascii_case("minute") {
        Unit::Seconds(60)
    } else if period.eq_ignore_ascii_case("hour") {
        Unit::Seconds(60 * 60)
    } else if period.eq_ignore_ascii_case("day") {
        Unit::Seconds(24 * 60 * 60)
    } else if period.eq_ignore_ascii_case("week") {
        Unit::Seconds(7 * 24 * 60 * 60)
    } else if period.eq_ignore_ascii_case("month") {
        Unit::Months(1)
    } else if period.eq_ignore_ascii_case("year") {
        Unit::Months(12)
    } else if ago {
        // `ago` makes any period be counted as seconds.
        Unit::Seconds(1)
    } else {
        return None;
    };
    Some((period, unit))
}

/// Subtract all `pairs` from `now`, in input order, like Git's `approxidate()` applies them.
///
/// Seconds-based units subtract from the timestamp, while months and years only step down the
/// respective fields and normalize later, so that repeated units accumulate and a day beyond
/// the end of the target month rolls over.
fn subtract_pairs(now: Option<Zoned>, pairs: &[Pair<'_>]) -> Result<Zoned, Exn<Error>> {
    /// The calendar and clock fields for subtraction.
    struct Fields {
        year: i16,
        month: i8,
        day: i8,
        time: civil::Time,
        timezone: TimeZone,
    }

    impl From<Zoned> for Fields {
        fn from(zdt: Zoned) -> Self {
            Fields {
                year: zdt.year(),
                month: zdt.month(),
                day: zdt.day(),
                time: zdt.time(),
                timezone: zdt.time_zone().clone(),
            }
        }
    }

    impl Fields {
        /// Turn the fields back into a point in time: a day beyond the end of the month rolls over into the
        /// following month. One month before May 31st is thus May 1st, a day after April 30th.
        fn normalize(&self) -> Result<Zoned, Exn<Error>> {
            let first_of_month = civil::Date::new(self.year, self.month, 1)
                .or_raise(|| Error::new(format!("Date lies out of range: {}-{:02}", self.year, self.month)))?;
            let days_beyond_first = SignedDuration::from_secs((i64::from(self.day) - 1) * 24 * 60 * 60);
            first_of_month
                .checked_add(days_beyond_first)
                .or_raise(|| Error::new(format!("Day {} lies out of range", self.day)))?
                .to_datetime(self.time)
                .to_zoned(self.timezone.clone())
                .or_raise(|| Error::new("Could not convert date to a point in time"))
        }
    }

    let now = now.ok_or(ValidationError::new("Missing current time"))?;
    let mut fields = Fields::from(now);
    for Pair { period, count, unit } in pairs {
        let err = || Error::new(format!("Couldn't parse span from '{period} {count}'"));
        match unit {
            Unit::Seconds(factor) => {
                let seconds = count
                    .checked_mul(*factor)
                    .map(SignedDuration::from_secs)
                    .ok_or_else(err)?;
                let ts = fields.normalize()?.timestamp().checked_sub(seconds).or_raise(err)?;
                fields = ts.to_zoned(fields.timezone.clone()).into();
            }
            Unit::Months(factor) => {
                let months = count.checked_mul(*factor).ok_or_else(err)?;
                fields = fields.normalize()?.into();
                let total = (i64::from(fields.year) * 12 + i64::from(fields.month) - 1)
                    .checked_sub(months)
                    .ok_or_else(err)?;
                fields.year = i16::try_from(total.div_euclid(12)).ok().ok_or_else(err)?;
                fields.month = i8::try_from(total.rem_euclid(12) + 1).expect("a value in 1..=12");
            }
        }
    }
    fields.normalize()
}

fn subtract_duration(now: Option<&Zoned>, duration: SignedDuration) -> Result<Zoned, Exn<ValidationError>> {
    let now = now.ok_or(ValidationError::new("Missing current time"))?;
    now.timestamp()
        .checked_sub(duration)
        .map(|timestamp| timestamp.to_zoned(now.time_zone().clone()))
        .or_raise(|| Error::new(format!("Failed to subtract {duration} from {now}")))
}
