use gix_error::{OptionExt, Result, ResultExt, bail, message, validation};
use jiff::{SignedDuration, Zoned, civil};

pub fn parse(input: &str, now: Option<Zoned>) -> Option<Result<Zoned>> {
    let mut ending = input
        .rsplit(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty());
    if ending.next().is_some_and(|word| word.eq_ignore_ascii_case("now"))
        && ending.next().is_some_and(|word| word.eq_ignore_ascii_case("from"))
    {
        return None;
    }
    // First try named dates
    if let Some(result) = parse_named(input, now.as_ref()) {
        return Some(result);
    }

    // A rejected raw date must not become unrelated calendar fields merely because
    // a reference time is available. Valid raw dates have already been handled.
    let mut raw = input.split_whitespace();
    if raw
        .next()
        .is_some_and(|word| word.trim_start_matches('@').parse::<i64>().is_ok())
        && raw.next().is_some_and(|word| {
            word.strip_prefix(['+', '-'])
                .is_some_and(|digits| digits.as_bytes().first().is_some_and(u8::is_ascii_digit))
        })
        && raw.next().is_none()
    {
        return None;
    }
    // Numeric dates are a unit, not three independently guessed calendar fields.
    if input.split_whitespace().any(|word| {
        let end = word
            .bytes()
            .take_while(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'/' | b'-'))
            .count();
        let word = &word[..end];
        let mut fields = word.split(['.', '/', '-']);
        (0..3).all(|_| {
            fields
                .next()
                .is_some_and(|field| !field.is_empty() && field.bytes().all(|b| b.is_ascii_digit()))
        })
    }) {
        return None;
    }
    Some(apply_operations(now, &parse_operations(input)?))
}

/// Fast path for standalone epoch and fixed-duration names.
fn parse_named(input: &str, now: Option<&Zoned>) -> Option<Result<Zoned>> {
    let input = input.trim();
    if input.eq_ignore_ascii_case("never") {
        return Some(Ok(
            jiff::Timestamp::UNIX_EPOCH.to_zoned(now.map_or(jiff::tz::TimeZone::UTC, |now| now.time_zone().clone()))
        ));
    }
    let duration = if input.eq_ignore_ascii_case("now") {
        SignedDuration::ZERO
    } else if input.eq_ignore_ascii_case("yesterday") {
        SignedDuration::from_hours(24)
    } else {
        return None;
    };

    Some(subtract_duration(now, duration))
}

/// Keep clock punctuation while separating counts from words, including adjacent pairs.
fn tokens(mut input: &str) -> impl Iterator<Item = &str> {
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
    let mut operations = Vec::new();
    let mut touched = false;
    let mut tokens = tokens(input).peekable();
    while let Some(word) = tokens.next() {
        let operation = if word.contains(':') {
            let (clock, suffix) = word
                .split_once('.')
                .map_or((word, None), |(clock, suffix)| (clock, Some(suffix)));
            let clock = Clock::parse(clock)?;
            if tokens.peek().is_some_and(|word| meridian(word).is_some()) && !(1..=12).contains(&clock.hour) {
                return None;
            }
            Operation::Time {
                clock,
                suffix: suffix.filter(|_| tokens.peek().is_some_and(|word| unit(word).is_some())),
            }
        } else if word.as_bytes()[0].is_ascii_digit() {
            let count = word.parse().ok()?;
            Operation::Number(count)
        } else if let Some(month) = super::git::month_name(word) {
            Operation::Month(month)
        } else if word.eq_ignore_ascii_case("noon") {
            Operation::NamedTime(12)
        } else if word.eq_ignore_ascii_case("midnight") {
            Operation::NamedTime(0)
        } else if word.eq_ignore_ascii_case("tea") {
            Operation::NamedTime(17)
        } else if word.eq_ignore_ascii_case("yesterday") {
            Operation::Yesterday
        } else if word.eq_ignore_ascii_case("now") {
            Operation::Now
        } else if word.eq_ignore_ascii_case("today") {
            Operation::Today
        } else if word.eq_ignore_ascii_case("never") {
            Operation::Never
        } else if let Some(is_pm) = meridian(word) {
            Operation::Meridian { is_pm }
        } else if let Some(count) = count(word) {
            Operation::CountWord(count)
        } else if let Some((period, unit)) = unit(word) {
            Operation::Unit { period, unit }
        } else {
            continue;
        };
        touched |= !matches!(operation, Operation::Unit { .. });
        operations.push(operation);
    }
    touched.then_some(operations)
}

pub(super) fn meridian(input: &str) -> Option<bool> {
    if input.eq_ignore_ascii_case("am") {
        Some(false)
    } else if input.eq_ignore_ascii_case("pm") {
        Some(true)
    } else {
        None
    }
}

/// A count spelled with one of zero-ten or `last`.
fn count(input: &str) -> Option<i64> {
    const NAMES: &[&str] = &[
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    ];
    NAMES
        .iter()
        .position(|name| input.eq_ignore_ascii_case(name))
        .map(|pos| pos as i64)
        .or_else(|| input.eq_ignore_ascii_case("last").then_some(1))
}

