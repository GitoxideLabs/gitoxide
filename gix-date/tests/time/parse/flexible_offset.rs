use gix_date::Time;

#[test]
fn timezone_inference_preserves_jiffs_timestamp_range() -> gix_error::TestResult {
    let inputs = [
        "12/31/9999 00:00:00",
        "99991231T000000",
        "9999-12-31 00:00:00",
        "December 31 9999 00:00:00",
    ];
    for (offset, expected) in [
        ("+2359", Time::new(253_402_128_060, 86_340)),
        ("+23", Time::new(253_402_131_600, 82_800)),
    ] {
        for input in inputs {
            assert_eq!(
                gix_date::parse(&format!("{input} {offset}"), None)?,
                expected,
                "{input}: the local date is representable with its explicit offset even beyond the UTC timestamp limit"
            );
        }
    }
    let now: gix_date::Zoned = "2025-01-15T10:00:00+23:59[+23:59]".parse()?;
    for input in inputs {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            Time::new(253_402_128_060, 86_340),
            "{input}: an inferred offset retains the same range as an explicit offset"
        );
    }
    Ok(())
}

#[test]
fn numeric_dates_without_offsets_use_the_reference_timezone() -> gix_error::TestResult {
    for (reference, inputs, expected) in [
        (
            "2025-07-15T10:00:00[America/New_York]",
            [
                "2008-02-14 20:30:45",
                "2008-02-14T20:30:45",
                "02/14/2008 20:30:45",
                "14.02.2008 20:30:45",
                "20080214T203045",
            ],
            "2008-02-14 20:30:45 -0500",
        ),
        (
            "2025-01-15T10:00:00[America/New_York]",
            [
                "2008-07-14 20:30:45",
                "2008-07-14T20:30:45",
                "07/14/2008 20:30:45",
                "14.07.2008 20:30:45",
                "20080714T203045",
            ],
            "2008-07-14 20:30:45 -0400",
        ),
        (
            "2025-01-15T10:00:00[America/New_York]",
            [
                "2024-03-10 02:30:00",
                "2024-03-10T02:30:00",
                "03/10/2024 02:30:00",
                "10.03.2024 02:30:00",
                "20240310T023000",
            ],
            "2024-03-10 02:30:00 -0500",
        ),
        (
            "2025-07-15T10:00:00[America/New_York]",
            [
                "2024-11-03 01:30:00",
                "2024-11-03T01:30:00",
                "11/03/2024 01:30:00",
                "03.11.2024 01:30:00",
                "20241103T013000",
            ],
            "2024-11-03 01:30:00 -0400",
        ),
    ] {
        let now: gix_date::Zoned = reference.parse()?;
        let expected = gix_date::parse(expected, None)?;
        for input in inputs {
            assert_eq!(
                gix_date::parse(input, Some(now.clone()))?,
                expected,
                "{input}: the supplied target clock determines the offset independently of the reference season"
            );
            assert_eq!(
                gix_date::parse(input, None)?,
                Time::new(expected.seconds + i64::from(expected.offset), 0),
                "{input}: omitting the reference retains deterministic UTC behavior"
            );
            for offset in ["Z", " +0530", " -05", " -0001"] {
                let input = format!("{input}{offset}");
                assert_eq!(
                    gix_date::parse(&input, Some(now.clone()))?,
                    gix_date::parse(&input, None)?,
                    "{input}: an explicit offset takes precedence over the reference timezone"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn z_suffix_for_utc() {
    assert_eq!(
        gix_date::parse("1970-01-01 00:00:00 Z", None).unwrap(),
        Time { seconds: 0, offset: 0 },
        "1970-01-01 00:00:00 Z = Unix epoch"
    );
}

#[test]
fn two_digit_hour_offset() {
    assert_eq!(
        gix_date::parse("2008-02-14 20:30:45 -05", None).unwrap(),
        Time {
            seconds: 1203039045,
            offset: -18000,
        },
        "2008-02-14 20:30:45 -05 = 2008-02-14 20:30:45 -0500"
    );
}

#[test]
fn colon_separated_offset() {
    assert_eq!(
        gix_date::parse("2008-02-14 20:30:45 -05:00", None).unwrap(),
        Time {
            seconds: 1203039045,
            offset: -18000,
        },
        "2008-02-14 20:30:45 -05:00 = 2008-02-14 20:30:45 -0500"
    );
}

#[test]
fn fifteen_minute_offset() {
    assert_eq!(
        gix_date::parse("2008-02-14 20:30:45 -0015", None).unwrap(),
        Time {
            seconds: 1203021945,
            offset: -900,
        },
        "2008-02-14 20:30:45 -0015"
    );
}

#[test]
fn offsets_preserve_jiffs_range_and_precision() {
    // Recorded from git 2.50.1: `git commit --date="2022-01-01 12:00:00 +2359"` keeps `+2359`,
    // while `+2400` is not read as a timezone at all and git falls back to the local one.
    assert_eq!(
        gix_date::parse("2022-01-01 12:00:00 +2359", None).expect("git takes this offset"),
        Time {
            seconds: 1640952060,
            offset: 86340,
        },
        "23:59 is the widest offset git accepts, and it is accepted here too"
    );
    assert_eq!(
        gix_date::parse("2022-01-01 12:00:00 -2359", None).expect("git takes this offset"),
        Time {
            seconds: 1641124740,
            offset: -86340,
        },
        "the same holds for a negative offset"
    );

    for (input, offset) in [
        // Intentional difference: preserve all explicit offset seconds and supported hours;
        // Git truncates seconds or falls back to the local offset beyond 23:59.
        ("2022-01-01 12:00:00 +2400", 86400),
        ("2022-01-01 12:00:00 -2400", -86400),
        ("2022-01-01T12:00:00+24:00", 86400),
        ("2022-01-01 12:00:00 +2559", 93540),
        ("2022-01-01T12:00:00+23:59:59", 86399),
        ("2022-01-01T12:00:00+25:59:59", 93599),
        ("2022-01-01T12:00:00+01:02:03", 3723),
        ("2022-01-01T12:00:00-01:02:03", -3723),
    ] {
        assert_eq!(
            gix_date::parse(input, None).expect("Jiff can represent this offset"),
            Time {
                seconds: 1641038400 - i64::from(offset),
                offset
            },
            "the supplied offset determines the instant, even beyond Git's range"
        );
    }
    assert!(
        gix_date::parse("2022-01-01T12:00:00+26:00", None).is_err(),
        "offsets beyond Jiff's range still fail gracefully"
    );
}

#[test]
fn a_relative_date_keeps_the_offset_of_the_time_it_is_relative_to() {
    // Relative dates inherit the reference timezone, including Jiff's wider offsets.
    let now = jiff::Timestamp::from_second(1_700_000_000)
        .expect("a timestamp well inside range")
        .to_zoned(jiff::tz::TimeZone::fixed(jiff::tz::Offset::constant(25)));
    assert_eq!(
        gix_date::parse("1 day ago", Some(now))
            .expect("the input names no timezone, so there is nothing to reject")
            .offset,
        25 * 3600,
        "the offset of `now` is passed through untouched"
    );
}
