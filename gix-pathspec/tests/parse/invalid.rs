use crate::parse::check_against_baseline;

#[test]
fn empty_input() {
    let input = "";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let output = gix_pathspec::parse(input.as_bytes(), Default::default());
    assert!(output.is_err());
    assert_eq!(
        output.unwrap_err().to_string(),
        "An empty string is not a valid pathspec"
    );
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
        assert!(
            output
                .map_err(|err| err.to_string())
                .unwrap_err()
                .starts_with("Unimplemented short keyword:")
        );
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
        assert!(
            output
                .map_err(|err| err.to_string())
                .unwrap_err()
                .ends_with("in signature, which is not a valid keyword")
        );
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
        assert!(
            output
                .map_err(|err| err.to_string())
                .unwrap_err()
                .starts_with("Attribute has non-ascii characters or starts with '-'")
        );
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
            output
                .map_err(|err| err.to_string())
                .unwrap_err()
                .starts_with("Invalid character in attribute value:"),
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
        assert_eq!(
            output.unwrap_err().to_string(),
            r"Escape character '\' is not allowed as the last character in an attribute value"
        );
    }
}

#[test]
fn empty_attribute_specification() {
    let input = ":(attr:)";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let output = gix_pathspec::parse(input.as_bytes(), Default::default());
    assert!(output.is_err());
    assert_eq!(
        output.unwrap_err().to_string(),
        "Attribute specification cannot be empty"
    );
}

#[test]
fn multiple_attribute_specifications() {
    let input = ":(attr:one,attr:two)some/path";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let output = gix_pathspec::parse(input.as_bytes(), Default::default());
    assert!(output.is_err());
    assert_eq!(
        output.unwrap_err().to_string(),
        "Only one attribute specification is allowed in the same pathspec"
    );
}

#[test]
fn missing_parentheses() {
    let input = ":(top";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let output = gix_pathspec::parse(input.as_bytes(), Default::default());
    assert!(output.is_err());
    assert_eq!(
        output.unwrap_err().to_string(),
        "Missing ')' at the end of pathspec signature"
    );
}

#[test]
fn glob_and_literal_keywords_present() {
    let input = ":(glob,literal)some/path";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let output = gix_pathspec::parse(input.as_bytes(), Default::default());
    assert!(output.is_err());
    assert_eq!(
        output.unwrap_err().to_string(),
        "'literal' and 'glob' keywords cannot be used together in the same pathspec"
    );
}
