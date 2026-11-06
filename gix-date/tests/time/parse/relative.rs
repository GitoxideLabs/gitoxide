use std::time::SystemTime;

use jiff::{ToSpan, Zoned, tz::TimeZone};
use pretty_assertions::assert_eq;

fn utc(time: SystemTime) -> Zoned {
    Zoned::new(time.try_into().expect("system time is representable"), TimeZone::UTC)
}

#[test]
fn normalization_preserves_the_requested_local_clock() -> gix_testtools::TestResult {
    // Intentional difference: Git carries now's DST flag into the pre-transition clock,
    // producing March 9 at 23:00. March 10 midnight is 1710046800 with the actual -0500 offset.
    let now: Zoned = "2024-03-10T03:30:00[America/New_York]".parse()?;
    for input in ["today", "00:00", "midnight"] {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            gix_date::Time::new(1_710_046_800, -18000),
            "{input}: midnight stays on the requested day across a DST transition"
        );
    }
    Ok(())
}

#[test]
fn punctuation_between_relative_counts_and_units() -> gix_testtools::TestResult {
    let now: Zoned = "2009-08-30T19:20:00[UTC]".parse()?;
    let expected = gix_date::parse("2 days", Some(now.clone()))?;
    for input in ["2 -days", "2..days", "2//days", "2--days"] {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            expected,
            "{input}: separators without numeric date or offset fields remain relative punctuation"
        );
    }
    Ok(())
}

#[test]
fn delimited_numeric_dates_are_not_guessed_as_separate_fields() -> gix_testtools::TestResult {
    let now = jiff::Timestamp::from_second(1_251_660_000)?.to_zoned(TimeZone::UTC);
    for input in ["02/14/2008", "2008/02/14", "14.02.2008", "02/14/08", "02/30/2008"] {
        for reference in [None, Some(now.clone())] {
            assert!(
                gix_date::parse(input, reference).is_err(),
                "{input}: unsupported date-only formats must not produce an unrelated cutoff"
            );
        }
    }
    assert_eq!(
        gix_date::parse("02/14/2008 20:30:45 -0500", Some(now))?,
        gix_date::Time::new(1_203_039_045, -18000),
        "complete numeric dates retain their absolute interpretation"
    );
    Ok(())
}

#[test]
fn annotated_iso_dates_do_not_reach_relative_guessing() -> gix_testtools::TestResult {
    let now = jiff::Timestamp::from_second(1_251_660_000)?.to_zoned(TimeZone::UTC);
    for input in [
        "2008-02-14 20:30:45 -0500 (EST)",
        "2008-02-14T20:30:45-0500 (EST)",
        "2008-02-14t20:30:45-0500 (EST)",
        "2008-02-14, 20:30:45 -0500 (EST)",
        "2008-02-14 25:30:45 -0500",
    ] {
        for reference in [None, Some(now.clone())] {
            assert!(
                gix_date::parse(input, reference).is_err(),
                "{input}: unsupported absolute syntax must not become an unrelated relative cutoff"
            );
        }
    }
    assert_eq!(
        gix_date::parse("2008-02-14 20:30:45 -0500", Some(now))?,
        gix_date::Time::new(1_203_039_045, -18000),
        "supported absolute dates still retain their offset"
    );
    Ok(())
}

#[test]
fn never_resets_to_the_unix_epoch() -> gix_testtools::Result {
    // Intentional differences: .3.days applies after the reset, and today keeps January 1
    // instead of Git restoring the original reference's day (January 30).
    let now = jiff::Timestamp::from_second(1_251_660_000)?.to_zoned(TimeZone::UTC);
    for (input, seconds) in [
        ("never", 0),
        ("NEVER", 0),
        ("never noon", 43200),
        ("noon never", 0),
        ("1 day never", 0),
        ("1 never", 0),
        ("1never", 0),
        ("never now", 0),
        ("now never", 0),
        ("never 12:34:56.3.days.ago", -213904),
        ("today never", 0),
        ("never today", 0),
    ] {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            gix_date::Time::new(seconds, 0),
            "{input}: reset the calendar and clock at never, then apply subsequent operations"
        );
    }
    assert_eq!(
        gix_date::parse(" never ", None)?,
        gix_date::Time::new(0, 0),
        "the epoch sentinel needs no reference time"
    );
    Ok(())
}

