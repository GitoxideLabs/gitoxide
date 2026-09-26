use crate::parse::Events;

#[test]
fn line_no_is_one_indexed() {
    let err = Events::from_str("[hello").expect_err("the section header is unterminated");
    assert_eq!(err.line_number(), 1);
    insta::assert_debug_snapshot!(format_args!("{err}"), "parser diagnostics use one-based line numbers", @"Got an unexpected token on line 1 while trying to parse a section header: '[hello'");
}

#[test]
fn malformed_input_retains_validation_and_bad_tokens() {
    use gix_error::ErrorExt;

    let err = Events::from_str("[hello")
        .expect_err("the section header is unterminated")
        .raise_typed();
    insta::assert_debug_snapshot!(err, "malformed configuration retains the offending input", @"Got an unexpected token on line 1 while trying to parse a section header: '[hello'");
    assert!(err.is_validation(), "malformed configuration is invalid input");
    assert!(
        err.probable_cause().is::<gix_config::parse::Error>(),
        "the parser error, not its classification marker, is the probable cause"
    );
    assert_eq!(err.remaining_data(), b"[hello", "the parser retains the unparsed input");
}

#[test]
fn to_string_truncates_extra_values() {
    let err = Events::from_str("[1234567890").expect_err("the section header is unterminated");
    insta::assert_debug_snapshot!(format_args!("{err}"), "long invalid tokens show ten characters and the omitted length", @"Got an unexpected token on line 1 while trying to parse a section header: '[123456789' ... (1 characters omitted)");
}

#[test]
fn to_string() {
    let err = Events::from_str("[a_b]\n c=d").expect_err("underscores are invalid in section names");
    insta::assert_debug_snapshot!(format_args!("{err}"), "underscores in section names are rejected by Git", @"
    Got an unexpected token on line 1 while trying to parse a section header: '[a_b]
     c=d'
    ");
    let err = Events::from_str("[core] a=b\\\n cd\n[core]\n\n 4a=3").expect_err("names cannot start with a digit");
    insta::assert_debug_snapshot!(format_args!("{err}"), "line numbers include continuations, section headers, and blank lines", @"Got an unexpected token on line 5 while trying to parse a name: '4a=3'");
    let err = Events::from_str("[core] a=b\\\n cd\n 4a=3").expect_err("names cannot start with a digit");
    insta::assert_debug_snapshot!(format_args!("{err}"), "line numbers include continued values", @"Got an unexpected token on line 3 while trying to parse a name: '4a=3'");
    let err = Events::from_str("[core] a=b\n 4a=3").expect_err("names cannot start with a digit");
    insta::assert_debug_snapshot!(format_args!("{err}"), "an invalid name is reported with its line and unparsed input", @"Got an unexpected token on line 2 while trying to parse a name: '4a=3'");
    let err = Events::from_str("[core] a=b\n =3").expect_err("names cannot be empty");
    insta::assert_debug_snapshot!(format_args!("{err}"), "a missing name reports the remaining assignment", @"Got an unexpected token on line 2 while trying to parse a name: '=3'");
    let err = Events::from_str("[core").expect_err("the section header is unterminated");
    insta::assert_debug_snapshot!(format_args!("{err}"), "an unterminated header identifies the section parser", @"Got an unexpected token on line 1 while trying to parse a section header: '[core'");
    let err = Events::from_str("[a]\n\tb \u{8}\n").expect_err("backspace is not trailing whitespace");
    insta::assert_debug_snapshot!(format_args!("{err}"), "Git rejects backspace as trailing whitespace after an implicit boolean", @"Got an unexpected token on line 2 while trying to parse a name: '\u{8}\n'");
}
