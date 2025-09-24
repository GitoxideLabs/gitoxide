use gix_date::{
    time::{format, Format},
    Time,
};

#[test]
fn short() {
    assert_eq!(time().format(format::SHORT), "1973-11-30");
}

#[test]
fn unix() {
    let expected = "123456789";
    assert_eq!(time().format(Format::Unix), expected);
    assert_eq!(time().format(format::UNIX), expected);
}

#[test]
fn raw() {
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
        assert_eq!(time.format(Format::Raw), expected);
        assert_eq!(time.format(format::RAW), expected);
    }
}

#[test]
fn iso8601() {
    assert_eq!(time().format(format::ISO8601), "1973-11-30 00:03:09 +0230");
}

#[test]
fn iso8601_strict() {
    assert_eq!(time().format(format::ISO8601_STRICT), "1973-11-30T00:03:09+02:30");
}

#[test]
fn rfc2822() {
    assert_eq!(time().format(format::RFC2822), "Fri, 30 Nov 1973 00:03:09 +0230");
    assert_eq!(time_dec1().format(format::RFC2822), "Sat, 01 Dec 1973 00:03:09 +0230");
}

#[test]
fn git_rfc2822() {
    assert_eq!(time().format(format::GIT_RFC2822), "Fri, 30 Nov 1973 00:03:09 +0230");
    assert_eq!(
        time_dec1().format(format::GIT_RFC2822),
        "Sat, 1 Dec 1973 00:03:09 +0230"
    );
}

#[test]
fn default() {
    assert_eq!(
        time().format(gix_date::time::format::GITOXIDE),
        "Fri Nov 30 1973 00:03:09 +0230"
    );
    assert_eq!(
        time_dec1().format(gix_date::time::format::GITOXIDE),
        "Sat Dec 01 1973 00:03:09 +0230"
    );
}

#[test]
fn git_default() {
    assert_eq!(
        time().format(gix_date::time::format::DEFAULT),
        "Fri Nov 30 00:03:09 1973 +0230"
    );
    assert_eq!(
        time_dec1().format(gix_date::time::format::DEFAULT),
        "Sat Dec 1 00:03:09 1973 +0230"
    );
}

#[test]
fn bad_offset() {
    assert_eq!(
        time_bad_offset().format(gix_date::time::format::DEFAULT),
        "Wed Jan 24 22:36:22 2018 -3407"
    );
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

fn time_bad_offset() -> Time {
    Time {
        seconds: 1516956202,
        offset: -122820,
    }
}