#[test]
fn never_uses_the_local_epoch_calendar_and_historical_offset() -> gix_testtools::Result {
    for (zone, input, seconds, offset) in [
        ("Asia/Shanghai", "never", 0, 28800),
        ("Asia/Shanghai", "never noon", 14400, 28800),
        ("Asia/Shanghai", "never 12:34:56", 16496, 28800),
        ("America/New_York", "never", 0, -18000),
        ("America/New_York", "never 23:00", 14400, -18000),
        ("America/New_York", "noon never", 0, -18000),
    ] {
        let now = jiff::Timestamp::from_second(1_251_660_000)?.to_zoned(TimeZone::get(zone)?);
        assert_eq!(
            gix_date::parse(input, Some(now))?,
            gix_date::Time::new(seconds, offset),
            "{input} in {zone}: the epoch's local fields and offset replace those of now"
        );
    }
    Ok(())
}

#[test]
fn pending_numbers_fill_calendar_fields_and_units() -> gix_testtools::Result {
    // Intentional differences: 008 means eight, 00/38 fill years 2000/2038, and .3.days
    // always subtracts three days. Reject unused numbers instead of Git silently dropping them.
    let now = jiff::Timestamp::from_second(1_251_660_000)?.to_zoned(TimeZone::UTC);
    for (input, seconds) in [
        ("2 long days ago", 1251487200),
        ("one or two days ago", 1251573600),
        ("5 noon", 1249473600),
        ("5 6 noon", 1244203200),
        ("5 6 2008 noon", 1212667200),
        ("5 6 08 noon", 1212667200),
        ("5 6 38 noon", 2159352000),
        ("5 6 00 noon", 960206400),
        ("5 6 70 noon", 13435200),
        ("37 noon", 2135246400),
        ("two", 1249240800),
        ("2 nonsense", 1249240800),
        ("008 days", 1250968800),
        ("12:34:56.008 days", 1250944496),
        ("12:34:56.08 days", 1250944496),
        ("2 now", 1251660000),
        ("2 yesterday", 1251573600),
        ("5 6 2008 12:34:56.3.days.ago", 1212410096),
    ] {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            gix_date::Time::new(seconds, 0),
            "{input}: pending numbers survive filler words and fill calendar fields only when flushed"
        );
    }
    for input in ["1745582210 +2400", "1313584730 +000001", "@1745582210 +2400"] {
        for reference in [None, Some(now.clone())] {
            assert!(
                gix_date::parse(input, reference).is_err(),
                "{input}: malformed raw dates do not become relative dates"
            );
        }
    }
    for input in ["florx ago", "days", "2 hours 3", "12345 florx ago", "0 nonsense"] {
        assert!(
            gix_date::parse(input, Some(now.clone())).is_err(),
            "{input}: no meaningful date token"
        );
    }
    Ok(())
}

