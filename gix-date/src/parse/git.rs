use crate::Time;
use jiff::{SignedDuration, Zoned, civil::Date};

/// Resolve Git's timezone abbreviations before RFC 2822 can treat an unfamiliar name as UTC.
pub(super) fn normalize_named_timezone(input: &str) -> Option<String> {
    let (date, name) = input.trim_end().rsplit_once(char::is_whitespace)?;
    // Keep the historical meanings from Git's `date.c::timezone_names`, including daylight time.
    let hours = match name.to_ascii_uppercase().as_str() {
        "IDLW" => -12,
        "NT" => -11,
        "CAT" | "HST" => -10,
        "HDT" | "YST" => -9,
        "YDT" | "PST" => -8,
        "PDT" | "MST" => -7,
        "MDT" | "CST" => -6,
        "CDT" | "EST" => -5,
        "EDT" => -4,
        "AST" => -3,
        "ADT" => -2,
        "WAT" => -1,
        "GMT" | "UTC" | "UT" | "Z" | "WET" => 0,
        "BST" | "CET" | "MET" | "MEWT" | "FWT" => 1,
        "MEST" | "CEST" | "MESZ" | "FST" | "EET" => 2,
        "EEST" => 3,
        "WAST" => 7,
        "WADT" | "CCT" => 8,
        "JST" => 9,
        "EAST" | "GST" => 10,
        "EADT" => 11,
        "NZT" | "NZST" | "IDLE" => 12,
        "NZDT" => 13,
        _ => return None,
    };
    Some(format!("{date} {hours:+03}00"))
}

/// Parse Git-style flexible date formats that aren't covered by standard strptime:
/// - Complete textual dates: `February 14th, 2008 20:30:45 -0500`
/// - Numeric dates with dots or slashes: `2008.02.14`, `14.02.2008`, `02/14/2008`, `02/14/08`
/// - Compact ISO8601: `20080214T203045`, `20080214T20:30:45`, `20080214T2030`, `20080214T20`
/// - Z suffix for UTC: `1970-01-01 00:00:00 Z`
/// - 2-digit hour offset: `2008-02-14 20:30:45 -05`
/// - Colon-separated offset: `2008-02-14 20:30:45 -05:00`
/// - Subsecond precision (ignored): `20080214T203045.019-04:00`
// TODO: this can probably be done more smartly, right now it's more of a brute force. Learn from Git here.
//       After all, this is generated to have something quickly.
pub fn parse_git_date_format(input: &str) -> Option<Time> {
    parse_numeric_date(input)
        .or_else(|| parse_textual_date(input))
        .or_else(|| parse_compact_iso8601(input))
        .or_else(|| parse_flexible_iso8601(input))
}

