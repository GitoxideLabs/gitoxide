use gix_date::Time;

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
        ("2022-01-01 12:00:00 +2400", 86400),
        ("2022-01-01 12:00:00 -2400", -86400),
        ("2022-01-01T12:00:00+24:00", 86400),
        ("2022-01-01 12:00:00 +2559", 93540),
        ("2022-01-01T12:00:00+23:59:59", 86399),
        ("2022-01-01T12:00:00+25:59:59", 93599),
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