#[test]
fn textual_month_fragments_infer_missing_calendar_fields() -> gix_testtools::Result {
    // Intentional differences: retain padded numbers, attached month/day tokens and explicit
    // dates after duration arithmetic; Git can ignore each and return an unrelated cutoff.
    let now = jiff::Timestamp::from_second(1_251_660_000)?.to_zoned(TimeZone::UTC);
    for (input, seconds) in [
        ("July 5th", 1246821600),
        ("5 July", 1246821600),
        ("July", 1248981600),
        ("December", 1230664800),
        ("December 31", 1230751200),
        ("August 31", 1251746400),
        ("January 5th noon pm", 1231156800),
        ("6AM, June 7, 2009", 1244354400),
        ("June 7 6am 2009", 1244354400),
        ("Dec 6, 1992", 723669600),
        ("Dec 02", 1228245600),
        ("Dec 0002", 1228245600),
        ("June 2008", 1214853600),
        ("June 7 10", 1275938400),
        ("June 7 38", 2159551200),
        ("June 7 70", 13634400),
        ("June 7 00", 960405600),
        ("June 7 0008", 1212866400),
        ("June 7 2008 12:34:56.3.days.ago", 1212582896),
        ("July 5 2 days ago", 1246648800),
        ("2 days July 5", 1246821600),
        ("June July 5", 1246821600),
        ("now December", 1262200800),
        ("December now", 1230664800),
        ("Sept 5", 1220642400),
        ("Septe 5", 1220642400),
        ("JUNE7", 1244402400),
        ("6AM, June7, 2009", 1244354400),
        ("July 5th noon", 1246795200),
    ] {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            gix_date::Time::new(seconds, 0),
            "{input}: infer missing fields, retain explicit decimal dates, and reject impossible dates"
        );
        assert!(
            gix_date::parse(input, None).is_err(),
            "{input}: calendar inference requires now"
        );
    }
    Ok(())
}

#[test]
fn relative_month_names_accept_prefixes_and_attached_days() -> gix_testtools::Result {
    let now = jiff::Timestamp::from_second(1_251_660_000)?.to_zoned(TimeZone::UTC);
    for (index, name) in [
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
        let month = index + 1;
        let year = if month > 8 { 2008 } else { 2009 };
        let expected = gix_date::parse(&format!("{year}-{month:02}-05 19:20:00 +0000"), None)?;
        for length in 3..=name.len() {
            let input = format!("{} 5th", name[..length].to_ascii_uppercase());
            assert_eq!(
                gix_date::parse(&input, Some(now.clone()))?,
                expected,
                "{input}: case-insensitive month prefix"
            );
        }
    }
    for input in ["Ju", "Julyish"] {
        assert!(
            gix_date::parse(input, Some(now.clone())).is_err(),
            "{input}: not a month name"
        );
    }
    Ok(())
}

#[test]
fn textual_month_inference_uses_local_calendar_fields() -> gix_testtools::Result {
    let now = jiff::Timestamp::from_second(1_251_660_000)?.to_zoned(TimeZone::fixed(jiff::tz::Offset::from_hours(8)?));
    for (input, expected) in [
        ("July 5th", "2009-07-05 03:20:00 +0800"),
        ("December", "2008-12-31 03:20:00 +0800"),
        ("6AM, June 7, 2009", "2009-06-07 06:00:00 +0800"),
        ("September 5", "2008-09-05 03:20:00 +0800"),
    ] {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            gix_date::parse(expected, None)?,
            "{input}: missing fields come from local now, not UTC"
        );
    }
    Ok(())
}

#[test]
fn fractional_clock_suffixes_do_not_become_calendar_days() -> gix_testtools::Result {
    let now = jiff::Timestamp::from_second(1_251_616_800)?.to_zoned(TimeZone::UTC);
    assert_eq!(
        gix_date::parse("December 37 noon 12:34:56.3", Some(now))?,
        gix_date::parse("2037-12-29 12:34:56 +0000", None)?,
        "a fractional suffix without a following unit is discarded, independently of calendar inference"
    );
    Ok(())
}

#[test]
fn large_offsets() {
    gix_date::parse("999999999999999 weeks ago", Some(utc(SystemTime::UNIX_EPOCH))).ok();
}