enum Operation<'a> {
    Number(i64),
    CountWord(i64),
    Month(i8),
    Unit { period: &'a str, unit: Unit },
    Time { clock: Clock, suffix: Option<&'a str> },
    NamedTime(i8),
    Meridian { is_pm: bool },
    Yesterday,
    Now,
    Today,
    Never,
}

/// Allow ISO's end-of-day 24:00, while rejecting other overflowing clocks.
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
        (fields.next().is_none()
            && (0..=24).contains(&hour)
            && (0..60).contains(&minute)
            && (0..60).contains(&second)
            && (hour < 24 || (minute == 0 && second == 0)))
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
/// seconds, while `month` and `year` subtract calendar units with a constrained day.
enum Unit {
    /// One unit is this many seconds, to be taken off the timestamp.
    Seconds(i64),
    /// One unit is this many months, to be taken off the year and month fields.
    Months(i64),
    /// The nth previous occurrence of this weekday, numbered from Sunday as zero.
    Weekday(i8),
}

/// Classify `period`, also returning duration units in singular form.
fn unit(period: &str) -> Option<(&str, Unit)> {
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
    } else {
        return None;
    };
    Some((period, unit))
}

/// Apply `operations` to `now` in input order, like Git's `approxidate()`.
///
/// Seconds-based units subtract from the timestamp, while months and years only step down the
/// respective fields, constraining the day to the target month before the next operation.
fn apply_operations(now: Option<Zoned>, operations: &[Operation<'_>]) -> Result<Zoned> {
    /// The calendar and clock fields for subtraction.
    struct Fields {
        year: Option<i16>,
        month: Option<i8>,
        // Negative values are Git's unspecified day (-1) and pending previous day (-2).
        day: i8,
        clock: Clock,
        // Retain the last normalized instant, including its offset at ambiguous local times.
        zoned: Zoned,
    }

    impl From<Zoned> for Fields {
        fn from(zdt: Zoned) -> Self {
            Fields {
                year: Some(zdt.year()),
                month: Some(zdt.month()),
                day: zdt.day(),
                clock: zdt.time().into(),
                zoned: zdt,
            }
        }
    }

    impl Fields {
        /// Git flushes a pending number into the first calendar field it can fill.
        fn pending_number(&mut self, pending: &mut Option<i64>) -> Result<()> {
            let Some(number) = pending.take() else {
                return Ok(());
            };
            if self.day < 0 && (1..32).contains(&number) {
                self.day = number as i8;
            } else if self.month.is_none() && (1..13).contains(&number) {
                self.month = Some(number as i8);
            } else if self.year.is_none() {
                self.year = match number {
                    100..=9999 => Some(number as i16),
                    70..=99 => Some(number as i16 + 1900),
                    1..=69 => Some(number as i16 + 2000),
                    0 if self.day > 0 && self.month.is_some() => Some(2000),
                    _ => bail!(validation("Number is not a supported calendar year")),
                };
            } else {
                bail!(validation("Unused number after complete calendar fields"));
            }
            Ok(())
        }

        /// Validate explicit dates and resolve local clocks using the target timezone rules.
        fn normalize(&self, day: i8) -> Result<Zoned> {
            let month = self.month.unwrap_or(self.zoned.month());
            let year = self
                .year
                .unwrap_or_else(|| self.zoned.year() - i16::from(month > self.zoned.month()));
            if year == self.zoned.year()
                && month == self.zoned.month()
                && day == self.zoned.day()
                && self.clock == self.zoned.time().into()
            {
                return Ok(self.zoned.clone());
            }
            let first_of_month = civil::Date::new(year, month, 1)
                .or_raise(|| message!("Date lies out of range: {year}-{month:02}").validation())?;
            let day = if self.day < 0 {
                day.min(first_of_month.days_in_month())
            } else {
                day
            };
            let datetime = civil::Date::new(year, month, day)
                .or_raise(|| message!("Invalid calendar date: {year}-{month:02}-{day:02}").validation())?
                .at(0, 0, 0, 0)
                .checked_add(self.clock.duration())
                .or_raise(|| gix_error::validation("Clock rollover lies out of range"))?;
            datetime
                .to_zoned(self.zoned.time_zone().clone())
                .or_raise(|| gix_error::validation("Could not convert date to a point in time"))
        }

        /// Git defers a named clock's previous-day hint until normalization. A nonzero
        /// duration overrides that hint, so `noon 1 day ago` only goes back one day.
        fn update(&self, seconds: i64) -> Result<Zoned> {
            let day = if self.day < 0 { self.zoned.day() } else { self.day };
            let mut normalized = self.normalize(day)?;
            if seconds == 0 && self.day < -1 {
                normalized = normalized
                    .datetime()
                    .checked_sub(SignedDuration::from_hours(24))
                    .or_raise(|| validation("Previous local day is out of range"))?
                    .to_zoned(normalized.time_zone().clone())
                    .or_raise(|| validation("Previous local clock is out of range"))?;
            }
            let timestamp = normalized
                .timestamp()
                .checked_sub(SignedDuration::from_secs(seconds))
                .or_raise(|| gix_error::validation("Relative date is out of range"))?;
            Ok(timestamp.to_zoned(normalized.time_zone().clone()))
        }
    }

    let now = now.ok_or_raise(|| validation("Missing current time"))?;
    let mut fields = Fields::from(now);
    fields.year = None;
    fields.month = None;
    fields.day = -1;
    let mut pending = None;
    let mut explicit_clock = false;
    for operation in operations {
        let (period, count, unit) = match operation {
            Operation::Number(count) => {
                fields.pending_number(&mut pending)?;
                pending = Some(*count);
                continue;
            }
            Operation::Month(month) => {
                // A pending day can precede or follow the month name.
                fields.month = Some(*month);
                fields.day = -1;
                continue;
            }
            Operation::CountWord(count) => {
                pending.get_or_insert(*count);
                continue;
            }
            Operation::Time { clock, suffix } => {
                fields.pending_number(&mut pending)?;
                fields.clock = *clock;
                explicit_clock = true;
                pending = suffix.and_then(|suffix| suffix.parse().ok());
                continue;
            }
            Operation::NamedTime(hour) => {
                explicit_clock = true;
                fields.pending_number(&mut pending)?;
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
            Operation::Meridian { is_pm } => {
                if let Some(hour) = pending.take() {
                    if !(1..=12).contains(&hour) {
                        bail!(validation("AM/PM hours must lie in 1..=12"));
                    }
                    fields.clock = Clock {
                        hour: (hour % 12) as i8,
                        minute: 0,
                        second: 0,
                        nanosecond: 0,
                    };
                }
                explicit_clock = true;
                fields.clock.hour = fields.clock.hour % 12 + if *is_pm { 12 } else { 0 };
                continue;
            }
            Operation::Yesterday => {
                pending = None;
                fields = fields.update(24 * 60 * 60)?.into();
                continue;
            }
            Operation::Now => {
                pending = None;
                fields = fields.update(0)?.into();
                continue;
            }
            Operation::Today => {
                pending = None;
                if fields.day == -2 {
                    fields.day = -1;
                }
                if !explicit_clock {
                    fields.clock = Clock {
                        hour: 0,
                        minute: 0,
                        second: 0,
                        nanosecond: 0,
                    };
                }
                fields = fields.update(0)?.into();
                continue;
            }
            Operation::Never => {
                pending = None;
                explicit_clock = false;
                fields = jiff::Timestamp::UNIX_EPOCH
                    .to_zoned(fields.zoned.time_zone().clone())
                    .into();
                continue;
            }
            Operation::Unit { period, unit } => (period, pending.take().unwrap_or(0), unit),
        };
        // Zero is an identity operation, and a bare unit has no count to apply.
        if count == 0 {
            continue;
        }
        // A specified duration or weekday overrides a named clock's previous-day hint.
        if fields.day == -2 {
            fields.day = -1;
        }
        let err = || message!("Couldn't parse span from '{period} {count}'").validation();
        let seconds = match unit {
            Unit::Seconds(factor) => count.checked_mul(*factor).ok_or_raise(err)?,
            Unit::Weekday(weekday) => {
                fields = fields.update(0)?.into();
                let days = (i64::from(fields.zoned.weekday().to_sunday_zero_offset()) - i64::from(*weekday) - 1)
                    .rem_euclid(7)
                    + 1;
                let seconds = count
                    .checked_sub(1)
                    .and_then(|weeks| weeks.checked_mul(7))
                    .and_then(|weeks| weeks.checked_add(days))
                    .and_then(|days| days.checked_mul(24 * 60 * 60))
                    .ok_or_raise(err)?;
                fields = fields
                    .zoned
                    .datetime()
                    .checked_sub(SignedDuration::from_secs(seconds))
                    .or_raise(err)?
                    .to_zoned(fields.zoned.time_zone().clone())
                    .or_raise(err)?
                    .into();
                continue;
            }
            Unit::Months(factor) => {
                let months = count.checked_mul(*factor).ok_or_raise(err)?;
                fields = fields.update(0)?.into();
                let span = jiff::Span::new().try_months(months).or_raise(err)?;
                fields = fields
                    .zoned
                    .datetime()
                    .checked_sub(span)
                    .or_raise(err)?
                    .to_zoned(fields.zoned.time_zone().clone())
                    .or_raise(err)?
                    .into();
                continue;
            }
        };
        fields = fields.update(seconds).or_raise(err)?.into();
    }
    fields.pending_number(&mut pending)?;
    fields.update(0)
}

fn subtract_duration(now: Option<&Zoned>, duration: SignedDuration) -> Result<Zoned> {
    let now = now.ok_or_raise(|| validation("Missing current time"))?;
    now.timestamp()
        .checked_sub(duration)
        .map(|timestamp| timestamp.to_zoned(now.time_zone().clone()))
        .or_raise(|| message!("Failed to subtract {duration} from {now}").validation())
}
