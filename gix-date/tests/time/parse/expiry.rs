use gix_date::parse_expiry;
use gix_testtools::TestResult;

#[test]
fn special_values_do_not_need_a_clock() -> TestResult {
    for (input, expected) in [("never", 0), ("false", 0), ("all", u64::MAX), ("now", u64::MAX)] {
        assert_eq!(parse_expiry(input, None)?, expected, "{input:?} is an expiry sentinel");
    }
    Ok(())
}

#[test]
fn pre_epoch_relative_dates_wrap() -> TestResult {
    let now = jiff::Timestamp::from_second(1)?.to_zoned(jiff::tz::TimeZone::UTC);
    for (input, seconds) in [("2 seconds ago", -1_i64), ("1 year ago", 1 - 365 * 24 * 60 * 60)] {
        assert_eq!(
            parse_expiry(input, Some(now.clone()))?,
            seconds as u64,
            "{input:?} wraps into the unsigned timestamp range"
        );
    }
    Ok(())
}

#[test]
fn expiry_values_match_git() -> TestResult {
    let dir = gix_testtools::tempfile::tempdir()?;
    for (input, seconds) in [
        ("never", 1_000_000_000),
        ("false", 1_000_000_000),
        ("all", 1_000_000_000),
        ("now", 1_000_000_000),
        ("NOW", 1_000_000_000),
        (" now ", 1_000_000_000),
        ("2.weeks.ago", 1_000_000_000),
        ("2020-01-02 03:04:05 +0000", 1_000_000_000),
        ("1660874655 +0800", 1_000_000_000),
        ("2 seconds ago", 1),
        // Git for Windows' mktime() rejects calendar dates before the epoch.
        ("1 year ago", 1_000_000_000),
    ] {
        let git = gix_testtools::git_command(dir.path())
            .args([
                "config",
                "--file",
                "missing",
                "--type=expiry-date",
                "--default",
                input,
                "--get",
                "test.expiry",
            ])
            .env("GIT_TEST_DATE_NOW", seconds.to_string())
            .env("TZ", "UTC")
            .output()?;
        assert!(
            git.status.success(),
            "Git accepts {input:?}: {}",
            String::from_utf8_lossy(&git.stderr)
        );
        let expected: u64 = std::str::from_utf8(&git.stdout)?.trim().parse()?;
        let now = jiff::Timestamp::from_second(seconds)?.to_zoned(jiff::tz::TimeZone::UTC);
        assert_eq!(
            parse_expiry(input, Some(now))?,
            expected,
            "{input:?} agrees with Git at {seconds}"
        );
    }
    Ok(())
}

#[test]
fn invalid_expiry_values_retain_their_input() -> TestResult {
    let dir = gix_testtools::tempfile::tempdir()?;
    for input in ["", "x", "false ", "FALSE", "all ", "ALL"] {
        let git = gix_testtools::git_command(dir.path())
            .args([
                "config",
                "--file",
                "missing",
                "--type=expiry-date",
                "--default",
                input,
                "--get",
                "test.expiry",
            ])
            .output()?;
        assert!(!git.status.success(), "Git rejects {input:?}");
        let err = parse_expiry(input, None).expect_err("invalid expiry dates fail");
        assert!(err.is_validation(), "invalid expiry dates are validation errors");
        assert_eq!(
            err.metadata().find_map(|metadata| metadata.get("input")),
            Some(&gix_error::MetadataValue::Bytes(input.as_bytes().into())),
            "the invalid expiry input remains available"
        );
    }
    Ok(())
}