#[test]
fn large_offsets_do_not_panic() {
    insta::assert_debug_snapshot!(gix_date::parse("9999999999 weeks ago", Some(utc(SystemTime::UNIX_EPOCH)))
            .expect_err("large offsets do not panic"), "large offsets do not panic", @"
    Couldn't parse span from 'week 9999999999'

    Caused by:
        0: Relative date is out of range
        1: parameter 'Unix timestamp seconds' is not in the required range of -377705023201..=253402207200
    ");
    insta::assert_debug_snapshot!(gix_date::parse(
            "2027 years 9223372036854775807 months ago",
            Some(utc(
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_774_958_400)
            )),
        )
        .expect_err("month subtraction beyond Jiff's range must fail"), "large offsets do not panic", @"
    Couldn't parse span from 'month 9223372036854775807'

    Caused by:
        0: parameter 'months' is not in the required range of -239976..=239976
    ");
}

#[test]
fn offset_leading_to_before_unix_epoch_can_be_represented() {
    let date = gix_date::parse("1 second ago", Some(utc(SystemTime::UNIX_EPOCH))).unwrap();
    assert_eq!(date.seconds, -1);
}

#[test]
fn the_timezone_of_now_controls_calendar_arithmetic() -> gix_testtools::TestResult {
    let now = jiff::Timestamp::from_second(1_772_325_000)?.to_zoned(TimeZone::fixed(jiff::tz::Offset::from_hours(-5)?));
    let actual = gix_date::parse("1 month ago", Some(now))?;
    assert_eq!(
        actual.seconds, 1_769_646_600,
        "local February 28th becomes January 28th"
    );
    assert_eq!(actual.offset, -5 * 60 * 60, "the timezone offset is retained");
    Ok(())
}

#[test]
fn counted_weekdays_use_the_local_weekday() -> gix_testtools::Result {
    let now = jiff::Timestamp::from_second(1_251_660_000)?.to_zoned(TimeZone::fixed(jiff::tz::Offset::from_hours(8)?));
    let actual = gix_date::parse("last monday", Some(now))?;
    assert_eq!(
        actual.seconds,
        1_251_660_000 - 7 * 86400,
        "Monday in the supplied timezone goes back a full week, even though UTC is still Sunday"
    );
    assert_eq!(actual.offset, 8 * 3600, "the supplied timezone is retained");
    Ok(())
}

#[test]
fn weekday_selection_and_fixed_duration_days_have_distinct_dst_semantics() -> gix_testtools::Result {
    // Intentional difference: weekday selection preserves 01:30 across DST, while Git uses
    // 86400 seconds and shifts the clock. Explicit day units retain their duration meaning.
    for (input, now, expected) in [
        (
            "last sunday",
            "2024-03-11T01:30:00-04:00[America/New_York]",
            "2024-03-10T00:30:00-05:00[America/New_York]",
        ),
        (
            "last sunday",
            "2024-11-04T01:30:00-05:00[America/New_York]",
            "2024-11-03T01:30:00-05:00[America/New_York]",
        ),
        (
            "last saturday",
            "2024-11-03T01:30:00-05:00[America/New_York]",
            "2024-11-02T02:30:00-04:00[America/New_York]",
        ),
    ] {
        let now: Zoned = now.parse()?;
        let expected: Zoned = expected.parse()?;
        for input in [input, "1 day ago"] {
            let expected = if input == "1 day ago" {
                expected.clone()
            } else {
                now.datetime()
                    .checked_sub(jiff::SignedDuration::from_hours(24))?
                    .to_zoned(now.time_zone().clone())?
            };
            let actual = gix_date::parse(input, Some(now.clone()))?;
            assert_eq!(
                actual.seconds,
                expected.timestamp().as_second(),
                "{input:?}: weekday selection preserves the local clock; day units subtract 86400 seconds"
            );
            assert_eq!(
                actual.offset,
                expected.offset().seconds(),
                "{input:?}: the resulting instant determines the timezone offset"
            );
        }
    }
    Ok(())
}

#[test]
fn counted_weekdays_require_now_and_check_overflow() {
    assert!(
        gix_date::parse("last tuesday", None).is_err(),
        "a weekday needs a reference time"
    );
    for input in [
        "9223372036854775807 mondays ago",
        "15250284452472 mondays ago",
        "100000000 mondays ago",
    ] {
        assert!(
            gix_date::parse(input, Some(utc(SystemTime::UNIX_EPOCH))).is_err(),
            "{input:?} exceeds the day count, seconds count, or representable timestamp range"
        );
    }
}

#[test]
fn named_clocks_compose_with_durations_and_calendar_weekdays() -> gix_testtools::Result {
    // Git 2.55 fixed deferred named clocks. Intentional difference: after month subtraction,
    // compute the actual weekday; July 30, 2009 is Thursday, so last Friday is July 24.
    // Git's stale August weekday instead selects July 28, a Tuesday.
    let now = jiff::Timestamp::from_second(1_251_616_800)?.to_zoned(TimeZone::UTC);
    for (input, seconds) in [
        ("noon yesterday", 1251547200),
        ("yesterday noon", 1251547200),
        ("yesterday tea", 1251565200),
        ("last Friday at noon", 1251460800),
        ("tea last saturday", 1251565200),
        ("noon 1 day ago", 1251547200),
        ("1 day ago noon", 1251547200),
        ("1 month noon", 1248955200),
        ("1 month noon last Friday", 1248436800),
        ("noon midnight tea", 1251565200),
        ("now noon", 1251633600),
    ] {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            gix_date::Time::new(seconds, 0),
            "{input:?}: weekday selectors use the calendar after the preceding operations"
        );
    }
    Ok(())
}

