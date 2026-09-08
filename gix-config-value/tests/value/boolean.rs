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
fn numbers_are_parsed_like_git_ints() {
    // Recorded from `git -c foo.bar=<input> config --type=bool foo.bar` on git 2.50.1.
    // `git_parse_maybe_bool_text()` hands the value to `git_parse_int()`, so the bases
    // and suffixes are those of an integer, bounded to a C `int`.
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
        ("2147483647", true),
        // `git` accepts this from 2.50, which changed the lower bound in
        // `git_parse_signed()` from `-max` to `-max - 1`; 2.43 refuses it. It parsed
        // here in decimal before this change, so it keeps parsing.
        ("-2147483648", true),
    ] {
        assert_eq!(
            Boolean::try_from(input).map(Into::into),
            Ok(expected),
            "{input:?}: `git` reads this as {expected}"
        );
    }
}

#[test]
fn numbers_outside_a_c_int_are_rejected_like_git() {
    // `git` refuses each of these as a boolean, even though they are valid integers,
    // because the numeric fallback is bounded to a C `int`.
    for input in ["2147483648", "-2147483649", "4294967296", "9223372036854775807", "2g"] {
        assert!(
            Boolean::try_from(input).is_err(),
            "{input:?}: out of range for a C `int`, which `git` refuses here"
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
