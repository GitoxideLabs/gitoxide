use crate::{SecondsSinceUnixEpoch, Time};

/// Strictly parse the raw commit header format like `1745582210 +0200`.
///
/// Some strict rules include:
///
/// - The timezone offset must be present.
/// - The timezone offset must have a sign; either `+` or `-`.
/// - The timezone offset hours must be less than 24.
/// - The timezone offset minutes must be less than 60.
/// - The timezone offset seconds may be present, but 0 is the only valid value.
/// - Only whitespace may suffix the timezone offset.
///
/// But this function isn't perfectly strict insofar as it allows arbitrary
/// whitespace before and after the seconds and offset components.
///
/// The goal is to only accept inputs that _unambiguously_ look like
/// git's raw date format.
pub fn parse_raw(input: &str) -> Option<Time> {
    let mut split = input.split_whitespace();
    let seconds = split.next()?.parse::<SecondsSinceUnixEpoch>().ok()?;
    let offset_str = split.next()?;
    if split.next().is_some() {
        return None;
    }
    let offset_len = offset_str.len();
    if (offset_len != 5 && offset_len != 7) || !offset_str.as_bytes()[1..].iter().all(u8::is_ascii_digit) {
        return None;
    }
    let sign: i32 = match offset_str.get(..1)? {
        "-" => Some(-1),
        "+" => Some(1),
        _ => None,
    }?;
    let hours: u8 = offset_str.get(1..3)?.parse().ok()?;
    let minutes: u8 = offset_str.get(3..5)?.parse().ok()?;
    let offset_seconds: u8 = if offset_len == 7 {
        offset_str.get(5..7)?.parse().ok()?
    } else {
        0
    };
    if hours > 23 || minutes > 59 || offset_seconds != 0 {
        return None;
    }
    let offset: i32 = sign * ((hours as i32) * 3600 + (minutes as i32) * 60);
    Time { seconds, offset }.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_raw_valid() {
        // These examples show how it's more loose than it has to be,
        // merely as a side effect of the implementation.
        for (valid, expected_seconds, expected_offset) in [
            ("12345 +0000", 12345, 0),
            ("-1234567 +0000", -1234567, 0),
            ("+1234567 -000000", 1234567, 0),
            ("   +0    -000000    ", 0, 0),
            ("\t-0\t-0000\t", 0, 0),
            ("\n-0\r\n-0000\n", 0, 0),
        ] {
            assert_eq!(
                parse_raw(valid),
                Some(Time {
                    seconds: expected_seconds,
                    offset: expected_offset
                }),
                "should succeed: '{valid}'"
            );
        }
    }

    #[test]
    fn parse_raw_invalid() {
        for (bad_date_str, message) in [
            ("123456 !0600", "invalid sign - must be + or -"),
            ("123456 0600", "missing offset sign"),
            ("123456 +060", "positive offset too short"),
            ("123456 -060", "negative offset too short"),
            ("123456 +06000", "not enough offset seconds"),
            ("123456 --060", "duplicate offset sign with correct offset length"),
            ("123456 -+060", "multiple offset signs with correct offset length"),
            ("123456 --0600", "multiple offset signs, but incorrect offset length"),
            ("123456 +-06000", "multiple offset signs with correct offset length"),
            ("123456 +-0600", "multiple offset signs with incorrect offset length"),
            ("123456 +-060", "multiple offset signs with correct offset length"),
            ("123456 +10030", "invalid offset length with one 'second' field"),
            ("123456 06000", "invalid offset length, missing sign"),
            ("123456 +0600 extra", "extra field past offset"),
            ("123456 +0600 2005", "extra field past offset that looks like year"),
            ("123456+0600", "missing space between unix timestamp and offset"),
            (
                "123456 + 600",
                "extra spaces between sign and offset (which also is too short)",
            ),
            ("123456 -2400", "negative offset hours out of bounds"),
            ("123456 +2400", "positive offset hours out of bounds"),
            ("123456 +6600", "positive offset hours out of bounds"),
            ("123456 +0660", "invalid offset minutes"),
            ("123456 +060010", "positive offset seconds is allowed but only if zero"),
            ("123456 -060010", "negative offset seconds is allowed but only if zero"),
            ("123456 +0075", "positive offset minutes invalid"),
            ("++123456 +0000", "duplicate timestamp sign"),
            ("--123456 +0000", "duplicate timestamp sign"),
            ("1234567 -+1+1+0", "unsigned offset parsing rejects '+'"),
        ] {
            assert!(
                parse_raw(bad_date_str).is_none(),
                "should fail: '{bad_date_str}': {message}"
            );
        }
    }
}
