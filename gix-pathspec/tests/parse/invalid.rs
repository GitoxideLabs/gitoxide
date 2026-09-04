use gix_error::ValidationError;

use crate::parse::check_against_baseline;

fn assert_validation(input: &str, message: &str) -> ValidationError {
    let err = gix_pathspec::parse(input.as_bytes(), Default::default()).expect_err("pathspec is invalid");
    assert_eq!(err.message, message);
    err
}

#[test]
fn empty_input() {
    let input = "";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let err = assert_validation(input, "An empty string is not a valid pathspec");
    assert_eq!(err.input.as_ref().map(|input| input.as_slice()), Some(b"".as_slice()));
}

#[test]
fn invalid_short_signatures() {
    let inputs = vec![
        ":\"()", ":#()", ":%()", ":&()", ":'()", ":,()", ":-()", ":;()", ":<()", ":=()", ":>()", ":@()", ":_()",
        ":`()", ":~()",
    ];

    for input in inputs.into_iter() {
        assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

        let err = assert_validation(input, "Unimplemented short keyword");
        assert_eq!(err.input.as_ref().map(|input| input.len()), Some(1));
    }
}

#[test]
fn invalid_keywords() {
    let inputs = vec![
        ":( )some/path",
        ":(tp)some/path",
        ":(top, exclude)some/path",
        ":(top,exclude,icse)some/path",
    ];

    for input in inputs.into_iter() {
        assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

        let err = assert_validation(input, "Found invalid keyword in pathspec signature");
        assert!(err.input.is_some(), "the invalid keyword is retained");
    }
}

#[test]
fn invalid_attributes() {
    let inputs = vec![
        ":(attr:+invalidAttr)some/path",
        ":(attr:validAttr +invalidAttr)some/path",
        ":(attr:+invalidAttr,attr:valid)some/path",
        r":(attr:inva\lid)some/path",
        ":(attr:a\tb)some/path",
        ":(attr:a\rb)some/path",
        ":(attr:!a=b)some/path",
        ":(attr:-a=b)some/path",
    ];

    for input in inputs {
        assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

        let err = assert_validation(input, "Attribute has non-ascii characters or starts with '-'");
        assert!(err.input.is_some(), "the invalid attribute name is retained");
    }
}

#[test]
fn attribute_values_are_not_split_on_non_space_blanks() {
    let input = ":(attr:a=one\tb=two)some/path";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");
    let err = assert_validation(input, "Invalid character in attribute value");
    assert_eq!(err.input.as_ref().map(|input| input.as_slice()), Some(b"\t".as_slice()));
}

#[test]
fn invalid_attribute_values() {
    let inputs = vec![
        r":(attr:v=inva#lid)some/path",
        r":(attr:v=inva\\lid)some/path",
        r":(attr:v=invalid\\)some/path",
        r":(attr:v=invalid\#)some/path",
        r":(attr:v=inva\=lid)some/path",
        r":(attr:a=valid b=inva\#lid)some/path",
        ":(attr:v=val��)",
        ":(attr:pr=pre��x:,)�",
    ];

    for input in inputs {
        assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

        let err = assert_validation(input, "Invalid character in attribute value");
        assert_eq!(err.input.as_ref().map(|input| input.len()), Some(1));
    }
}

#[test]
fn escape_character_at_end_of_attribute_value() {
    let inputs = vec![
        r":(attr:v=invalid\)some/path",
        r":(attr:v=invalid\ )some/path",
        r":(attr:v=invalid\ valid)some/path",
    ];

    for input in inputs {
        assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

        let err = assert_validation(
            input,
            r"Escape character '\' is not allowed as the last character in an attribute value",
        );
        assert!(err.input.is_some(), "the invalid attribute value is retained");
    }
}

#[test]
fn empty_attribute_specification() {
    let input = ":(attr:)";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    assert_validation(input, "Attribute specification cannot be empty");
}

#[test]
fn multiple_attribute_specifications() {
    let input = ":(attr:one,attr:two)some/path";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let err = assert_validation(
        input,
        "Only one attribute specification is allowed in the same pathspec",
    );
    assert!(err.input.is_some(), "the duplicate attribute specification is retained");
}

#[test]
fn missing_parentheses() {
    let input = ":(top";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let err = assert_validation(input, "Missing ')' at the end of pathspec signature");
    assert_eq!(err.input.as_ref().map(|input| input.as_slice()), Some(input.as_bytes()));
}

#[test]
fn glob_and_literal_keywords_present() {
    let input = ":(glob,literal)some/path";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let err = assert_validation(
        input,
        "'literal' and 'glob' keywords cannot be used together in the same pathspec",
    );
    assert_eq!(
        err.input.as_ref().map(|input| input.as_slice()),
        Some(b"literal".as_slice())
    );
}
