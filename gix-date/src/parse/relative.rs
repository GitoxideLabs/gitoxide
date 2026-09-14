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

/// Keep clock punctuation while separating counts from words, including adjacent pairs.
fn tokens(mut input: &str) -> impl Iterator<Item = &str> + Clone {
    std::iter::from_fn(move || {
        let start = input.as_bytes().iter().position(u8::is_ascii_alphanumeric)?;
        input = &input[start..];
        let bytes = input.as_bytes();
        let mut end = if bytes[0].is_ascii_digit() {
            bytes.iter().take_while(|byte| byte.is_ascii_digit()).count()
        } else {
            bytes.iter().take_while(|byte| byte.is_ascii_alphabetic()).count()
        };
        if bytes[0].is_ascii_digit() {
            let digits_end = end;
            for _ in 0..2 {
                if bytes.get(end) != Some(&b':') || !bytes.get(end + 1).is_some_and(u8::is_ascii_digit) {
                    break;
                }
                end += 1;
                end += bytes[end..].iter().take_while(|byte| byte.is_ascii_digit()).count();
            }
            if end > digits_end && bytes.get(end) == Some(&b'.') && bytes.get(end + 1).is_some_and(u8::is_ascii_digit) {
                end += 1;
                end += bytes[end..].iter().take_while(|byte| byte.is_ascii_digit()).count();
            }
        }
        // Both boundaries are next to ASCII characters, even when separators are non-ASCII.
        let (token, rest) = input.split_at(end);
        input = rest;
        Some(token)
    })
}

/// Parse relative units and clock adjustments in input order.
fn parse_operations(input: &str) -> Option<Vec<Operation<'_>>> {
    let mut words = tokens(input);
    let ago = words.clone().any(|word| word.eq_ignore_ascii_case("ago"));
    let mut operations = Vec::new();
    let mut date_known = false;
    while let Some(mut word) = words.next() {
        if word.contains(':') {
            let (clock, suffix) = word
                .split_once('.')
                .map_or((word, None), |(clock, suffix)| (clock, Some(suffix)));
            operations.push(Operation::Time(Clock::parse(clock)?));
            // Git discards fractional seconds only once all calendar fields are known.
            // Otherwise the dot separates a new count, as in `12:34:56.3.days.ago`.
            let Some(suffix) = suffix.filter(|_| !date_known) else {
                continue;
            };
            word = suffix;
        }
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
        } else if let Some(is_pm) = meridian(word) {
            Operation::Meridian { hour: None, is_pm }
        } else {
            let Some(count) = count(word) else { continue };
            let Some(period) = words.next() else { break };
            if let Some(is_pm) = meridian(period) {
                Operation::Meridian {
                    hour: (count != 0).then_some((count % 12) as i8),
                    is_pm,
                }
            } else {
                let (period, unit) = unit(period, ago)?;
                Operation::Pair(Pair { period, count, unit })
            }
        };
        date_known |= matches!(operation, Operation::Now | Operation::Yesterday)
            || matches!(&operation, Operation::Pair(pair) if pair.count != 0);
        operations.push(operation);
    }
    (!operations.is_empty()).then_some(operations)
}

fn meridian(input: &str) -> Option<bool> {
    if input.eq_ignore_ascii_case("am") {
        Some(false)
    } else if input.eq_ignore_ascii_case("pm") {
        Some(true)
    } else {
        None
    }
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
    Time(Clock),
    NamedTime(i8),
    Meridian { hour: Option<i8>, is_pm: bool },
    Yesterday,
    Now,
}

/// Git permits hour 24 and second 60, deferring their rollover until date normalization.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Clock {
    hour: i8,
    minute: i8,
    second: i8,
    nanosecond: i32,
}

impl From<civil::Time> for Clock {
    fn from(time: civil::Time) -> Self {
        Clock {
            hour: time.hour(),
            minute: time.minute(),
            second: time.second(),
            nanosecond: time.subsec_nanosecond(),
        }
    }
}

impl Clock {
    fn parse(input: &str) -> Option<Self> {
        let mut fields = input.split(':');
        let hour = fields.next()?.parse().ok()?;
        let minute = fields.next()?.parse().ok()?;
        let second = fields.next().map(str::parse).transpose().ok()?.unwrap_or(0);
        (fields.next().is_none() && (0..=24).contains(&hour) && (0..60).contains(&minute) && (0..=60).contains(&second))
            .then_some(Clock {
                hour,
                minute,
                second,
                nanosecond: 0,
            })
    }

    fn duration(self) -> SignedDuration {
        SignedDuration::new(
            i64::from(self.hour) * 3600 + i64::from(self.minute) * 60 + i64::from(self.second),
            self.nanosecond,
        )
    }
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
        clock: Clock,
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
                clock: zdt.time().into(),
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
                && self.clock == self.zoned.time().into()
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
                .at(0, 0, 0, 0)
                .checked_add(self.clock.duration())
                .or_raise(|| Error::new("Clock rollover lies out of range"))?
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
            Operation::Time(clock) => {
                fields.clock = *clock;
                continue;
            }
            Operation::NamedTime(hour) => {
                if fields.day < 0 && fields.clock.hour < *hour {
                    fields.day = -2;
                }
                fields.clock = Clock {
                    hour: *hour,
                    minute: 0,
                    second: 0,
                    nanosecond: 0,
                };
                continue;
            }
            Operation::Meridian { hour, is_pm } => {
                if let Some(hour) = hour {
                    fields.clock = Clock {
                        hour: *hour,
                        minute: 0,
                        second: 0,
                        nanosecond: 0,
                    };
                }
                fields.clock.hour = fields.clock.hour % 12 + if *is_pm { 12 } else { 0 };
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