#[test]
fn named_clocks_use_the_reference_timezone() -> gix_testtools::Result {
    // This instant is Monday morning locally, but still Sunday evening in UTC.
    let now = jiff::Timestamp::from_second(1_251_660_000)?.to_zoned(TimeZone::fixed(jiff::tz::Offset::from_hours(8)?));
    for (input, expected) in [
        ("noon", "2009-08-30 12:00:00 +0800"),
        ("midnight", "2009-08-31 00:00:00 +0800"),
        ("tea", "2009-08-30 17:00:00 +0800"),
        ("now noon", "2009-08-31 12:00:00 +0800"),
        ("tea last saturday", "2009-08-29 17:00:00 +0800"),
    ] {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            gix_date::parse(expected, None)?,
            "{input:?}: clock and weekday adjustments use the supplied timezone"
        );
        assert!(
            gix_date::parse(input, None).is_err(),
            "{input:?} needs a reference date"
        );
    }
    Ok(())
}

#[test]
fn numeric_clocks_keep_the_local_date_and_token_boundaries() -> gix_testtools::Result {
    let now = jiff::Timestamp::from_second(1_251_660_000)?.to_zoned(TimeZone::fixed(jiff::tz::Offset::from_hours(8)?));
    for (input, expected) in [
        ("15:00", "2009-08-31 15:00:00 +0800"),
        ("24:00", "2009-09-01 00:00:00 +0800"),
        ("12:34:56.3.days.ago", "2009-08-28 12:34:56 +0800"),
        ("12:34:56\u{2003}1day", "2009-08-30 12:34:56 +0800"),
    ] {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            gix_date::parse(expected, None)?,
            "{input:?}: preserve clocks, distinguish subsequent counts, and use the local date"
        );
        assert!(gix_date::parse(input, None).is_err(), "a clock alone needs a date");
    }
    Ok(())
}

#[test]
fn clock_assignments_clear_fractional_seconds() -> gix_testtools::Result {
    let now = jiff::Timestamp::new(0, 500_000_000)?.to_zoned(TimeZone::UTC);
    for input in ["midnight 1 second ago", "00:00 1 second ago", "12am 1 second ago"] {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            gix_date::Time::new(-1, 0),
            "{input:?}: assigning midnight clears the cached fractional second before subtraction"
        );
    }
    Ok(())
}

#[test]
fn am_pm_adjustments_use_local_clock_components() -> gix_testtools::Result {
    let now = jiff::Timestamp::from_second(1_251_660_000)?.to_zoned(TimeZone::fixed(jiff::tz::Offset::from_hours(8)?));
    for (input, expected) in [
        ("6pm", "2009-08-31 18:00:00 +0800"),
        ("6:30:45pm", "2009-08-31 18:30:45 +0800"),
        ("12am", "2009-08-31 00:00:00 +0800"),
        ("12pm", "2009-08-31 12:00:00 +0800"),
        ("PM", "2009-08-31 15:20:00 +0800"),
        ("6am yesterday", "2009-08-30 06:00:00 +0800"),
    ] {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            gix_date::parse(expected, None)?,
            "{input:?}: AM/PM adjusts the local hour and only clears minutes/seconds with an explicit nonzero hour"
        );
        assert!(gix_date::parse(input, None).is_err(), "AM/PM still needs a date");
    }
    Ok(())
}

