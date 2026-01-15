use crate::{debug_string, new_tree_error, Error as Simple, ErrorWithSource};
use gix_error::{Error, ErrorExt};
use std::error::Error as _;

#[test]
fn from_exn_error() {
    let err = Error::from(Simple("one").raise());
    assert_eq!(err.to_string(), "one");
    insta::assert_snapshot!(debug_string(&err), @"one, at gix-error/tests/error/error.rs:7:41");
    insta::assert_debug_snapshot!(err, @"one");
    assert_eq!(err.source().map(debug_string), None);
}

#[test]
fn from_exn_error_tree() {
    let err = Error::from(new_tree_error().raise(Simple("topmost")));
    assert_eq!(err.to_string(), "topmost");
    insta::assert_snapshot!(debug_string(&err), @r"
    topmost, at gix-error/tests/error/error.rs:16:44
    |
    └─ E6, at gix-error/tests/error/main.rs:25:9
        |
        └─ E5, at gix-error/tests/error/main.rs:17:18
        |   |
        |   └─ E3, at gix-error/tests/error/main.rs:9:21
        |   |   |
        |   |   └─ E1, at gix-error/tests/error/main.rs:8:30
        |   |
        |   └─ E10, at gix-error/tests/error/main.rs:12:22
        |   |   |
        |   |   └─ E9, at gix-error/tests/error/main.rs:11:30
        |   |
        |   └─ E12, at gix-error/tests/error/main.rs:15:23
        |       |
        |       └─ E11, at gix-error/tests/error/main.rs:14:32
        |
        └─ E4, at gix-error/tests/error/main.rs:20:21
        |   |
        |   └─ E2, at gix-error/tests/error/main.rs:19:30
        |
        └─ E8, at gix-error/tests/error/main.rs:23:21
            |
            └─ E7, at gix-error/tests/error/main.rs:22:30
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
    insta::assert_debug_snapshot!(err.iter_frames().map(ToString::to_string).collect::<Vec<_>>(), @r#"
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
        Some(r#"Error("E6")"#),
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
    let err = Error::from_error(Simple("one"));
    assert_eq!(err.to_string(), "one");
    assert_eq!(debug_string(&err), r#"Error("one")"#);
    insta::assert_debug_snapshot!(err, @r#"
    Error(
        "one",
    )
    "#);
    assert_eq!(err.source().map(debug_string), None);
    assert_eq!(err.probable_cause().to_string(), "one");
}

#[test]
fn from_any_error_with_source() {
    let err = Error::from_error(ErrorWithSource("main", Simple("one")));
    assert_eq!(err.to_string(), "main", "display is the error itself");
    assert_eq!(debug_string(&err), r#"ErrorWithSource("main", Error("one"))"#);
    insta::assert_debug_snapshot!(err, @r#"
    ErrorWithSource(
        "main",
        Error(
            "one",
        ),
    )
    "#);
    assert_eq!(
        err.source().map(debug_string).as_deref(),
        Some(r#"Error("one")"#),
        "The source is provided by the wrapped error"
    );
}