/// Recognize complete textual dates without guessing missing fields from the current time.
fn parse_textual_date(input: &str) -> Option<Time> {
    if input.contains('\n') {
        return None;
    }
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    const WEEKDAYS: [&str; 7] = [
        "Sundays",
        "Mondays",
        "Tuesdays",
        "Wednesdays",
        "Thursdays",
        "Fridays",
        "Saturdays",
    ];
    let name_index = |word: &str, names: &[&str]| {
        (word.len() >= 3)
            .then(|| {
                names.iter().position(|name| {
                    name.get(..word.len())
                        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(word))
                })
            })
            .flatten()
    };
    let mut words = input
        .split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|word| !word.is_empty());
    let mut first = words.next()?;
    if name_index(first, &WEEKDAYS).is_some() {
        // Like Git, ignore even a weekday that disagrees with the calendar date.
        first = words.next()?;
    }
    let second = words.next()?;
    let (month, day) = if let Some(month) = name_index(first, &MONTHS) {
        (month, second)
    } else {
        (name_index(second, &MONTHS)?, first)
    };
    let digits = day.bytes().take_while(u8::is_ascii_digit).count();
    if !(1..=2).contains(&digits) {
        return None;
    }
    let suffix = &day[digits..];
    if !suffix.is_empty() && !["st", "nd", "rd", "th"].iter().any(|s| suffix.eq_ignore_ascii_case(s)) {
        return None;
    }
    let day: i64 = day[..digits].parse().ok()?;
    let third = words.next()?;
    let fourth = words.next()?;
    let (year, clock) = if third.contains(':') {
        (fourth, third)
    } else {
        (third, fourth)
    };
    if !year.bytes().all(|byte| byte.is_ascii_digit()) || !(1..=31).contains(&day) {
        return None;
    }
    // Unlike grouped numeric dates, match_digit() requires exactly two digits
    // and a known day for 00..09; 10..69 never establish an absolute textual year.
    let year = match (year.len(), year.parse::<i16>().ok()?) {
        (4, year @ 1970..=2099) => year,
        (2, year @ 0..=9) => year + 2000,
        (2, year @ 70..=99) => year + 1900,
        _ => return None,
    };
    // Require a colon clock. Bare numbers follow different guessing rules in Git.
    let mut components = clock.split(':').map(|part| {
        (!part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse::<i64>().ok())
            .flatten()
    });
    let hour = components.next()??;
    let minute = components.next()??;
    let second = components.next().unwrap_or(Some(0))?;
    if components.next().is_some() || hour > 24 || minute > 59 || second > 60 {
        return None;
    }
    let zone = words.next()?;
    let offset_digits = zone.strip_prefix(['+', '-'])?;
    let valid_offset = match offset_digits.len() {
        2 | 4 => offset_digits.bytes().all(|byte| byte.is_ascii_digit()),
        5 => {
            offset_digits.as_bytes()[2] == b':'
                && offset_digits
                    .bytes()
                    .enumerate()
                    .all(|(index, byte)| index == 2 || byte.is_ascii_digit())
        }
        _ => false,
    };
    if !valid_offset {
        return None;
    }
    let offset = parse_flexible_offset(zone)?;
    if words.next().is_some() {
        return None;
    }
    // Git's tm_to_time_t() permits February 31, hour 24, and second 60. Normalize
    // from the first of the month instead of rejecting these as invalid civil dates.
    let datetime = Date::new(year, month as i8 + 1, 1)
        .ok()?
        .at(0, 0, 0, 0)
        .checked_add(SignedDuration::from_secs(
            (day - 1) * 86400 + hour * 3600 + minute * 60 + second,
        ))
        .ok()?;
    let zoned = datetime
        .to_zoned(jiff::tz::Offset::from_seconds(offset).ok()?.to_time_zone())
        .ok()?;
    Some(Time::new(zoned.timestamp().as_second(), offset))
}

/// Normalize numeric dates using Git's separator-dependent month/day preference.
fn parse_numeric_date(input: &str) -> Option<Time> {
    let (date, rest) = input.trim().split_once(char::is_whitespace)?;
    let (first, _) = date.split_once(['.', '/'])?;
    let (_, last) = date.rsplit_once(['.', '/'])?;
    let formats = match (date.contains('/'), first.len() == 4, last.len() == 4) {
        (true, true, _) => Some(["%Y/%m/%d", "%Y/%d/%m"]),
        (false, true, _) => Some(["%Y.%m.%d", "%Y.%d.%m"]),
        (true, false, true) => Some(["%m/%d/%Y", "%d/%m/%Y"]),
        (false, false, true) => Some(["%d.%m.%Y", "%m.%d.%Y"]),
        _ => None,
    };
    // Preserve literal four-digit years, including Jiff's wider range, before expanding short years.
    let date = match formats {
        Some(formats) => formats.iter().find_map(|fmt| Date::strptime(fmt, date).ok())?,
        None => parse_numeric_date_with_short_year(date)?,
    };
    parse_flexible_iso8601(&format!("{date} {rest}"))
}

fn parse_numeric_date_with_short_year(date: &str) -> Option<Date> {
    let separator = if date.contains('/') { b'/' } else { b'.' };
    if !date.bytes().all(|byte| byte.is_ascii_digit() || byte == separator) {
        return None;
    }
    let mut fields = date.split(char::from(separator)).map(str::parse::<i16>);
    let first = fields.next()?.ok()?;
    let second = fields.next()?.ok()?;
    let third = fields.next()?.ok()?;
    if fields.next().is_some() {
        return None;
    }

    let (year, month, day) = if first > 70 {
        (first, second, third)
    } else if separator == b'/' {
        (third, first, second)
    } else {
        (third, second, first)
    };
    // Git's `set_date()` uses these value ranges, not strptime's conventional `%y` pivot.
    let year = match year {
        0..=37 => year + 2000,
        71..=99 => year + 1900,
        _ => return None,
    };
    let candidate = |month: i16, day: i16| Date::new(year, month.try_into().ok()?, day.try_into().ok()?).ok();
    candidate(month, day).or_else(|| candidate(day, month))
}