#[test]
fn today_preserves_explicit_clocks_and_the_accumulated_date() -> gix_testtools::Result {
    // Intentional differences: today preserves explicit clocks even when equal to now,
    // and the accumulated date after subtraction. Git erases an equal clock and restores
    // the original day of the month. Dot-separated count/unit pairs always apply.
    for seconds in [1_251_616_800, 1_251_660_000] {
        let now = jiff::Timestamp::from_second(seconds)?.to_zoned(TimeZone::UTC);
        for (input, expected) in [
            ("today", 1251590400),
            ("TODAY", 1251590400),
            ("noon today", 1251633600),
            ("today at noon", 1251633600),
            ("6pm today", 1251655200),
            ("today 6pm", 1251655200),
            ("6am today", 1251612000),
            ("today now", 1251590400),
            ("now today", 1251590400),
            ("1 day today", 1251504000),
            ("today 1 day", 1251504000),
            ("1 month today", 1248912000),
            ("today 1 month", 1248912000),
            ("now today 12:34:56.3.days.ago", 1251376496),
            ("07:20 today", 1251616800),
        ] {
            assert_eq!(
                gix_date::parse(input, Some(now.clone()))?,
                gix_date::Time::new(expected, 0),
                "{input:?} from {now}: today preserves an explicit clock, otherwise defaults to midnight"
            );
        }
    }
    Ok(())
}

#[test]
fn today_uses_local_midnight() -> gix_testtools::Result {
    let now = jiff::Timestamp::from_second(1_251_660_000)?.to_zoned(TimeZone::fixed(jiff::tz::Offset::from_hours(8)?));
    for (input, expected) in [
        ("today", "2009-08-31 00:00:00 +0800"),
        ("today at noon", "2009-08-31 12:00:00 +0800"),
        ("noon today", "2009-08-31 12:00:00 +0800"),
    ] {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            gix_date::parse(expected, None)?,
            "{input:?}: today refers to the supplied local date, not UTC"
        );
        assert!(gix_date::parse(input, None).is_err(), "today needs a reference date");
    }
    Ok(())
}

#[test]
fn numeric_clocks_check_component_and_timestamp_ranges() {
    for input in [
        "25:00",
        "24:01",
        "23:59:60",
        "12:60",
        "12:00:61",
        "99999999999999999999:00",
    ] {
        assert!(
            gix_date::parse(input, Some(utc(SystemTime::UNIX_EPOCH))).is_err(),
            "{input:?} is not a clock that Git's set_time() accepts"
        );
    }
    assert!(
        gix_date::parse("24:00", Some(jiff::Timestamp::MAX.to_zoned(TimeZone::UTC))).is_err(),
        "clock rollover beyond the timestamp range returns an error"
    );
}

