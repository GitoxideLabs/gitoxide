use std::{collections::HashMap, time::SystemTime};

use gix_date::{
    SecondsSinceUnixEpoch,
    time::{Format, format},
};
use gix_testtools::Result;
use std::sync::LazyLock;
use std::time::Duration;

struct Sample {
    format_name: Option<String>,
    exit_code: usize,
    seconds: SecondsSinceUnixEpoch,
}

/// Returns true if the pattern is one of the relative dates recorded by `baseline_relative()`,
/// which are the entries whose expected value depends on `GIT_TEST_DATE_NOW`.
///
/// `ago` is matched anywhere rather than as a suffix, because a separator other than a space is
/// just as good to Git: `1.hour.ago` is a relative date too.
fn is_relative_date(pattern: &str) -> bool {
    let pattern = pattern.trim().to_ascii_lowercase();
    pattern.contains("ago") || matches!(pattern.as_str(), "now" | "today" | "yesterday" | "last week")
}

/// The fixed "now" timestamp used for testing relative dates.
/// This matches GIT_TEST_DATE_NOW=1000000000 in the baseline script.
/// This is Sun Sep 9 01:46:40 UTC 2001.
fn fixed_now() -> SystemTime {
    std::time::UNIX_EPOCH + Duration::from_secs(1_000_000_000)
}

static BASELINE: LazyLock<HashMap<String, Sample>> = LazyLock::new(|| {
    (|| -> Result<_> {
        let base = gix_testtools::scripted_fixture_read_only("generate_git_date_baseline.sh")?;
        let mut map = HashMap::new();
        let file = std::fs::read(base.join("baseline.git"))?;
        let baseline = std::str::from_utf8(&file).expect("valid utf");
        let mut lines = baseline.lines();
        while let Some(date_str) = lines.next() {
            let format_name = lines.next().expect("four lines per baseline").to_string();
            let exit_code = lines.next().expect("four lines per baseline").parse()?;
            let seconds: SecondsSinceUnixEpoch = lines
                .next()
                .expect("four lines per baseline")
                .parse()
                .expect("valid epoch value");
            map.insert(
                date_str.into(),
                Sample {
                    format_name: (!format_name.is_empty()).then_some(format_name),
                    exit_code,
                    seconds,
                },
            );
        }
        Ok(map)
    })()
    .expect("baseline format is well known and can always be parsed")
});

#[test]
fn parse_compare_format() {
    for (
        pattern,
        Sample {
            format_name,
            exit_code,
            seconds: time_in_seconds_since_unix_epoch,
        },
    ) in BASELINE.iter()
    {
        let now = is_relative_date(pattern).then(fixed_now);
        let res = gix_date::parse(pattern.as_str(), now);
        assert_eq!(
            res.is_ok(),
            *exit_code == 0,
            "{pattern:?} disagrees with baseline: {res:?}"
        );
        if let Ok(t) = res {
            let actual = t.seconds;
            assert_eq!(
                actual, *time_in_seconds_since_unix_epoch,
                "{pattern:?} disagrees with baseline seconds since epoch: {actual:?}"
            );
            if let Some(format_name) = format_name {
                let reformatted = t
                    .format(match format_name.as_str() {
                        "SHORT" => Format::Custom(format::SHORT),
                        "RFC2822" => Format::Custom(format::RFC2822),
                        "ISO8601" => Format::Custom(format::ISO8601),
                        "ISO8601_STRICT" => Format::Custom(format::ISO8601_STRICT),
                        "GITOXIDE" => Format::Custom(format::GITOXIDE),
                        "UNIX" => Format::Unix,
                        "RAW" => Format::Raw,
                        unknown => unreachable!("All formats should be well-known and implemented: {unknown:?}"),
                    })
                    .expect("valid input time");
                assert_eq!(
                    reformatted, *pattern,
                    "{reformatted:?} disagrees with baseline pattern: {pattern:?}"
                );
            }
        }
    }
}
