use std::str::FromStr;

use crate::Error;
use gix_error::{Exn, ResultExt, ValidationError};
use jiff::{SignedDuration, Zoned, civil};

pub fn parse(input: &str, now: Option<Zoned>) -> Option<Result<Zoned, Exn<Error>>> {
    // First try named dates
    if let Some(result) = parse_named(input, now.as_ref()) {
        return Some(result);
    }

    Some(apply_operations(now, &parse_operations(input)?))
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

/// Parse relative units and clock adjustments in input order.
fn parse_operations(input: &str) -> Option<Vec<Operation<'_>>> {
    let mut words = input
        .as_bytes()
        .chunk_by(|a, b| a.is_ascii_digit() == b.is_ascii_digit() && a.is_ascii_alphabetic() == b.is_ascii_alphabetic())
        .filter(|word| word[0].is_ascii_alphanumeric())
        .map(|word| std::str::from_utf8(word).expect("each retained chunk contains only ASCII letters or digits"));
    let ago = words.clone().any(|word| word.eq_ignore_ascii_case("ago"));
    let mut operations = Vec::new();
    while let Some(word) = words.next() {
        let operation = if word.eq_ignore_ascii_case("noon") {
            Operation::NamedTime(12)
        } else if word.eq_ignore_ascii_case("midnight") {
            Operation::NamedTime(0)
        } else if word.eq_ignore_ascii_case("tea") {
            Operation::NamedTime(17)
        } else if word.eq_ignore_ascii_case("yesterday") {
            Operation::Yesterday
        } else if word.eq_ignore_ascii_case("now") {
            Operation::Now
        } else {
            let Some(count) = count(word) else { continue };
            let Some(period) = words.next() else { break };
            let (period, unit) = unit(period, ago)?;
            Operation::Pair(Pair { period, count, unit })
        };
        operations.push(operation);
    }
    (!operations.is_empty()).then_some(operations)
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
    /// The unit name for error messages, in singular form for duration units.
    period: &'a str,
    /// The count in front of the unit.
    count: i64,
    /// How the pair is subtracted.
    unit: Unit,
}

enum Operation<'a> {
    Pair(Pair<'a>),
    NamedTime(i8),
    Yesterday,
    Now,
}

/// How a unit is subtracted: Git's `date.c` counts `second` through `week` as fixed numbers of
/// seconds, while `month` and `year` step down the calendar fields and leave the day alone.
enum Unit {
    /// One unit is this many seconds, to be taken off the timestamp.
    Seconds(i64),
    /// One unit is this many months, to be taken off the year and month fields.
    Months(i64),
    /// The nth previous occurrence of this weekday, numbered from Sunday as zero.
    Weekday(i8),
}

