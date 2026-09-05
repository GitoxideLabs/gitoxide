use gix_config_value::{Integer, integer::Suffix};

#[test]
fn from_utf8_str() -> crate::Result {
    assert_eq!(
        Integer::try_from("1k")?,
        Integer {
            value: 1,
            suffix: Some(Suffix::Kibi),
        },
        "UTF-8 strings use the same integer parser as byte strings"
    );
    Ok(())
}

#[test]
fn from_str_no_suffix() {
    assert_eq!(Integer::try_from("1").unwrap(), Integer { value: 1, suffix: None });

    assert_eq!(
        Integer::try_from("-1").unwrap(),
        Integer {
            value: -1,
            suffix: None
        }
    );
}

#[test]
fn from_str_with_suffix() {
    assert_eq!(
        Integer::try_from("1k").unwrap(),
        Integer {
            value: 1,
            suffix: Some(Suffix::Kibi),
        }
    );

    assert_eq!(
        Integer::try_from("1m").unwrap(),
        Integer {
            value: 1,
            suffix: Some(Suffix::Mebi),
        }
    );

    assert_eq!(
        Integer::try_from("1g").unwrap(),
        Integer {
            value: 1,
            suffix: Some(Suffix::Gibi),
        }
    );
}

#[test]
fn invalid_from_str() {
    assert!(Integer::try_from("").is_err());
    assert!(Integer::try_from("-").is_err());
    assert!(Integer::try_from("k").is_err());
    assert!(Integer::try_from("m").is_err());
    assert!(Integer::try_from("g").is_err());
    assert!(Integer::try_from("123123123123123123123123").is_err());
    assert!(Integer::try_from("gg").is_err());
    assert!(Integer::try_from("™️🤦‍♂️").is_err());
}

#[test]
fn as_decimal() {
    fn decimal(input: &str) -> Option<i64> {
        Integer::try_from(input).unwrap().to_decimal()
    }

    assert_eq!(decimal("12"), Some(12), "works without suffix");
    assert_eq!(decimal("13k"), Some(13 * 1024), "works with kilobyte suffix");
    assert_eq!(decimal("13K"), Some(13 * 1024), "works with Kilobyte suffix");
    assert_eq!(decimal("14m"), Some(14 * 1_048_576), "works with megabyte suffix");
    assert_eq!(decimal("14M"), Some(14 * 1_048_576), "works with Megabyte suffix");
    assert_eq!(decimal("15g"), Some(15 * 1_073_741_824), "works with gigabyte suffix");
    assert_eq!(decimal("15G"), Some(15 * 1_073_741_824), "works with Gigabyte suffix");

    assert_eq!(decimal(&format!("{}g", i64::MAX)), None, "overflow results in None");
    assert_eq!(decimal(&format!("{}g", i64::MIN)), None, "underflow results in None");
}

/// git hands config integers to `strtoimax()` with a base of `0`, so a `0x` prefix is
/// hexadecimal, a `0b` prefix is binary, and a leading `0` is octal.
#[test]
fn bases_match_git() {
    fn decimal(input: &str) -> Option<i64> {
        Integer::try_from(input).ok().and_then(|int| int.to_decimal())
    }

    assert_eq!(decimal("0x10"), Some(16), "0x is hexadecimal");
    assert_eq!(decimal("0X1F"), Some(31), "the prefix is case insensitive");
    assert_eq!(decimal("+0x10"), Some(16), "a positive sign precedes the prefix");
    assert_eq!(decimal("-0x10"), Some(-16), "a sign precedes the prefix");
    assert_eq!(decimal("0b101"), Some(5), "0b is binary");
    assert_eq!(decimal("0B101"), Some(5), "the prefix is case insensitive");
    assert_eq!(decimal("+0b101"), Some(5), "binary values may have a positive sign");
    assert_eq!(decimal("-0b101"), Some(-5), "binary values may have a negative sign");
    assert_eq!(decimal("010"), Some(8), "a leading zero is octal, not decimal");
    assert_eq!(decimal("+010"), Some(8), "octal values may have a positive sign");
    assert_eq!(decimal("-010"), Some(-8), "octal values may have a negative sign");
    assert_eq!(decimal("00"), Some(0), "a second zero is an octal digit");
    assert_eq!(decimal("0"), Some(0), "a lone zero stays decimal");

    assert_eq!(
        decimal("0x10k"),
        Some(16 * 1024),
        "a suffix applies to a hexadecimal value…"
    );
    assert_eq!(decimal("0x10K"), Some(16 * 1024), "…in either case");
    assert_eq!(decimal("0b101k"), Some(5 * 1024), "…and to a binary one");
    assert_eq!(decimal("010k"), Some(8 * 1024), "…and to an octal one");

    assert_eq!(
        decimal("0x7fffffffffffffff"),
        Some(i64::MAX),
        "the whole range is available in hexadecimal"
    );
    assert!(
        Integer::try_from("0x8000000000000000").is_err(),
        "one above i64::MAX is rejected"
    );
    // `git_parse_signed()` bounds values at `-max - 1` since git 2.50; up to 2.49 the
    // bound was `-max`, which rejected this value. `main` already accepted it in
    // decimal form, so only the hexadecimal spelling is new here.
    assert_eq!(
        decimal("-0x8000000000000000"),
        Some(i64::MIN),
        "including the value whose magnitude Git before 2.50 rejected"
    );
    assert!(
        Integer::try_from("-0x8000000000000001").is_err(),
        "one below i64::MIN is rejected"
    );

    for invalid in ["08", "09", "0x", "0xg", "0b", "0b2", "0o17"] {
        assert!(
            Integer::try_from(invalid).is_err(),
            "`{invalid}` is rejected by git too"
        );
    }

    for prefix in ["0", "0x", "0X", "0b", "0B"] {
        for outer_sign in ["", "+", "-"] {
            for inner_sign in ["+", "-"] {
                for suffix in ["", "k"] {
                    let invalid = format!("{outer_sign}{prefix}{inner_sign}1{suffix}");
                    assert!(
                        Integer::try_from(invalid.as_str()).is_err(),
                        "`{invalid}` is rejected because a sign is only valid before the prefix"
                    );
                }
            }
        }
    }
}
