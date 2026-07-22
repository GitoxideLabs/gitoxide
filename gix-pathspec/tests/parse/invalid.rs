use gix_pathspec::parse::Error;

use crate::parse::check_against_baseline;

#[test]
fn empty_input() {
    let input = "";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let output = gix_pathspec::parse(input.as_bytes(), Default::default());
    assert!(output.is_err());
    assert!(matches!(output.unwrap_err(), Error::EmptyString));
}

#[test]
fn invalid_short_signatures() {
    let inputs = vec![
        ":\"()", ":#()", ":%()", ":&()", ":'()", ":,()", ":-()", ":;()", ":<()", ":=()", ":>()", ":@()", ":_()",
        ":`()", ":~()",
    ];

    for input in inputs.into_iter() {
        assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

        let output = gix_pathspec::parse(input.as_bytes(), Default::default());
        assert!(output.is_err());
        assert!(matches!(output.unwrap_err(), Error::Unimplemented { .. }));
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

        let output = gix_pathspec::parse(input.as_bytes(), Default::default());
        assert!(output.is_err());
        assert!(matches!(output.unwrap_err(), Error::InvalidKeyword { .. }));
    }
}

#[test]
fn invalid_attributes() {
    let inputs = vec![
        ":(attr:+invalidAttr)some/path",
        ":(attr:validAttr +invalidAttr)some/path",
        ":(attr:+invalidAttr,attr:valid)some/path",
        r":(attr:inva\lid)some/path",
    ];

    for input in inputs {
        assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

        let output = gix_pathspec::parse(input.as_bytes(), Default::default());
        assert!(output.is_err(), "This pathspec did not produce an error {input}");
        assert!(matches!(output.unwrap_err(), Error::InvalidAttribute { .. }));
    }
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

        let output = gix_pathspec::parse(input.as_bytes(), Default::default());
        assert!(output.is_err(), "This pathspec did not produce an error {input}");
        assert!(
            matches!(output.unwrap_err(), Error::InvalidAttributeValue { .. }),
            "Errors did not match for pathspec: {input}"
        );
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

        let output = gix_pathspec::parse(input.as_bytes(), Default::default());
        assert!(output.is_err(), "This pathspec did not produce an error {input}");
        assert!(matches!(output.unwrap_err(), Error::TrailingEscapeCharacter));
    }
}

#[test]
fn empty_attribute_specification() {
    let input = ":(attr:)";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let output = gix_pathspec::parse(input.as_bytes(), Default::default());
    assert!(output.is_err());
    assert!(matches!(output.unwrap_err(), Error::EmptyAttribute));
}

#[test]
fn multiple_attribute_specifications() {
    let input = ":(attr:one,attr:two)some/path";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let output = gix_pathspec::parse(input.as_bytes(), Default::default());
    assert!(output.is_err());
    assert!(matches!(output.unwrap_err(), Error::MultipleAttributeSpecifications));
}

#[test]
fn missing_parentheses() {
    let input = ":(top";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let output = gix_pathspec::parse(input.as_bytes(), Default::default());
    assert!(output.is_err());
    assert!(matches!(output.unwrap_err(), Error::MissingClosingParenthesis));
}

#[test]
fn glob_and_literal_keywords_present() {
    let input = ":(glob,literal)some/path";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let output = gix_pathspec::parse(input.as_bytes(), Default::default());
    assert!(output.is_err());
    assert!(matches!(output.unwrap_err(), Error::IncompatibleSearchModes));
}
