use gix_error::Message;

use crate::parse::check_against_baseline;

fn assert_validation(input: &str) -> Message {
    let err = gix_pathspec::parse(input.as_bytes(), Default::default()).expect_err("pathspec is invalid");
    assert!(err.is_validation(), "invalid pathspecs retain their classification");
    err.into_inner()
}

#[test]
fn empty_input() {
    let input = "";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let err = assert_validation(input);
    insta::assert_debug_snapshot!(err, "empty input", @r#"
    Message {
        message: "An empty string is not a valid pathspec",
        class: Validation,
        values: {"input": Bytes("")},
    }
    "#);
    assert_eq!(
        err.values.get("input"),
        Some(&gix_error::MetadataValue::from(b"".as_slice()))
    );
}

#[test]
fn invalid_short_signatures() {
    let mut diagnostics = Vec::new();
    let inputs = vec![
        ":\"()", ":#()", ":%()", ":&()", ":'()", ":,()", ":-()", ":;()", ":<()", ":=()", ":>()", ":@()", ":_()",
        ":`()", ":~()",
    ];

    for input in inputs.into_iter() {
        assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

        let err = assert_validation(input);
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert!(matches!(err.values.get("input"), Some(gix_error::MetadataValue::Bytes(input)) if input.len() == 1));
    }
    insta::assert_debug_snapshot!(diagnostics, "invalid short signatures", @r##"
    [
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes("\"")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes("#")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes("%")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes("&")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes("\'")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes(",")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes("-")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes(";")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes("<")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes("=")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes(">")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes("@")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes("_")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes("`")},
        },
        Message {
            message: "Unimplemented short keyword",
            class: Validation,
            values: {"input": Bytes("~")},
        },
    ]
    "##);
}