/// Parse compact ISO8601 formats:
/// - `20080214T203045` (compact time)
/// - `20080214T20:30:45` (normal time)
/// - `20080214T2030` (hours and minutes only)
/// - `20080214T20` (hours only)
/// - With optional subsecond precision (ignored)
/// - With optional timezone
fn parse_compact_iso8601(input: &str) -> Option<Time> {
    let input = input.trim();

    // Must have T separator and start with 8 digits for YYYYMMDD
    let t_pos = input.find('T')?;
    if t_pos != 8 {
        return None;
    }

    let date_part = &input.get(..8)?;
    // Verify date part is ASCII (valid date chars are all ASCII)
    if !date_part.is_ascii() {
        return None;
    }
    // Parse YYYYMMDD
    let year: i32 = date_part[0..4].parse().ok()?;
    let month: i32 = date_part[4..6].parse().ok()?;
    let day: i32 = date_part[6..8].parse().ok()?;

    // Parse time part - may have colons or not, may have subseconds, may have timezone
    let rest = &input.get(9..)?; // after T
    let (time_str, offset_str) = split_time_and_offset(rest);

    // Strip subseconds (anything after a dot in the time part, before offset)
    let time_str = if let Some(dot_pos) = time_str.find('.') {
        &time_str[..dot_pos]
    } else {
        time_str
    };

    // Parse time - could be HH:MM:SS, HHMMSS, HH:MM, HHMM, or HH
    let (hour, minute, second) = parse_time_component(time_str)?;

    // Parse offset
    let offset = parse_flexible_offset(offset_str)?;

    // Construct the datetime
    let zoned = new_zoned(year, month, day, hour, minute, second, offset)?;
    Time::new(zoned.timestamp().as_second(), offset).into()
}

/// Parse ISO8601 with flexible timezone (Z suffix, 2-digit offset, colon-separated offset)
/// and optional subsecond precision
fn parse_flexible_iso8601(input: &str) -> Option<Time> {
    let input = input.trim();

    // Check if this looks like ISO8601 (YYYY-MM-DD format)
    let date_part = &input.get(..10)?;
    // Verify date part is ASCII (valid date chars are all ASCII)
    if !date_part.is_ascii() {
        return None;
    }
    if date_part.chars().nth(4)? != '-' || date_part.chars().nth(7)? != '-' {
        return None;
    }

    // Parse the date
    let year: i32 = date_part[0..4].parse().ok()?;
    let month: i32 = date_part[5..7].parse().ok()?;
    let day: i32 = date_part[8..10].parse().ok()?;

    // Rest after date
    let rest = &input.get(10..)?;
    if rest.is_empty() {
        return None;
    }

    // Skip T or space separator
    let rest = if rest.starts_with('T') || rest.starts_with(' ') {
        &rest[1..]
    } else {
        return None;
    };

    // Split into time and offset
    let (time_str, offset_str) = split_time_and_offset(rest);

    // Strip subseconds
    let time_str = if let Some(dot_pos) = time_str.find('.') {
        &time_str[..dot_pos]
    } else {
        time_str
    };

    // Parse time HH:MM:SS
    let (hour, minute, second) = parse_time_component(time_str)?;

    // Parse offset
    let offset = parse_flexible_offset(offset_str)?;

    // Construct the datetime
    let zoned = new_zoned(year, month, day, hour, minute, second, offset)?;
    Some(Time::new(zoned.timestamp().as_second(), offset))
}

fn new_zoned(year: i32, month: i32, day: i32, hour: u32, minute: u32, second: u32, offset: i32) -> Option<Zoned> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let date = jiff::civil::Date::new(year as i16, month as i8, day as i8).ok()?;
    let datetime = date.at(hour as i8, minute as i8, second as i8, 0);
    let tz_offset = jiff::tz::Offset::from_seconds(offset).ok()?;
    let zoned = datetime.to_zoned(tz_offset.to_time_zone()).ok()?;
    zoned.into()
}