/// Classify `period`, also returning duration units in singular form.
///
/// An unknown period returns `None` unless `ago` occurred in the input, in which case it is
/// treated as seconds.
fn unit(period: &str, ago: bool) -> Option<(&str, Unit)> {
    const WEEKDAYS: [&str; 7] = [
        "sundays",
        "mondays",
        "tuesdays",
        "wednesdays",
        "thursdays",
        "fridays",
        "saturdays",
    ];
    if period.len() >= 3
        && let Some(day) = WEEKDAYS.iter().position(|name| {
            name.get(..period.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(period))
        })
    {
        return Some((period, Unit::Weekday(day as i8)));
    }
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

/// Apply `operations` to `now` in input order, like Git's `approxidate()`.
///
/// Seconds-based units subtract from the timestamp, while months and years only step down the
/// respective fields and normalize later, so that repeated units accumulate and a day beyond
/// the end of the target month rolls over.
fn apply_operations(now: Option<Zoned>, operations: &[Operation<'_>]) -> Result<Zoned, Exn<Error>> {
    /// The calendar and clock fields for subtraction.
    struct Fields {
        year: i16,
        month: i8,
        // Negative values are Git's unspecified day (-1) and pending previous day (-2).
        day: i8,
        time: civil::Time,
        // Retain the last normalized instant, including its choice of offset at ambiguous local
        // times. Like Git's tm_wday, its weekday stays unchanged when month/year fields change.
        zoned: Zoned,
    }

    impl From<Zoned> for Fields {
        fn from(zdt: Zoned) -> Self {
            Fields {
                year: zdt.year(),
                month: zdt.month(),
                day: zdt.day(),
                time: zdt.time(),
                zoned: zdt,
            }
        }
    }

    impl Fields {
        /// Turn the fields back into a point in time: a day beyond the end of the month rolls over into the
        /// following month. One month before May 31st is thus May 1st, a day after April 30th.
        fn normalize(&self, day: i8) -> Result<Zoned, Exn<Error>> {
            if self.year == self.zoned.year()
                && self.month == self.zoned.month()
                && day == self.zoned.day()
                && self.time == self.zoned.time()
            {
                return Ok(self.zoned.clone());
            }
            let first_of_month = civil::Date::new(self.year, self.month, 1)
                .or_raise(|| Error::new(format!("Date lies out of range: {}-{:02}", self.year, self.month)))?;
            let days_beyond_first = SignedDuration::from_secs((i64::from(day) - 1) * 24 * 60 * 60);
            // TODO: Match Git's retained tm_isdst when changed calendar fields cross DST boundaries.
            first_of_month
                .checked_add(days_beyond_first)
                .or_raise(|| Error::new(format!("Day {day} lies out of range")))?
                .to_datetime(self.time)
                .to_zoned(self.zoned.time_zone().clone())
                .or_raise(|| Error::new("Could not convert date to a point in time"))
        }

        /// Git defers a named clock's previous-day hint until normalization. A nonzero
        /// duration overrides that hint, so `noon 1 day ago` only goes back one day.
        fn update(&self, reference_day: i8, mut seconds: i64) -> Result<Zoned, Exn<Error>> {
            let day = if self.day < 0 {
                if seconds == 0 && self.day < -1 {
                    seconds = 24 * 60 * 60;
                }
                reference_day
            } else {
                self.day
            };
            let normalized = self.normalize(day)?;
            let timestamp = normalized
                .timestamp()
                .checked_sub(SignedDuration::from_secs(seconds))
                .or_raise(|| Error::new("Relative date is out of range"))?;
            Ok(timestamp.to_zoned(normalized.time_zone().clone()))
        }
    }

    let now = now.ok_or(ValidationError::new("Missing current time"))?;
    let reference_day = now.day();
    let mut fields = Fields::from(now);
    fields.day = -1;
    for operation in operations {
        let Pair { period, count, unit } = match operation {
            Operation::NamedTime(hour) => {
                if fields.day < 0 && fields.time.hour() < *hour {
                    fields.day = -2;
                }
                fields.time = civil::Time::new(*hour, 0, 0, 0).expect("named hours are valid");
                continue;
            }
            Operation::Yesterday => {
                fields.day = -1;
                fields = fields.update(reference_day, 24 * 60 * 60)?.into();
                continue;
            }
            Operation::Now => {
                fields = fields.update(reference_day, 0)?.into();
                continue;
            }
            Operation::Pair(pair) => pair,
        };
        // Git's zero pending count never applies a unit or normalizes calendar fields.
        if *count == 0 {
            continue;
        }
        let err = || Error::new(format!("Couldn't parse span from '{period} {count}'"));
        let seconds = match unit {
            Unit::Seconds(factor) => count.checked_mul(*factor).ok_or_else(err)?,
            Unit::Weekday(weekday) => {
                let days = (i64::from(fields.zoned.weekday().to_sunday_zero_offset()) - i64::from(*weekday) - 1)
                    .rem_euclid(7)
                    + 1;
                count
                    .checked_sub(1)
                    .and_then(|weeks| weeks.checked_mul(7))
                    .and_then(|weeks| weeks.checked_add(days))
                    .and_then(|days| days.checked_mul(24 * 60 * 60))
                    .ok_or_else(err)?
            }
            Unit::Months(factor) => {
                let months = count.checked_mul(*factor).ok_or_else(err)?;
                fields = fields.update(reference_day, 0)?.into();
                let total = (i64::from(fields.year) * 12 + i64::from(fields.month) - 1)
                    .checked_sub(months)
                    .ok_or_else(err)?;
                fields.year = i16::try_from(total.div_euclid(12)).ok().ok_or_else(err)?;
                fields.month = i8::try_from(total.rem_euclid(12) + 1).expect("a value in 1..=12");
                continue;
            }
        };
        fields = fields.update(reference_day, seconds).or_raise(err)?.into();
    }
    fields.update(reference_day, 0)
}

fn subtract_duration(now: Option<&Zoned>, duration: SignedDuration) -> Result<Zoned, Exn<ValidationError>> {
    let now = now.ok_or(ValidationError::new("Missing current time"))?;
    now.timestamp()
        .checked_sub(duration)
        .map(|timestamp| timestamp.to_zoned(now.time_zone().clone()))
        .or_raise(|| Error::new(format!("Failed to subtract {duration} from {now}")))
}