#[test]
fn various() {
    // A fixed reference makes comparisons with Jiff reproducible. Days through weeks are
    // fixed-duration units; months and years constrain the target day to its month's length.
    let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
    let cases = [
        ("5 seconds ago", 5.seconds()),
        ("12345 seconds ago", 12_345.seconds()),
        ("5 minutes ago", 5.minutes()),
        ("5 hours ago", 5.hours()),
        ("5 days ago", 5.days()),
        ("2 Days 1 hour ago", 49.hours()),
        ("2 Days ago 1 hour", 49.hours()),
        ("2 Days and 23 hours ago", 71.hours()),
        ("last day", 24.hours()),
        ("3 weeks ago", 3.weeks()),
        ("21 days ago", 21.days()),              // 3 weeks
        ("504 hours ago", 504.hours()),          // 3 weeks
        ("30240 minutes ago", 30_240.minutes()), // 3 weeks
        ("2 months ago", 2.months()),
        ("1460 hours ago", 1460.hours()),        // 2 months
        ("87600 minutes ago", 87_600.minutes()), // 2 months
        ("14 weeks ago", 14.weeks()),
        ("98 days ago", 98.days()),                // 14 weeks
        ("2352 hours ago", 2352.hours()),          // 14 weeks
        ("141120 minutes ago", 141_120.minutes()), // 14 weeks
        ("5 months ago", 5.months()),
        ("3650 hours ago", 3650.hours()),          // 5 months
        ("219000 minutes ago", 219_000.minutes()), // 5 months
        ("26 weeks ago", 26.weeks()),
        ("182 days ago", 182.days()),              // 26 weeks
        ("4368 hours ago", 4368.hours()),          // 26 weeks
        ("262080 minutes ago", 262_080.minutes()), // 26 weeks
        ("8 months ago", 8.months()),
        ("5840 hours ago", 5840.hours()),          // 8 months
        ("350400 minutes ago", 350_400.minutes()), // 8 months
        ("38 weeks ago", 38.weeks()),
        ("266 days ago", 266.days()),              // 38 weeks
        ("6384 hours ago", 6384.hours()),          // 38 weeks
        ("383040 minutes ago", 383_040.minutes()), // 38 weeks
        ("11 months ago", 11.months()),
        ("8030 hours ago", 8030.hours()),          // 11 months
        ("481800 minutes ago", 481_800.minutes()), // 11 months
        ("14 months ago", 14.months()),            // "1 year, 2 months ago" not yet supported.
        ("21 months ago", 21.months()),            // "1 year, 9 months ago" not yet supported.
        ("2 years ago", 2.years()),
        ("20 years ago", 20.years()),
        ("630720000 seconds ago", 630_720_000.seconds()), // 20 years
    ];

    let cases_with_times = cases.map(|(input, _)| {
        let time = gix_date::parse(input, Some(utc(now))).expect("relative time string should parse to a Time");
        (input, time)
    });
    assert_eq!(
        cases_with_times.map(|(_, time)| time.offset),
        cases_with_times.map(|_| 0),
        "They don't pick up local time"
    );

    let expected = cases.map(|(input, span)| {
        let expected = utc(now)
            // account for the loss of precision when creating `Time` with seconds
            .round(
                jiff::ZonedRound::new()
                    .smallest(jiff::Unit::Second)
                    .mode(jiff::RoundMode::Trunc),
            )
            .expect("test needs to truncate current timestamp to seconds")
            .saturating_sub(span)
            .timestamp();

        (input, expected)
    });
    let actual = cases_with_times.map(|(input, time)| {
        let actual = jiff::Timestamp::from_second(time.seconds)
            .expect("seconds obtained from a Time should convert to Timestamp");
        (input, actual)
    });
    assert_eq!(actual, expected);
}