#[test]
fn invalid_keywords() {
    let mut diagnostics = Vec::new();
    let inputs = vec![
        ":( )some/path",
        ":(tp)some/path",
        ":(top, exclude)some/path",
        ":(top,exclude,icse)some/path",
    ];

    for input in inputs.into_iter() {
        assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

        let err = assert_validation(input);
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert!(err.values.contains_key("input"), "the invalid keyword is retained");
    }
    insta::assert_debug_snapshot!(diagnostics, "invalid keywords", @r#"
    [
        Message {
            message: "Found invalid keyword in pathspec signature",
            class: Validation,
            values: {"input": Bytes(" ")},
        },
        Message {
            message: "Found invalid keyword in pathspec signature",
            class: Validation,
            values: {"input": Bytes("tp")},
        },
        Message {
            message: "Found invalid keyword in pathspec signature",
            class: Validation,
            values: {"input": Bytes(" exclude")},
        },
        Message {
            message: "Found invalid keyword in pathspec signature",
            class: Validation,
            values: {"input": Bytes("icse")},
        },
    ]
    "#);
}

#[test]
fn invalid_attributes() {
    let mut diagnostics = Vec::new();
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

        let err = assert_validation(input);
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert!(
            err.values.contains_key("input"),
            "the invalid attribute name is retained"
        );
    }
    insta::assert_debug_snapshot!(diagnostics, "invalid attributes", @r#"
    [
        Message {
            message: "Attribute has non-ascii characters or starts with '-'",
            class: Validation,
            values: {"input": Bytes("+invalidAttr")},
        },
        Message {
            message: "Attribute has non-ascii characters or starts with '-'",
            class: Validation,
            values: {"input": Bytes("+invalidAttr")},
        },
        Message {
            message: "Attribute has non-ascii characters or starts with '-'",
            class: Validation,
            values: {"input": Bytes("+invalidAttr")},
        },
        Message {
            message: "Attribute has non-ascii characters or starts with '-'",
            class: Validation,
            values: {"input": Bytes("inva\\lid")},
        },
        Message {
            message: "Attribute has non-ascii characters or starts with '-'",
            class: Validation,
            values: {"input": Bytes("a\tb")},
        },
        Message {
            message: "Attribute has non-ascii characters or starts with '-'",
            class: Validation,
            values: {"input": Bytes("a\rb")},
        },
        Message {
            message: "Attribute has non-ascii characters or starts with '-'",
            class: Validation,
            values: {"input": Bytes("a=b")},
        },
        Message {
            message: "Attribute has non-ascii characters or starts with '-'",
            class: Validation,
            values: {"input": Bytes("a=b")},
        },
    ]
    "#);
}

#[test]
fn attribute_values_are_not_split_on_non_space_blanks() {
    let input = ":(attr:a=one\tb=two)some/path";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");
    let err = assert_validation(input);
    insta::assert_debug_snapshot!(err, "attribute values are not split on non space blanks", @r#"
    Message {
        message: "Invalid character in attribute value",
        class: Validation,
        values: {"input": Bytes("\t")},
    }
    "#);
    assert_eq!(
        err.values.get("input"),
        Some(&gix_error::MetadataValue::from(b"\t".as_slice()))
    );
}

#[test]
fn invalid_attribute_values() {
    let mut diagnostics = Vec::new();
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

        let err = assert_validation(input);
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert!(matches!(err.values.get("input"), Some(gix_error::MetadataValue::Bytes(input)) if input.len() == 1));
    }
    insta::assert_debug_snapshot!(diagnostics, "invalid attribute values", @r##"
    [
        Message {
            message: "Invalid character in attribute value",
            class: Validation,
            values: {"input": Bytes("#")},
        },
        Message {
            message: "Invalid character in attribute value",
            class: Validation,
            values: {"input": Bytes("\\")},
        },
        Message {
            message: "Invalid character in attribute value",
            class: Validation,
            values: {"input": Bytes("\\")},
        },
        Message {
            message: "Invalid character in attribute value",
            class: Validation,
            values: {"input": Bytes("#")},
        },
        Message {
            message: "Invalid character in attribute value",
            class: Validation,
            values: {"input": Bytes("=")},
        },
        Message {
            message: "Invalid character in attribute value",
            class: Validation,
            values: {"input": Bytes("#")},
        },
        Message {
            message: "Invalid character in attribute value",
            class: Validation,
            values: {"input": Bytes("\xef")},
        },
        Message {
            message: "Invalid character in attribute value",
            class: Validation,
            values: {"input": Bytes("\xef")},
        },
    ]
    "##);
}

#[test]
fn escape_character_at_end_of_attribute_value() {
    let mut diagnostics = Vec::new();
    let inputs = vec![
        r":(attr:v=invalid\)some/path",
        r":(attr:v=invalid\ )some/path",
        r":(attr:v=invalid\ valid)some/path",
    ];

    for input in inputs {
        assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

        let err = assert_validation(input);
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert!(
            err.values.contains_key("input"),
            "the invalid attribute value is retained"
        );
    }
    insta::assert_debug_snapshot!(diagnostics, "escape character at end of attribute value", @r#"
    [
        Message {
            message: "Escape character '\\' is not allowed as the last character in an attribute value",
            class: Validation,
            values: {"input": Bytes("invalid\\")},
        },
        Message {
            message: "Escape character '\\' is not allowed as the last character in an attribute value",
            class: Validation,
            values: {"input": Bytes("invalid\\")},
        },
        Message {
            message: "Escape character '\\' is not allowed as the last character in an attribute value",
            class: Validation,
            values: {"input": Bytes("invalid\\")},
        },
    ]
    "#);
}

#[test]
fn empty_attribute_specification() {
    let input = ":(attr:)";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    insta::assert_debug_snapshot!(assert_validation(input), "empty attribute specification", @r#"
    Message {
        message: "Attribute specification cannot be empty",
        class: Validation,
    }
    "#);
}

#[test]
fn multiple_attribute_specifications() {
    let input = ":(attr:one,attr:two)some/path";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let err = assert_validation(input);
    insta::assert_debug_snapshot!(err, "multiple attribute specifications", @r#"
    Message {
        message: "Only one attribute specification is allowed in the same pathspec",
        class: Validation,
        values: {"input": Bytes("attr:two")},
    }
    "#);
    assert!(
        err.values.contains_key("input"),
        "the duplicate attribute specification is retained"
    );
}

#[test]
fn missing_parentheses() {
    let input = ":(top";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let err = assert_validation(input);
    insta::assert_debug_snapshot!(err, "missing parentheses", @r#"
    Message {
        message: "Missing ')' at the end of pathspec signature",
        class: Validation,
        values: {"input": Bytes(":(top")},
    }
    "#);
    assert_eq!(
        err.values.get("input"),
        Some(&gix_error::MetadataValue::from(input.as_bytes()))
    );
}

#[test]
fn glob_and_literal_keywords_present() {
    let input = ":(glob,literal)some/path";

    assert!(!check_against_baseline(input), "This pathspec is valid in git: {input}");

    let err = assert_validation(input);
    insta::assert_debug_snapshot!(err, "glob and literal keywords present", @r#"
    Message {
        message: "'literal' and 'glob' keywords cannot be used together in the same pathspec",
        class: Validation,
        values: {"input": Bytes("literal")},
    }
    "#);
    assert_eq!(
        err.values.get("input"),
        Some(&gix_error::MetadataValue::from(b"literal".as_slice()))
    );
}
