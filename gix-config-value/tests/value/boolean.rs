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
    assert!(Boolean::try_from("yes")?.0);
    assert!(Boolean::try_from("on")?.0);
    assert!(Boolean::try_from("true")?.0);
    assert!(Boolean::try_from("1")?.0);
    assert!(Boolean::try_from("+10")?.0);
    assert!(Boolean::try_from("-1")?.0);
    Ok(())
}

#[test]
fn ignores_case() {
    // Random subset
    for word in &["no", "yes", "on", "off", "true", "false"] {
        let first: bool = Boolean::try_from(*word).expect("valid boolean").into();
        let second: bool = Boolean::try_from(word.to_uppercase().as_str())
            .expect("valid boolean")
            .into();
        assert_eq!(first, second);
    }
}

#[test]
fn from_str_err() {
    assert!(Boolean::try_from("yesn't").is_err());
    assert!(Boolean::try_from("yesno").is_err());
}