/// Split time string into time component and offset component
fn split_time_and_offset(input: &str) -> (&str, &str) {
    // Look for offset indicators: Z, +, - (but - after digits could be in time)
    // The offset is at the end, after the time
    let input = input.trim();

    // Check for Z suffix
    if let Some(stripped) = input.strip_suffix('Z') {
        return (stripped, "Z");
    }

    // Look for + or - that indicates timezone (not part of time)
    // The time never contains a sign, so an offset starts no earlier than after `HH`.
    let mut offset_start = None;
    for (i, c) in input.char_indices().rev() {
        if (c == '+' || c == '-') && i >= 2 {
            // Check if this looks like an offset (followed by digits)
            let after = &input[i + 1..];
            if after.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                offset_start = Some(i);
                break;
            }
        }
    }

    // Also handle space-separated offset
    if let Some(space_pos) = input.rfind(' ')
        && space_pos > 5
    {
        let potential_offset = input[space_pos + 1..].trim();
        if potential_offset.starts_with('+') || potential_offset.starts_with('-') || potential_offset == "Z" {
            return (&input[..space_pos], potential_offset);
        }
    }

    if let Some(pos) = offset_start {
        (&input[..pos], &input[pos..])
    } else {
        (input, "")
    }
}

/// Parse time component: HH:MM:SS, HHMMSS, HH:MM, HHMM, or HH
fn parse_time_component(time: &str) -> Option<(u32, u32, u32)> {
    let time = time.trim();

    // Time components must be ASCII
    if !time.is_ascii() {
        return None;
    }

    let (hour, minute, second) = if time.contains(':') {
        // Colon-separated: HH:MM:SS or HH:MM
        let parts: Vec<&str> = time.split(':').collect();
        let hour: u32 = parts.first()?.parse().ok()?;
        let minute: u32 = parts.get(1).unwrap_or(&"0").parse().ok()?;
        let second: u32 = parts.get(2).unwrap_or(&"0").parse().ok()?;
        Some((hour, minute, second))
    } else {
        // Compact: HHMMSS, HHMM, or HH
        match time.len() {
            2 => {
                let hour: u32 = time.parse().ok()?;
                Some((hour, 0, 0))
            }
            4 => {
                let hour: u32 = time[0..2].parse().ok()?;
                let minute: u32 = time[2..4].parse().ok()?;
                Some((hour, minute, 0))
            }
            6 => {
                let hour: u32 = time[0..2].parse().ok()?;
                let minute: u32 = time[2..4].parse().ok()?;
                let second: u32 = time[4..6].parse().ok()?;
                Some((hour, minute, second))
            }
            _ => None,
        }
    }?;
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    (hour, minute, second).into()
}

/// Parse flexible timezone offset:
/// - Empty or missing: +0000
/// - Z: +0000
/// - +/-HH: +/-HH00
/// - +/-HHMM: +/-HHMM
/// - +/-HH:MM: +/-HHMM
fn parse_flexible_offset(offset: &str) -> Option<i32> {
    let offset = offset.trim();

    if offset.is_empty() {
        return Some(0);
    }

    // Offset must be ASCII
    if !offset.is_ascii() {
        return None;
    }

    if offset == "Z" {
        return Some(0);
    }

    let (sign, rest) = if let Some(stripped) = offset.strip_prefix('+') {
        (1, stripped)
    } else {
        let stripped = offset.strip_prefix('-')?;
        (-1, stripped)
    };

    // Remove colon if present
    let rest = rest.replace(':', "");
    let (hours, minutes) = match rest.len() {
        2 => {
            // HH format
            let hours: i32 = rest.parse().ok()?;
            (hours, 0)
        }
        4 => {
            // HHMM format
            let hours: i32 = rest[0..2].parse().ok()?;
            let minutes: i32 = rest[2..4].parse().ok()?;
            (hours, minutes)
        }
        _ => return None,
    };

    if hours > 23 || minutes > 59 {
        return None;
    }

    Some(sign * (hours * 3600 + minutes * 60))
}
