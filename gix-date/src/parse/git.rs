use crate::Time;
use jiff::Zoned;

/// Parse Git-style flexible date formats that aren't covered by standard strptime:
/// - ISO8601 with dots: `2008.02.14 20:30:45 -0500`
/// - Compact ISO8601: `20080214T203045`, `20080214T20:30:45`, `20080214T2030`, `20080214T20`
/// - Z suffix for UTC: `1970-01-01 00:00:00 Z`
/// - 2-digit hour offset: `2008-02-14 20:30:45 -05`
/// - Colon-separated offset: `2008-02-14 20:30:45 -05:00`
/// - Subsecond precision (ignored): `20080214T203045.019-04:00`
// TODO: this can probably be done more smartly, right now it's more of a brute force. Learn from Git here.
//       After all, this is generated to have something quickly.
pub fn parse_git_date_format(input: &str) -> Option<Time> {
    parse_iso8601_dots(input)
        .or_else(|| parse_compact_iso8601(input))
        .or_else(|| parse_flexible_iso8601(input))
}

/// Parse ISO8601 with dots: `2008.02.14 20:30:45 -0500`
fn parse_iso8601_dots(input: &str) -> Option<Time> {
    // Format: YYYY.MM.DD HH:MM:SS offset
    let input = input.trim();
    let first_10 = input.get(..10)?;
    if !first_10.is_ascii() || !first_10.contains('.') {
        return None;
    }

    // Replace dots with dashes for date part only
    let (date_part, rest) = input.split_once(' ')?;

    // Validate date part has dot separators
    if date_part.len() != 10 || date_part.chars().nth(4)? != '.' || date_part.chars().nth(7)? != '.' {
        return None;
    }

    // Convert to standard ISO8601 format
    let normalized = format!("{} {}", date_part.replace('.', "-"), rest);
    parse_flexible_iso8601(&normalized)
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
    // Time format is HH:MM:SS or HHMMSS, so offset starts after that
    // Find the last + or - that's after position 5 (minimum for HH:MM)
    let mut offset_start = None;
    for (i, c) in input.char_indices().rev() {
        if (c == '+' || c == '-') && i >= 5 {
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
