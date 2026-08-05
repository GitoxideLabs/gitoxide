use crate::{ErrorWithSource, debug_string, new_tree_error};
use gix_error::{CorruptionError, Error, ErrorExt, NotFoundError, RetryableError, ValidationError, message};
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
fn classification_survives_raising_a_converted_error() {
    let converted = Error::from_error(ErrorWithSource(
        "object lookup failed",
        ValidationError::new("invalid object header"),
    ));
    let err = Error::from(converted.and_raise(message("revision parsing failed")));

    assert!(err.is_validation());
}

#[test]
fn raising_a_converted_error_preserves_stored_types() {
    let converted =
        Error::from(ValidationError::new("invalid object header").and_raise(message("object lookup failed")));
    let err = Error::from(converted.and_raise(message("revision parsing failed")));

    assert!(
        err.sources().any(|source| source.is::<ValidationError>()),
        "the nested Error retains its typed frames"
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
    assert!(Error::from_error(err).is_validation());
    assert!(Error::from_error(ErrorWithSource("validation failed", ValidationError::new("invalid"))).is_validation());
}

#[test]
fn retryability_is_discovered_in_the_error_chain() {
    let retryable =
        std::io::Error::new(std::io::ErrorKind::TimedOut, "too slow").and_raise(message("network operation failed"));
    assert!(Error::from(retryable).can_retry());

    let permanent = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied")
        .and_raise(message("network operation failed"));
    assert!(!Error::from(permanent).can_retry());

    let dependency_specific =
        RetryableError::new(message("HTTP/2 stream failed")).and_raise(message("network operation failed"));
    assert!(Error::from(dependency_specific).can_retry());
}

#[test]
fn corruption_is_discovered_in_the_error_chain() {
    let corrupt = CorruptionError::new("checksum mismatch").and_raise(message("failed to open object database"));
    assert!(Error::from(corrupt).is_corrupted());

    assert!(!Error::from(message("repository was not found").raise()).is_corrupted());
}

#[test]
fn not_found_is_discovered_in_well_known_errors() {
    let classified = NotFoundError::new("reference does not exist").and_raise(message("failed to resolve HEAD"));
    assert!(Error::from(classified).is_not_found());

    let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing index")
        .and_raise(message("failed to open repository"));
    assert!(Error::from(io).is_not_found());

    let boxed = Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "missing object"));
    assert!(Error::from_boxed(boxed).is_not_found());

    assert!(!Error::from(message("permission denied").raise()).is_not_found());
}
