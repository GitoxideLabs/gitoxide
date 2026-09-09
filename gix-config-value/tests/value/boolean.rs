use gix_config_value::Boolean;

#[test]
fn from_utf8_str() -> crate::Result {
    assert_eq!(
        Boolean::try_from("yes")?,
        Boolean(true),
        "UTF-8 strings use the same boolean parser as byte strings"
    );
    Ok(())
}

#[test]
fn from_str_false() -> crate::Result {
    assert!(!Boolean::try_from("no")?.0);
    assert!(!Boolean::try_from("off")?.0);
    assert!(!Boolean::try_from("false")?.0);
    assert!(!Boolean::try_from("0")?.0);
    assert!(!Boolean::try_from("")?.0);
    Ok(())
}

#[test]
fn from_str_true() -> crate::Result {
    assert_eq!(Boolean::try_from("yes").map(Into::into), Ok(true));
    assert_eq!(Boolean::try_from("on"), Ok(Boolean(true)));
    assert_eq!(Boolean::try_from("true"), Ok(Boolean(true)));
    assert!(Boolean::try_from("1")?.0);
    assert!(Boolean::try_from("+10")?.0);
    assert!(Boolean::try_from("-1")?.0);
    Ok(())
}

#[test]
fn ignores_case() {
    // Random subset
    for word in &["no", "yes", "on", "off", "true", "false"] {
        let first: bool = Boolean::try_from(*word).unwrap().into();
        let second: bool = Boolean::try_from(word.to_uppercase().as_str()).unwrap().into();
        assert_eq!(first, second);
    }
}

#[test]
fn numbers_are_parsed_as_integers() {
    // Use the same bases, suffixes, and full `i64` range as `Integer`.
    for (input, expected) in [
        ("0x10", true),
        ("0X1F", true),
        ("-0x10", true),
        ("0x0", false),
        ("010", true),
        ("017", true),
        ("1k", true),
        ("2m", true),
        ("0k", false),
        ("-0", false),
        ("2147483647", true),  // i32::MAX
        ("-2147483648", true), // i32::MIN
        ("2147483648", true),
        ("-2147483649", true),
        ("4294967296", true),
        ("2g", true),
        ("9223372036854775807", true),  // i64::MAX
        ("-9223372036854775808", true), // i64::MIN
        ("0x7fffffffffffffff", true),
        ("-0x8000000000000000", true),
        ("-8589934592g", true), // i64::MIN after applying the suffix
    ] {
        assert_eq!(
            Boolean::try_from(input).map(Into::into),
            Ok(expected),
            "{input:?}: zero integers are false and nonzero integers are true"
        );
    }
}

#[test]
fn numbers_outside_i64_are_rejected() {
    for input in [
        "9223372036854775808",
        "-9223372036854775809",
        "0x8000000000000000",
        "-0x8000000000000001",
        "8589934592g",
        "-8589934593g",
    ] {
        assert!(
            Boolean::try_from(input).is_err(),
            "{input:?}: integers must fit in `i64` after applying any suffix"
        );
    }
}

#[test]
fn from_str_err() {
    assert!(Boolean::try_from("yesn't").is_err());
    assert!(Boolean::try_from("yesno").is_err());
    assert!(
        Boolean::try_from("08").is_err(),
        "`08` is not a valid octal number, and `git` refuses it too"
    );
}
