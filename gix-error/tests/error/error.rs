use crate::{ErrorWithSource, debug_string, new_tree_error};
use gix_error::{Error, ErrorExt, ValidationError, message};
use std::error::Error as _;

#[test]
fn from_exn_error() {
    let err = Error::from(message("one").raise());
    assert_eq!(err.to_string(), "one");
    insta::assert_snapshot!(debug_string(&err), @"one, at gix-error/tests/error/error.rs:7");
    insta::assert_debug_snapshot!(err, @"one");
    assert_eq!(err.source().map(debug_string), None);
}

#[test]
fn from_exn_error_tree() {
    let err = Error::from(new_tree_error().raise(message("topmost")));
    assert_eq!(err.to_string(), "topmost");
    insta::assert_snapshot!(debug_string(&err), @"
    topmost, at gix-error/tests/error/error.rs:16
    |
    └─ E6, at gix-error/tests/error/main.rs:25
        |
        └─ E5, at gix-error/tests/error/main.rs:17
        |   |
        |   └─ E3, at gix-error/tests/error/main.rs:9
        |   |   |
        |   |   └─ E1, at gix-error/tests/error/main.rs:8
        |   |
        |   └─ E10, at gix-error/tests/error/main.rs:12
        |   |   |
        |   |   └─ E9, at gix-error/tests/error/main.rs:11
        |   |
        |   └─ E12, at gix-error/tests/error/main.rs:15
        |       |
        |       └─ E11, at gix-error/tests/error/main.rs:14
        |
        └─ E4, at gix-error/tests/error/main.rs:20
        |   |
        |   └─ E2, at gix-error/tests/error/main.rs:19
        |
        └─ E8, at gix-error/tests/error/main.rs:23
            |
            └─ E7, at gix-error/tests/error/main.rs:22
    ");
    insta::assert_debug_snapshot!(err, @r"
    topmost
    |
    └─ E6
        |
        └─ E5
        |   |
        |   └─ E3
        |   |   |
        |   |   └─ E1
        |   |
        |   └─ E10
        |   |   |
        |   |   └─ E9
        |   |
        |   └─ E12
        |       |
        |       └─ E11
        |
        └─ E4
        |   |
        |   └─ E2
        |
        └─ E8
            |
            └─ E7
    ");
    insta::assert_debug_snapshot!(err.sources().map(ToString::to_string).collect::<Vec<_>>(), @r#"
    [
        "topmost",
        "E6",
        "E5",
        "E4",
        "E8",
        "E3",
        "E10",
        "E12",
        "E2",
        "E7",
        "E1",
        "E9",
        "E11",
    ]
    "#);
    assert_eq!(
        err.source().map(debug_string).as_deref(),
        Some(r#"Message("E6")"#),
        "The source is the first child"
    );
    assert_eq!(
        err.probable_cause().to_string(),
        "E6",
        "we get the top-most error that has most causes"
    );
}

#[test]
fn from_any_error() {
    let err = Error::from_error(message("one"));
    assert_eq!(err.to_string(), "one");
    assert_eq!(debug_string(&err), r#"Message("one")"#);
    insta::assert_debug_snapshot!(err, @r#"
    Message(
        "one",
    )
    "#);
    assert_eq!(err.source().map(debug_string), None);
    assert_eq!(err.probable_cause().to_string(), "one");
}

#[test]
fn from_any_error_with_source() {
    let err = Error::from_error(ErrorWithSource("main", message("one")));
    assert_eq!(err.to_string(), "main", "display is the error itself");
    assert_eq!(debug_string(&err), r#"ErrorWithSource("main", Message("one"))"#);
    insta::assert_debug_snapshot!(err, @r#"
    ErrorWithSource(
        "main",
        Message(
            "one",
        ),
    )
    "#);
    assert_eq!(
        err.source().map(debug_string).as_deref(),
        Some(r#"Message("one")"#),
        "The source is provided by the wrapped error"
    );
}

#[test]
fn validation_error_displays_input_with_debug_formatting() {
    let err = ValidationError::new_with_input("invalid input", "hello\n ");
    assert_eq!(
        err.to_string(),
        "invalid input: \"hello\\n \"",
        "it won't hide whitespace and other special characters"
    );
}
