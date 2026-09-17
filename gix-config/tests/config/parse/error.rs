use crate::parse::Events;

#[test]
fn line_no_is_one_indexed() {
    assert_eq!(Events::from_str("[hello").unwrap_err().line_number(), 1);
}

#[test]
fn conversion_retains_validation_and_bad_tokens() {
    use gix_error::ErrorExt;

    let err = Events::from_str("[hello")
        .expect_err("the section header is unterminated")
        .raise();
    assert!(err.is_validation(), "malformed configuration is invalid input");
    let err = err.into_error();
    assert!(err.is_validation(), "conversion preserves the classification");
    assert_eq!(
        err.downcast_any_ref::<gix_config::parse::Error>()
            .expect("retain the original parse error")
            .remaining_data(),
        b"[hello"
    );
}

#[test]
fn to_string_truncates_extra_values() {
    assert_eq!(
        Events::from_str("[1234567890").unwrap_err().to_string(),
        "Got an unexpected token on line 1 while trying to parse a section header: '[123456789' ... (1 characters omitted)"
    );
}

#[test]
fn to_string() {
    let input = "[a_b]\n c=d";
    assert_eq!(
        Events::from_str(input).unwrap_err().to_string(),
        "Got an unexpected token on line 1 while trying to parse a section header: '[a_b]\n c=d'",
        "underscores in section names aren't allowed and will be rejected by git"
    );
    let input = "[core] a=b\\\n cd\n[core]\n\n 4a=3";
    assert_eq!(
        Events::from_str(input).unwrap_err().to_string(),
        "Got an unexpected token on line 5 while trying to parse a name: '4a=3'"
    );
    let input = "[core] a=b\\\n cd\n 4a=3";
    assert_eq!(
        Events::from_str(input).unwrap_err().to_string(),
        "Got an unexpected token on line 3 while trying to parse a name: '4a=3'"
    );
    let input = "[core] a=b\n 4a=3";
    assert_eq!(
        Events::from_str(input).unwrap_err().to_string(),
        "Got an unexpected token on line 2 while trying to parse a name: '4a=3'"
    );
    let input = "[core] a=b\n =3";
    assert_eq!(
        Events::from_str(input).unwrap_err().to_string(),
        "Got an unexpected token on line 2 while trying to parse a name: '=3'"
    );
    let input = "[core";
    assert_eq!(
        Events::from_str(input).unwrap_err().to_string(),
        "Got an unexpected token on line 1 while trying to parse a section header: '[core'"
    );
    let input = "[a]\n\tb \u{8}\n";
    assert_eq!(
        Events::from_str(input).unwrap_err().to_string(),
        "Got an unexpected token on line 2 while trying to parse a name: '\u{8}\n'",
        "Git rejects backspace as trailing whitespace after an implicit boolean"
    );
}
