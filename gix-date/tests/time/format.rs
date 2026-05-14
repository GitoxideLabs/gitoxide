use gix_date::{
    Time,
    time::{Format, format},
};

#[test]
fn short() -> gix_testtools::Result {
    assert_eq!(time().format(format::SHORT)?, "1973-11-30");
    Ok(())
}

#[test]
fn unix() -> gix_testtools::Result {
    let expected = "123456789";
    assert_eq!(time().format(Format::Unix)?, expected);
    assert_eq!(time().format(format::UNIX)?, expected);
    Ok(())
}

#[test]
fn raw() -> gix_testtools::Result {
    for (time, expected) in [
        (time(), "123456789 +0230"),
        (
            Time {
                seconds: 1112911993,
                offset: 3600,
            },
            "1112911993 +0100",
        ),
    ] {
        assert_eq!(time.format(Format::Raw)?, expected);
        assert_eq!(time.format(format::RAW)?, expected);
    }
    Ok(())
}

#[test]
fn iso8601() -> gix_testtools::Result {
    assert_eq!(time().format(format::ISO8601)?, "1973-11-30 00:03:09 +0230");
    Ok(())
}

#[test]
fn iso8601_strict() -> gix_testtools::Result {
    assert_eq!(time().format(format::ISO8601_STRICT)?, "1973-11-30T00:03:09+02:30");
    Ok(())
}

#[test]
fn rfc2822() -> gix_testtools::Result {
    assert_eq!(time().format(format::RFC2822)?, "Fri, 30 Nov 1973 00:03:09 +0230");
    assert_eq!(time_dec1().format(format::RFC2822)?, "Sat, 01 Dec 1973 00:03:09 +0230");
    Ok(())
}

#[test]
fn git_rfc2822() -> gix_testtools::Result {
    assert_eq!(time().format(format::GIT_RFC2822)?, "Fri, 30 Nov 1973 00:03:09 +0230");
    assert_eq!(
        time_dec1().format(format::GIT_RFC2822)?,
        "Sat, 1 Dec 1973 00:03:09 +0230"
    );
    Ok(())
}

#[test]
fn default() -> gix_testtools::Result {
    assert_eq!(time().format(format::GITOXIDE)?, "Fri Nov 30 1973 00:03:09 +0230");
    assert_eq!(time_dec1().format(format::GITOXIDE)?, "Sat Dec 01 1973 00:03:09 +0230");
    Ok(())
}

#[test]
fn format_or_unix() {
    assert_eq!(
        Time {
            seconds: 42,
            offset: 7200 * 60
        }
        .format_or_unix(format::GITOXIDE),
        "42"
    );
}

#[test]
fn git_default() -> gix_testtools::Result {
    assert_eq!(
        time().format(gix_date::time::format::DEFAULT)?,
        "Fri Nov 30 00:03:09 1973 +0230"
    );
    assert_eq!(
        time_dec1().format(gix_date::time::format::DEFAULT)?,
        "Sat Dec 1 00:03:09 1973 +0230"
    );
    Ok(())
}

fn time() -> Time {
    Time {
        seconds: 123456789,
        offset: 9000,
    }
}

fn time_dec1() -> Time {
    Time {
        seconds: 123543189,
        offset: 9000,
    }
}
