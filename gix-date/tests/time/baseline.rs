use std::time::{Duration, SystemTime};

use gix_date::{
    SecondsSinceUnixEpoch,
    time::{Format, format},
};
use gix_testtools::Result;
use std::sync::LazyLock;

struct Sample {
    format_name: Option<String>,
    exit_code: usize,
    seconds: SecondsSinceUnixEpoch,
    now: Option<SystemTime>,
}

static BASELINE: LazyLock<Vec<(String, Sample)>> = LazyLock::new(|| {
    (|| -> Result<_> {
        let base = gix_testtools::scripted_fixture_read_only("generate_git_date_baseline.sh")?;
        let mut samples = Vec::new();
        let file = std::fs::read(base.join("baseline.git"))?;
        let baseline = std::str::from_utf8(&file).expect("valid utf");
        let mut lines = baseline.lines();
        while let Some(date_str) = lines.next() {
            let format_name = lines.next().expect("five lines per baseline").to_string();
            let exit_code = lines.next().expect("five lines per baseline").parse()?;
            let seconds: SecondsSinceUnixEpoch = lines
                .next()
                .expect("five lines per baseline")
                .parse()
                .expect("valid epoch value");
            let now = match lines.next().expect("five lines per baseline") {
                "" => None,
                seconds => Some(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds.parse()?)),
            };
            samples.push((
                date_str.into(),
                Sample {
                    format_name: (!format_name.is_empty()).then_some(format_name),
                    exit_code,
                    seconds,
                    now,
                },
            ));
        }
        Ok(samples)
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
            now,
        },
    ) in BASELINE.iter()
    {
        let res = gix_date::parse(pattern.as_str(), *now);
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
