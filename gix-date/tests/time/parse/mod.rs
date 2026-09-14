use gix_date::Time;
use gix_error::Exn;

#[test]
fn time_without_offset_defaults_to_utc() {
    // Git parses datetime without offset and defaults to UTC (+0000)
    let result = gix_date::parse("1979-02-26 18:30:00", Some(gix_date::Zoned::now()));
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
        gix_date::parse("1979-02-26", Some(gix_date::Zoned::now())).unwrap(),
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

    assert!(gix_date::parse("1313584730 +000001", None).is_err());
    for (input, offset) in [
        ("1313584730 +1500", 15 * 3600),
        ("1313584730 +0001", 60),
        ("1313584730 +000100", 60),
        ("@1313584730 -2359", -(23 * 3600 + 59 * 60)),
    ] {
        assert_eq!(
            gix_date::parse(input, None)?,
            Time {
                seconds: 1313584730,
                offset
            },
            "raw timestamps retain all valid hour and minute offsets"
        );
    }

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
        "Unknown date format: \"foobar\""
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

mod textual_dates {
    use gix_date::Time;

    #[test]
    fn complete_layouts_ordinals_and_offsets() -> gix_testtools::Result {
        let now = jiff::Timestamp::from_second(1_000_000_000)?.to_zoned(jiff::tz::TimeZone::UTC);
        for (input, seconds, offset) in [
            ("February 14, 2008 20:30:45 -0500", 1203039045, -18000),
            ("February 14th, 2008 20:30:45 -0500", 1203039045, -18000),
            ("14th February 2008 20:30:45 -0500", 1203039045, -18000),
            ("Feb 14 20:30:45 2008 -0500", 1203039045, -18000),
            ("Monday, fEbRu 14st, 2008 20:30:45 -0500", 1203039045, -18000),
            ("February 14 2008 20:30 -05", 1203039000, -18000),
            ("February 14 2008 20:30:45 -05:00", 1203039045, -18000),
            ("February 14 2008 20:30:45 CET", 1203017445, 3600),
            ("February 14 2008 20:30:45 Z", 1203021045, 0),
            ("February 14 2008 20:30:45 +2359", 1202934705, 86340),
            ("February 14 2008 20:30:45 -2359", 1203107385, -86340),
            ("February 14 2008 20:30:45 -0001", 1203021105, -60),
        ] {
            for reference in [None, Some(now.clone())] {
                assert_eq!(
                    gix_date::parse(input, reference).map_err(gix_error::Exn::into_error)?,
                    Time::new(seconds, offset),
                    "{input}: complete textual dates retain their offset and do not depend on now"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn month_names_accept_case_insensitive_prefixes() -> gix_testtools::Result {
        for (index, month) in [
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
        ]
        .into_iter()
        .enumerate()
        {
            let expected = gix_date::parse(&format!("2008-{:02}-14 20:30:45 +0000", index + 1), None)
                .map_err(gix_error::Exn::into_error)?;
            for length in 3..=month.len() {
                let input = format!("{} 14th, 2008 20:30:45 +0000", month[..length].to_ascii_uppercase());
                assert_eq!(
                    gix_date::parse(&input, None).map_err(gix_error::Exn::into_error)?,
                    expected,
                    "{input}: Git accepts every month-name prefix with at least three letters"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn calendar_and_clock_overflow_matches_git() -> gix_testtools::Result {
        for (input, seconds) in [
            ("Feb 29 2009 20:30:45 +0000", 1235939445),
            ("Feb 31 2008 20:30:45 +0000", 1204489845),
            ("Feb 14 2008 24:00:00 +0000", 1203033600),
            ("Feb 14 2008 23:59:60 +0000", 1203033600),
        ] {
            assert_eq!(
                gix_date::parse(input, None).map_err(gix_error::Exn::into_error)?,
                Time::new(seconds, 0),
                "{input}: Git normalizes calendar and clock overflow instead of rejecting it"
            );
        }
        Ok(())
    }

    #[test]
    fn incomplete_or_malformed_dates_are_not_absolute() {
        for input in [
            "Fe 14 2008 20:30:45 +0000",
            "Februbbish 14 2008 20:30:45 +0000",
            "February14 2008 20:30:45 +0000",
            "14-Feb-2008 20:30:45 +0000",
            "Feb-14-2008 20:30:45 +0000",
            "February 14 20:30:45 +0000",
            "February 14 2008 +0000",
            "February 32 2008 20:30:45 +0000",
            "February 14 2008 25:00:00 +0000",
            "February 14 2008 12:60:00 +0000",
            "February 14 2008 12:00:61 +0000",
            "February 14 2008 12:00:00:00 +0000",
            "February 14 2008 20:30:45 +05::00",
            "February 14 2008 20:30:45 +01-1",
            "February 14\n2008 20:30:45 +0000",
        ] {
            assert!(
                gix_date::parse(input, None).is_err(),
                "{input}: outside the supported complete textual-date grammar"
            );
        }
    }
}

mod numeric_dates {
    use gix_date::Time;

    #[test]
    fn short_year_mapping_is_value_based() {
        for (year, seconds) in [
            ("0", 950560245),
            ("00", 950560245),
            ("000", 950560245),
            ("8", 1203021045),
            ("08", 1203021045),
            ("008", 1203021045),
            ("10", 1266179445),
            ("37", 2118256245),
            ("71", 35411445),
            ("071", 35411445),
            ("99", 919024245),
        ] {
            for date in [format!("2/14/{year}"), format!("14.2.{year}")] {
                let input = format!("{date} 20:30:45 +0000");
                assert_eq!(
                    gix_date::parse(&input, None).expect("complete numeric dates need no reference time"),
                    Time::new(seconds, 0),
                    "{input}: Git expands numeric years by value, not a conventional two-digit-year pivot"
                );
            }
        }
    }

    #[test]
    fn separator_preference_fallback_and_year_first() {
        for (date, seconds) in [
            ("02/03/08", 1202070645),
            ("02.03.08", 1204489845),
            ("14/02/08", 1203021045),
            ("02.14.08", 1203021045),
            ("99/02/14", 919024245),
            ("99/14/02", 919024245),
            ("71.02.14", 35411445),
            ("71.14.02", 35411445),
            ("071/2/14", 35411445),
            ("08/02/14", 1407011445),
            ("08.02.14", 1391891445),
        ] {
            let input = format!("{date} 20:30:45 +0000");
            assert_eq!(
                gix_date::parse(&input, None).expect("all three numeric date fields and a clock are present"),
                Time::new(seconds, 0),
                "{input}: only a first value greater than 70 takes precedence as the year"
            );
        }
        for date in ["02/14/08", "14.02.08"] {
            let input = format!("{date} 20:30:45 -0500");
            assert_eq!(
                gix_date::parse(&input, None).expect("short years accept the existing time and offset syntax"),
                Time::new(1203039045, -18000),
                "{input}: normalizing the date preserves the instant and explicit offset"
            );
        }
    }

    #[test]
    fn four_digit_years_keep_existing_interpretation() {
        for date in [
            "2008/02/14",
            "2008.02.14",
            "2008/14/02",
            "2008.14.02",
            "02/14/2008",
            "14/02/2008",
            "14.02.2008",
            "02.14.2008",
        ] {
            let input = format!("{date} 20:30:45 -0500");
            assert_eq!(
                gix_date::parse(&input, None).expect("four-digit dates retain both field orderings"),
                Time::new(1203039045, -18000),
                "{input}: short-year support does not change existing four-digit dates"
            );
        }
        for year in ["0008", "1950", "2100"] {
            let expected = gix_date::parse(&format!("{year}-02-14 20:30:45 +0000"), None)
                .expect("ISO dates retain Jiff's wider year range");
            for date in [format!("{year}/02/14"), format!("14.02.{year}")] {
                let input = format!("{date} 20:30:45 +0000");
                assert_eq!(
                    gix_date::parse(&input, None).expect("four-digit years remain literal"),
                    expected,
                    "{input}: previously accepted four-digit years are not expanded or restricted to Git's range"
                );
            }
        }
    }

    #[test]
    fn unsupported_years_and_malformed_dates_are_rejected() {
        for date in [
            "02/14/38",
            "02/14/69",
            "02/14/70",
            "14.02.38",
            "14.02.70",
            "02/14/100",
            "02/14/32768",
            "32768/02/14",
            "02/14/+08",
            "02/14/-08",
            "02/14/08/01",
            "02.14/08",
            "02/00/08",
            "00/14/08",
            "02/29/01",
        ] {
            let input = format!("{date} 20:30:45 +0000");
            assert!(
                gix_date::parse(&input, None).is_err(),
                "{input}: short-year support does not guess individual tokens or normalize invalid calendar dates"
            );
        }
        assert!(
            gix_date::parse("02/14/08", None).is_err(),
            "a numeric date without a clock does not acquire one from the current time"
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
    use std::path::PathBuf;

    fn fuzz_artifact_paths(target: &str) -> Vec<PathBuf> {
        let mut paths = std::fs::read_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("fuzz/artifacts")
                .join(target),
        )
        .expect("artifact directory exists")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    #[test]
    fn reproduce_1979() {
        gix_date::parse("fRi ", None).ok();
    }

    #[test]
    fn artifact_inputs_can_be_parsed_without_panicking() {
        for path in fuzz_artifact_paths("parse") {
            let input = std::fs::read(path).expect("artifact is readable");
            if let Ok(input) = std::str::from_utf8(&input) {
                gix_date::parse(input, None).ok();
            }
        }
    }

    #[test]
    fn invalid_but_does_not_cause_panic() {
        for input in ["-9999-1-1", "7	-𬞋", "5 ڜ-09", "-4 week ago Z", "8960609 day ago"] {
            gix_date::parse(
                input,
                Some(jiff::Timestamp::UNIX_EPOCH.to_zoned(jiff::tz::TimeZone::UTC)),
            )
            .ok();
        }
    }
}
