use std::time::SystemTime;

use gix_date::Time;
use gix_error::Exn;

#[test]
fn time_without_offset_defaults_to_utc() {
    // Git parses datetime without offset and defaults to UTC (+0000)
    let result = gix_date::parse("1979-02-26 18:30:00", Some(SystemTime::now()));
    assert!(result.is_ok(), "Git parses datetime without offset, defaulting to UTC");
    let time = result.unwrap();
    assert_eq!(time.offset, 0, "Offset should default to UTC (+0000)");
}

#[test]
fn parse_header_is_not_too_lenient() {
    for not_a_header_str in ["2005-04-07T22:13:09", "2005-04-07 22:13:09"] {
        assert!(
            gix_date::parse_header(not_a_header_str).is_none(),
            "parse_header only accepts raw format (timestamp +offset), not ISO8601"
        );
        // Note: gix_date::parse() DOES accept these formats, matching Git's behavior
        // Git parses them with default UTC offset
    }
}

#[test]
fn short() {
    assert_eq!(
        gix_date::parse("1979-02-26", Some(SystemTime::now())).unwrap(),
        Time {
            seconds: 288835200,
            offset: 0,
        },
        "could not parse with SHORT format"
    );
}

#[test]
fn rfc2822() {
    assert_eq!(
        gix_date::parse("Thu, 18 Aug 2022 12:45:06 +0800", None).unwrap(),
        Time {
            seconds: 1660797906,
            offset: 28800,
        },
    );
}

#[test]
fn git_rfc2822() {
    let expected = Time {
        seconds: 1659329106,
        offset: 28800,
    };
    assert_eq!(
        gix_date::parse("Thu, 1 Aug 2022 12:45:06 +0800", None).unwrap(),
        expected,
    );
    assert_eq!(
        gix_date::parse("Thu,  1 Aug 2022 12:45:06 +0800", None).unwrap(),
        expected,
    );
}

#[test]
fn raw() -> Result<(), Exn<gix_date::Error>> {
    assert_eq!(
        gix_date::parse("1660874655 +0800", None)?,
        Time {
            seconds: 1660874655,
            offset: 28800,
        },
    );

    assert_eq!(
        gix_date::parse("1112911993 +0100", None)?,
        Time {
            seconds: 1112911993,
            offset: 3600,
        },
    );

    assert_eq!(
        gix_date::parse("1313584730 +051500", None)?,
        Time {
            seconds: 1313584730,
            offset: 18900,
        },
        "seconds for time-offsets work as well"
    );

    assert_eq!(
        gix_date::parse("1313584730 -0230", None)?,
        Time {
            seconds: 1313584730,
            offset: -150 * 60,
        },
    );

    assert!(gix_date::parse("1313584730 +1500", None).is_err());
    assert!(gix_date::parse("1313584730 +000001", None).is_err());
    assert!(gix_date::parse("1313584730 +0001", None).is_err());
    assert!(gix_date::parse("1313584730 +000100", None).is_err());

    let expected = Time {
        seconds: 1660874655,
        offset: -28800,
    };
    for date_str in [
        "1660874655 -0800",
        "1660874655 -0800  ",
        "  1660874655 -0800",
        "  1660874655 -0800  ",
        "  1660874655  -0800  ",
        "1660874655\t-0800",
        "1660874655\t-080000",
    ] {
        assert_eq!(gix_date::parse_header(date_str), Some(expected));
    }
    Ok(())
}

#[test]
fn bad_raw() {
    for bad_date_str in [
        "123456 !0600",
        "123456 +060",
        "123456 -060",
        "123456 +06000",
        "123456 +10030",
        "123456 06000",
        "123456  0600",
        "123456 +0600 extra",
        "123456+0600",
        "123456 + 600",
    ] {
        assert_eq!(
            gix_date::parse_header(bad_date_str),
            Some(Time {
                seconds: 123456,
                offset: 0
            }),
            "{bad_date_str}: invalid offsets default to zero (like in git2), and Git ignores them mostly"
        );
    }
}

#[test]
fn double_negation_in_offset() {
    let actual = gix_date::parse_header("1288373970 --700").unwrap();
    assert_eq!(
        actual,
        gix_date::Time {
            seconds: 1288373970,
            offset: 0,
        },
        "double-negation isn't special, it's considered malformed"
    );

    assert_eq!(
        actual.to_string(),
        "1288373970 +0000",
        "serialization is lossy as offset couldn't be parsed"
    );
}

#[test]
fn git_default() {
    assert_eq!(
        gix_date::parse("Thu Aug 8 12:45:06 2022 +0800", None).unwrap(),
        Time {
            seconds: 1659933906,
            offset: 28800,
        },
    );
}

#[test]
fn invalid_dates_can_be_produced_without_current_time() {
    assert_eq!(
        gix_date::parse("foobar", None).unwrap_err().to_string(),
        "Unknown date format: foobar"
    );
}

/// Tests for compact ISO8601 formats (YYYYMMDDTHHMMSS variants)
mod compact_iso8601;
mod relative;

/// Tests for ISO8601 with dots format (YYYY.MM.DD HH:MM:SS offset)
mod iso8601_dots {
    use gix_date::Time;

    #[test]
    fn basic() {
        assert_eq!(
            gix_date::parse("2008.02.14 20:30:45 -0500", None).unwrap(),
            Time {
                seconds: 1203039045,
                offset: -18000,
            },
            "2008.02.14 20:30:45 -0500"
        );
    }
}

/// Tests for flexible timezone offset formats
mod flexible_offset;

/// Tests for subsecond precision in ISO8601 formats (ignored like Git)
mod subsecond_precision {
    use gix_date::Time;

    #[test]
    fn iso8601_with_subseconds() {
        assert_eq!(
            gix_date::parse("2008-02-14 20:30:45.019-04:00", None).unwrap(),
            Time {
                seconds: 1203035445,
                offset: -14400,
            },
            "2008-02-14 20:30:45.019-04:00 - subseconds ignored"
        );
    }
}

/// Various cases the fuzzer found
mod fuzz {
    #[test]
    fn reproduce_1979() {
        gix_date::parse("fRi ", None).ok();
    }

    #[test]
    fn invalid_but_does_not_cause_panic() {
        for input in ["-9999-1-1", "7	-𬞋", "5 ڜ-09", "-4 week ago Z", "8960609 day ago"] {
            gix_date::parse(input, Some(std::time::UNIX_EPOCH)).ok();
        }
    }
}
