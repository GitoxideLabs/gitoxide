use gix_date::Time;
use gix_testtools::TestResult;

#[test]
fn calendar_subtraction_stays_in_the_target_month() -> TestResult {
    // Intentional difference: Jiff constrains the day to the target month. Git rolls
    // February 31 into March 3, so subtracting one month can stay in March.
    for (reference, input, expected) in [
        ("2026-03-31T12:00:00[UTC]", "1 month ago", "2026-02-28 12:00:00 +0000"),
        ("2024-02-29T12:00:00[UTC]", "1 year ago", "2023-02-28 12:00:00 +0000"),
        (
            "2026-03-31T12:00:00[UTC]",
            "1 month 1 month ago",
            "2026-01-28 12:00:00 +0000",
        ),
    ] {
        assert_eq!(
            gix_date::parse(input, Some(reference.parse()?))?,
            gix_date::parse(expected, None)?,
            "{input}: clamp each ordered calendar subtraction to its target month"
        );
    }
    Ok(())
}

#[test]
fn weekdays_use_the_updated_calendar_and_keep_the_local_clock() -> TestResult {
    // Intentional difference: recompute the weekday after changing the month.
    // Git uses August 30's cached Sunday when selecting a Thursday in July.
    for (reference, input, expected) in [
        (
            "2009-08-30T19:20:00[UTC]",
            "1 month last Thursday",
            "2009-07-23 19:20:00 +0000",
        ),
        (
            "2024-03-11T01:30:00[America/New_York]",
            "last sunday",
            "2024-03-10 01:30:00 -0500",
        ),
        (
            "2024-11-03T01:30:00-05:00[America/New_York]",
            "last saturday",
            "2024-11-02 01:30:00 -0400",
        ),
    ] {
        assert_eq!(
            gix_date::parse(input, Some(reference.parse()?))?,
            gix_date::parse(expected, None)?,
            "{input}: select the requested local weekday, preserving its clock across DST"
        );
    }
    Ok(())
}

#[test]
fn zero_and_zero_padded_counts_are_numbers() -> TestResult {
    // Intentional difference: Git ignores 008 and does not recognize the word zero.
    // Decimal padding must not erase an eight-day subtraction; zero is the identity.
    let now = "2009-08-30T19:20:00[UTC]".parse::<jiff::Zoned>()?;
    for (input, seconds) in [
        ("008 days", 1250968800),
        ("0008 days", 1250968800),
        ("zero days ago", 1251660000),
    ] {
        assert_eq!(
            gix_date::parse(input, Some(now.clone()))?,
            Time::new(seconds, 0),
            "{input}: a decimal count has its numerical value, including zero"
        );
    }
    Ok(())
}

#[test]
fn clocks_are_not_erased_by_today_or_confused_with_fractional_counts() -> TestResult {
    // Intentional differences: Git compares a clock with now to decide whether today
    // erases it, and discards .3.days after a date-setting operation. Explicit clocks
    // must survive even when equal to now; .3 is a fraction unless followed by a unit.
    for (input, expected) in [
        ("07:20 today", "2009-08-30 07:20:00 +0000"),
        ("12:34:56.3", "2009-08-30 12:34:56 +0000"),
        ("now 12:34:56.3.days.ago", "2009-08-27 12:34:56 +0000"),
        ("never today", "1970-01-01 00:00:00 +0000"),
    ] {
        assert_eq!(
            gix_date::parse(input, Some("2009-08-30T07:20:00[UTC]".parse()?))?,
            gix_date::parse(expected, None)?,
            "{input}: apply explicit clock and count tokens in order"
        );
    }
    Ok(())
}

#[test]
fn invalid_calendar_dates_meridians_and_future_directions_are_rejected() -> TestResult {
    // Intentional difference: Git normalizes February 31, reduces 25pm modulo 12,
    // treats 0pm as a bare PM, and interprets "from now" as subtraction. Reject these
    // invalid or unsupported requests instead of silently returning another date.
    for input in [
        "Feb 31 2008 20:30:45 -0500",
        "Feb 29 2009",
        "0pm",
        "25pm",
        "24:01",
        "23:59:60",
        "Feb 14 2008 0:30 PM -0500",
        "Feb 14 2008 13:30 PM -0500",
        "1 hour from now",
        "1.hour.from.now",
        "1 hour FROM NOW",
    ] {
        for reference in [None, Some("2009-08-30T19:20:00[UTC]".parse()?)] {
            assert!(
                gix_date::parse(input, reference).is_err(),
                "{input}: do not guess a different instant"
            );
        }
    }
    Ok(())
}

#[test]
fn textual_and_numeric_years_share_a_complete_pivot_and_literal_years() -> TestResult {
    // Intentional difference: Git's numeric year ranges omit 38..70, while its
    // textual parser only accepts 00..09 and 70..99. Use the consistent 70 pivot,
    // and let Jiff validate literal four-digit years instead of limiting them to 1970..2099.
    for (year, full_year) in [
        ("38", "2038"),
        ("69", "2069"),
        ("70", "1970"),
        ("1950", "1950"),
        ("2100", "2100"),
    ] {
        let expected = gix_date::parse(&format!("{full_year}-02-14 20:30:45 +0000"), None)?;
        for input in [
            format!("2/14/{year} 20:30:45 +0000"),
            format!("Feb 14 {year} 20:30:45 +0000"),
        ] {
            assert_eq!(
                gix_date::parse(&input, None)?,
                expected,
                "{input}: no unexplained gap in supported years"
            );
        }
    }
    Ok(())
}

#[test]
fn adjacent_count_unit_pairs_are_complete_operations() -> TestResult {
    // Intentional difference: Git mistakes these numbers for calendar fields and
    // returns August 2. Subtract both complete pairs: August 28 at 16:20 UTC.
    for input in ["2days3hours", "2 days3 hours ago"] {
        assert_eq!(
            gix_date::parse(input, Some("2009-08-30T19:20:00[UTC]".parse()?))?,
            Time::new(1_251_476_400, 0),
            "{input}: adjacency does not erase either count/unit pair"
        );
    }
    Ok(())
}

#[test]
fn explicit_offsets_are_never_unset_sentinels() -> TestResult {
    // Intentional difference: Git treats -0001 as an absent timezone. Minus one
    // minute means 20:30:45 local is 20:31:45 UTC, independent of the reference zone.
    for reference in [None, Some("2009-08-30T19:20:00[America/New_York]".parse()?)] {
        assert_eq!(
            gix_date::parse("Feb 14 2008 20:30:45 -0001", reference)?,
            Time::new(1_203_021_105, -60),
            "an explicit offset always determines the instant"
        );
    }
    Ok(())
}

#[test]
fn signed_epochs_are_exact_and_reference_free() -> TestResult {
    // Intentional extension: negative epoch seconds are valid instants before 1970.
    // Git's input-date parser rejects them; no calendar guessing or reference is needed.
    for (input, offset) in [("@-1000", 0), ("-1000 +0100", 3600)] {
        assert_eq!(
            gix_date::parse(input, None)?,
            Time::new(-1000, offset),
            "{input}: signed epoch seconds remain exact"
        );
    }
    Ok(())
}
