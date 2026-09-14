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