#[test]
fn months_and_years_constrain_month_ends() -> gix_testtools::TestResult {
    // Intentional difference: constrain each subtraction to the target month's last day.
    // Git rolls invalid days into the following month; March 31 minus a month can stay in March.
    // These expected dates are derived from calendar lengths, including leap February and ordering.
    let cases = [
        (1774958400, "1 month ago", 1772280000),               // 2026-02-28 at noon UTC
        (1780228800, "1 month ago", 1777550400),               // 2026-04-30 at noon UTC
        (1788177600, "6 months ago", 1772280000),              // 2026-02-28 at noon UTC
        (1774872000, "1 month ago", 1772280000),               // 2026-02-28 at noon UTC
        (1711800000, "1 month ago", 1709208000),               // 2024-02-29 at noon UTC
        (1711886400, "1 month ago", 1709208000),               // 2024-02-29 at noon UTC
        (1709208000, "1 year ago", 1677585600),                // 2023-02-28 at noon UTC
        (1709208000, "12 months ago", 1677585600),             // 2023-02-28 at noon UTC
        (1774958400, "2 months ago", 1769860800),              // 2026-01-31 at noon UTC
        (1774958400, "1 year ago", 1743422400),                // 2025-03-31 at noon UTC
        (1775044800, "1 day 1 month ago", 1772280000),         // 2026-02-28 at noon UTC
        (1775044800, "1 month 1 day ago", 1772280000),         // 2026-02-28 at noon UTC
        (1711886400, "1 month 1 year ago", 1677585600),        // 2023-02-28 at noon UTC
        (1711886400, "1 year 1 month ago", 1677585600),        // 2023-02-28 at noon UTC
        (1774958400, "1 month 1 month ago", 1769601600),       // 2026-01-28 at noon UTC
        (1769860800, "23 months ago", 1709208000),             // 2024-02-29 at noon UTC
        (1780228800, "1 month 1 day 1 month ago", 1774785600), // 2026-03-29 at noon UTC
        (1780228800, "1 month 1 month 1 day ago", 1774785600), // 2026-03-29 at noon UTC
        (1774958400, "1 day 1 day ago", 1774785600),           // 2026-03-29 at noon UTC
    ];
    for (now, input, expected) in cases {
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(now);
        let actual = gix_date::parse(input, Some(utc(now)))?.seconds;
        assert_eq!(
            actual, expected,
            "{input}: stay in the target month, constraining each ordered subtraction"
        );
    }
    Ok(())
}

#[test]
fn various_examples() {
    #[rustfmt::skip]
    let expected = [
    // ### 1. SHORT Format
        "2018-12-24",
        "1950-01-01",
        "1970-01-01",
        "2024-12-31",

    // ### 2. RFC2822 Format
        "Thu, 18 Aug 2022 12:45:06 +0800",
        "Mon Oct 27 10:30:00 2023 -0800",

    // ### 3. GIT_RFC2822 Format
        "Thu, 8 Aug 2022 12:45:06 +0800",
        "Mon Oct 27 10:30:00 2023 -0800",

    // ### 4. ISO8601 Format
        "2022-08-17 22:04:58 +0200",
        "1970-01-01 00:00:00 -0500",

    // ### 5. ISO8601_STRICT Format
        "2022-08-17T21:43:13+08:00",

    // ### 6. UNIX Timestamp (Seconds Since Epoch)
        "123456789",
        "@0",
        "@-1000",
        "@20080214",
        "@99999999",
        "1700000000",


    // ### 7. RAW Format
        "1745582210 +0200",
        "1660874655 +0800",
        "-1660874655 +0800",
    // ### 8. GITOXIDE Format
        "Thu Sep 04 2022 10:45:06 -0400",
        "Mon Oct 27 2023 10:30:00 +0000",
    // ### 9. DEFAULT Format
        "Thu Sep 4 10:45:06 2022 -0400",
        "Mon Oct 27 10:30:00 2023 +0000",
    ];
    for date in expected {
        _ = gix_date::parse(date, None).unwrap_or_else(|err| unreachable!("{date}: all examples can be parsed: {err}"));
    }
}

mod named {
    use std::time::{Duration, SystemTime};

    use super::utc;

    #[test]
    fn now() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let actual = gix_date::parse("now", Some(utc(now))).unwrap();
        assert_eq!(actual.seconds, 1_000_000);
    }

    #[test]
    fn today() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let actual = gix_date::parse(" today ", Some(utc(now))).expect("today resolves with a reference time");
        assert_eq!(
            actual.seconds, 950_400,
            "today is local midnight, independently of surrounding whitespace"
        );
    }

    #[test]
    fn yesterday() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let actual = gix_date::parse("yesterday", Some(utc(now))).unwrap();
        assert_eq!(
            actual.seconds,
            1_000_000 - 86400,
            "yesterday is 1 day (86400 seconds) before now"
        );
    }

    #[test]
    fn now_case_insensitive() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let actual = gix_date::parse("NOW", Some(utc(now))).unwrap();
        assert_eq!(actual.seconds, 1_000_000);
    }
}
