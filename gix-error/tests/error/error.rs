use crate::ErrorWithSource;
#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
use crate::{debug_string, fixup_paths, new_tree_error};
use gix_error::{Class, ClassificationMarker, Error, ErrorExt, Message, corruption, message, not_found, validation};
#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
use std::error::Error as _;

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
#[test]
fn from_exn_error() {
    let err = Error::from(message("one").raise_typed());
    assert_eq!(err, "one");
    insta::assert_compact_debug_snapshot!(
        &err,
        "compact Debug includes the caller location of the root frame",
        @"one, at gix-error/tests/error/error.rs:11"
    );
    insta::assert_debug_snapshot!(err, "pretty Debug omits caller locations", @"one");
    assert_eq!(err.source().map(debug_string), None);
}

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
#[test]
fn from_exn_error_tree() {
    let err = Error::from(new_tree_error().raise(message("topmost")));
    assert_eq!(err, "topmost");
    insta::assert_compact_debug_snapshot!(&err, "compact Debug renders the complete tree with caller locations", @"
    topmost, at gix-error/tests/error/error.rs:25
    |
    └─ E6, at gix-error/tests/error/main.rs:26
        |
        └─ E5, at gix-error/tests/error/main.rs:18
        |   |
        |   └─ E3, at gix-error/tests/error/main.rs:10
        |   |   |
        |   |   └─ E1, at gix-error/tests/error/main.rs:9
        |   |
        |   └─ E10, at gix-error/tests/error/main.rs:13
        |   |   |
        |   |   └─ E9, at gix-error/tests/error/main.rs:12
        |   |
        |   └─ E12, at gix-error/tests/error/main.rs:16
        |       |
        |       └─ E11, at gix-error/tests/error/main.rs:15
        |
        └─ E4, at gix-error/tests/error/main.rs:21
        |   |
        |   └─ E2, at gix-error/tests/error/main.rs:20
        |
        └─ E8, at gix-error/tests/error/main.rs:24
            |
            └─ E7, at gix-error/tests/error/main.rs:23
    ");
    insta::assert_debug_snapshot!(err, "pretty Debug renders the complete tree without caller locations", @r"
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
    insta::assert_debug_snapshot!(
        err.iter_errors().map(ToString::to_string).collect::<Vec<_>>(),
        "error iteration exposes the original errors without their frame locations",
        @r#"
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
    insta::assert_debug_snapshot!(
        err.iter_errors_with_locations().map(|source| fixup_paths(source.to_string())).collect::<Vec<_>>(),
        "error iteration with locations exposes the same errors together with their caller locations",
        @r#"
    [
        "topmost, at gix-error/tests/error/error.rs:25",
        "E6, at gix-error/tests/error/main.rs:26",
        "E5, at gix-error/tests/error/main.rs:18",
        "E4, at gix-error/tests/error/main.rs:21",
        "E8, at gix-error/tests/error/main.rs:24",
        "E3, at gix-error/tests/error/main.rs:10",
        "E10, at gix-error/tests/error/main.rs:13",
        "E12, at gix-error/tests/error/main.rs:16",
        "E2, at gix-error/tests/error/main.rs:20",
        "E7, at gix-error/tests/error/main.rs:23",
        "E1, at gix-error/tests/error/main.rs:9",
        "E9, at gix-error/tests/error/main.rs:12",
        "E11, at gix-error/tests/error/main.rs:15",
    ]
    "#
    );
    assert_eq!(
        err.iter_errors_with_locations()
            .map(|source| format!("{source:#}"))
            .collect::<Vec<_>>(),
        err.iter_errors().map(ToString::to_string).collect::<Vec<_>>(),
        "alternate display-source formatting exposes the underlying errors without locations"
    );
    let first_error = err
        .iter_errors_with_locations()
        .next()
        .expect("the root error with location is present");
    assert_eq!(
        first_error
            .location()
            .expect("the root frame has a captured caller location")
            .file(),
        file!(),
        "errors with locations expose their caller location"
    );
    insta::assert_debug_snapshot!(err.source(), "The source is the first child", @r#"
    Some(
        Message {
            message: "E6",
        },
    )
    "#);
    insta::assert_debug_snapshot!(format_args!("{}", err.probable_cause()), "the first causal branch selects its aggregate", @"E6");
}

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
#[test]
fn from_any_error() {
    let err = Error::from_error(message("one"));
    assert_eq!(err, "one");
    insta::assert_compact_debug_snapshot!(&err, "wrapping a native error preserves its diagnostic", @r#"Message { message: "one" }"#);
    insta::assert_debug_snapshot!(err, @r#"
    Message {
        message: "one",
    }
    "#);
    assert_eq!(err.source().map(debug_string), None);
    insta::assert_debug_snapshot!(format_args!("{}", err.probable_cause()), "wrapping a native error preserves its diagnostic", @"one");
}

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
#[test]
fn from_any_error_with_source() {
    let err = Error::from_error(ErrorWithSource("main", message("one")));
    assert_eq!(err, "main", "display is the error itself");
    insta::assert_compact_debug_snapshot!(&err, "native errors preserve their own Debug representation", @r#"ErrorWithSource("main", Message { message: "one" })"#);
    insta::assert_debug_snapshot!(err, @r#"
    ErrorWithSource(
        "main",
        Message {
            message: "one",
        },
    )
    "#);
    insta::assert_debug_snapshot!(err.source(), "The source is provided by the wrapped error", @r#"
    Some(
        Message {
            message: "one",
        },
    )
    "#);
}

#[test]
fn native_sources_retain_types_without_claiming_frame_locations() {
    type Middle = ErrorWithSource<Message>;
    type Root = ErrorWithSource<Middle>;

    let err = Error::from_error(ErrorWithSource("top", ErrorWithSource("middle", message("bottom"))));
    let errors = err.iter_errors().collect::<Vec<_>>();
    assert_eq!(errors.len(), 3, "the root and both native sources are exposed once");
    assert!(
        errors[0].is::<Root>(),
        "the owning root error retains its concrete type"
    );
    assert!(
        errors[1].is::<Middle>(),
        "the native middle source retains its concrete type"
    );
    assert!(
        errors[2].is::<Message>(),
        "the native source leaf retains its concrete type"
    );
    insta::assert_debug_snapshot!(err, "downcast_any_ref() searches native sources as well as stored frames", @r#"
    ErrorWithSource(
        "top",
        ErrorWithSource(
            "middle",
            Message {
                message: "bottom",
            },
        ),
    )
    "#);
    assert!(
        err.downcast_any_ref::<Middle>().is_some(),
        "downcast_any_ref() searches native sources as well as stored frames"
    );

    let errors_with_locations = err.iter_errors_with_locations().collect::<Vec<_>>();
    assert!(
        errors_with_locations
            .first()
            .expect("the root error is present")
            .location()
            .is_some(),
        "the root frame has its captured caller location"
    );
    assert!(
        errors_with_locations[1..]
            .iter()
            .all(|source| source.location().is_none()),
        "native sources have no caller location of their own"
    );
    assert!(
        errors_with_locations[1].error().is::<Middle>() && errors_with_locations[2].error().is::<Message>(),
        "iter_errors_with_locations() preserves native source types even without locations"
    );

    let err = Error::from(
        ErrorWithSource("root", ErrorWithSource("root source", message("root source leaf")))
            .raise_typed()
            .chain(ErrorWithSource("explicit child", message("child source"))),
    );
    insta::assert_debug_snapshot!(err.iter_errors().map(ToString::to_string).collect::<Vec<_>>(), "native sources and explicit frames share one logical breadth-first order", @r#"
    [
        "root",
        "root source",
        "explicit child",
        "root source leaf",
        "child source",
    ]
    "#);
    assert_eq!(
        err.iter_errors_with_locations()
            .map(|source| source.location().is_some())
            .collect::<Vec<_>>(),
        [true, false, true, false, false],
        "only explicitly created frames have captured caller locations"
    );
}

#[test]
fn nested_errors_are_expanded_in_breadth_first_order() {
    let nested = Error::from(message("nested root").raise_typed().chain(message("nested child")));
    let err = Error::from(
        message("outer root")
            .raise_typed()
            .chain(nested)
            .chain(message("outer sibling")),
    );

    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_debug_snapshot!(err, @r#"
    outer root
    |
    └─ nested root
    |   |
    |   └─ nested child
    |
    └─ outer sibling
    "#);

    fn name(error: &(dyn std::error::Error + 'static)) -> String {
        if let Some(error) = error.downcast_ref::<Error>() {
            format!("{:?}", error.error())
        } else {
            error.to_string()
        }
    }

    let expected = err.iter_errors().map(name).collect::<Vec<_>>();
    insta::assert_debug_snapshot!(expected, "a nested Error's root is queued behind the remaining errors at its wrapper's depth", @r#"
    [
        "outer root",
        "Message { message: \"nested root\" }",
        "outer sibling",
        "nested root",
        "nested child",
    ]
    "#);
    assert_eq!(
        err.iter_errors_with_locations()
            .map(|source| name(source.error()))
            .collect::<Vec<_>>(),
        expected,
        "location-aware iteration uses the same breadth-first order"
    );
}

#[test]
fn classification_survives_raising_a_converted_error() {
    let converted = Error::from_error(ErrorWithSource(
        "object lookup failed",
        validation("invalid object header"),
    ));
    let err = converted.and_raise_typed(message("revision parsing failed"));

    insta::assert_debug_snapshot!(err, "exceptions inspect validation causes within nested errors", @"
    revision parsing failed
    |
    └─ object lookup failed
    |
    └─ invalid object header
    ");
    assert!(
        err.is_validation(),
        "exceptions inspect validation causes within nested errors"
    );
    let err = Error::from(err);
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "classification survives raising a converted error", @r#"
        Message {
            message: "revision parsing failed",
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "classification survives raising a converted error", @"
        revision parsing failed
        |
        └─ object lookup failed
        |
        └─ invalid object header
        ");
    }
    assert!(err.is_validation());
    let err = validation("invalid").raise_typed();
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "classification survives raising a converted error", @"invalid");
    assert!(err.is_validation());
    let err = std::io::Error::from(std::io::ErrorKind::InvalidInput).raise_typed();
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "an I/O kind does not establish an explicit validation classification", @"invalid input parameter");
    assert!(
        !err.is_validation(),
        "an I/O kind does not establish an explicit validation classification"
    );
    let err = message("validation failed").raise_typed();
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "classification survives raising a converted error", @"validation failed");
    assert!(!err.is_validation());
}

#[test]
fn raising_a_converted_error_preserves_stored_types() {
    let converted = Error::from(validation("invalid object header").and_raise_typed(message("object lookup failed")));
    let converted = Error::from_error(converted);
    let err = Error::from(converted.and_raise_typed(message("revision parsing failed")));

    assert!(
        err.iter_errors().any(<dyn std::error::Error>::is::<Message>),
        "the nested Error retains its typed frames"
    );
    assert!(
        err.iter_errors_with_locations()
            .any(|source| source.error().is::<Message>()),
        "iter_errors_with_locations() recursively exposes typed errors from nested Error values"
    );
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(err, "probable_cause() returns the stored error, not a string-backed copy", @r#"
        Message {
            message: "revision parsing failed",
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(err, "probable_cause() returns the stored error, not a string-backed copy", @"
        revision parsing failed
        |
        └─ object lookup failed
        |
        └─ invalid object header
        ");
    }
    assert!(
        err.probable_cause().is::<Message>(),
        "probable_cause() returns the stored error, not a string-backed copy"
    );
}

#[test]
fn validation_error_displays_input_with_debug_formatting() {
    let err = validation("invalid input").with("input", b"hello\n ".as_slice());
    insta::assert_debug_snapshot!(format_args!("{}", err), "it won't hide whitespace and other special characters", @r#"invalid input, "input"="hello\n ""#);
    let err = Error::from_error(err);
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "validation error displays input with debug formatting", @r#"
    Message {
        message: "invalid input",
        class: Validation,
        values: {"input": Bytes("hello\n ")},
    }
    "#);
    assert!(err.is_validation());
    let err = Error::from_error(ErrorWithSource("validation failed", validation("invalid")));
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "validation error displays input with debug formatting", @r#"
    ErrorWithSource(
        "validation failed",
        Message {
            message: "invalid",
            class: Validation,
        },
    )
    "#);
    assert!(err.is_validation());
}

#[test]
fn retryability_is_discovered_in_the_error_chain() {
    let retryable = std::io::Error::new(std::io::ErrorKind::TimedOut, "too slow")
        .and_raise_typed(message("network operation failed"));
    let err = Error::from(retryable);
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "retryability is discovered in the error chain", @r#"
        Message {
            message: "network operation failed",
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "retryability is discovered in the error chain", @"
        network operation failed
        |
        └─ I/O error (TimedOut)
        |
        └─ too slow
        ");
    }
    assert!(err.can_retry());

    let permanent = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied")
        .and_raise_typed(message("network operation failed"));
    let err = Error::from(permanent);
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "retryability is discovered in the error chain", @r#"
        Message {
            message: "network operation failed",
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "retryability is discovered in the error chain", @"
        network operation failed
        |
        └─ I/O error (PermissionDenied)
        |
        └─ denied
        ");
    }
    assert!(!err.can_retry());

    let dependency_specific = ClassificationMarker::with_source(Class::Retryable, message("HTTP/2 stream failed"))
        .and_raise_typed(message("network operation failed"));
    let err = Error::from(dependency_specific);
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "retryability is discovered in the error chain", @r#"
        Message {
            message: "network operation failed",
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "retryability is discovered in the error chain", @"
        network operation failed
        |
        └─ HTTP/2 stream failed
        ");
    }
    assert!(err.can_retry());
}

#[test]
fn corruption_is_discovered_in_the_error_chain() {
    let corrupt = corruption("checksum mismatch").and_raise_typed(message("failed to open object database"));
    insta::assert_debug_snapshot!(corrupt, "exceptions recognize corruption below context", @"
    failed to open object database
    |
    └─ checksum mismatch
    ");
    assert!(corrupt.is_corrupted(), "exceptions recognize corruption below context");
    let err = Error::from(corrupt);
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "corruption is discovered in the error chain", @r#"
        Message {
            message: "failed to open object database",
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "corruption is discovered in the error chain", @"
        failed to open object database
        |
        └─ checksum mismatch
        ");
    }
    assert!(err.is_corrupted());

    let nested = Error::from_error(ErrorWithSource("invalid stream", corruption("bad checksum"))).raise_erased();
    insta::assert_debug_snapshot!(nested, "erased exceptions inspect native sources in nested errors", @"
    invalid stream
    |
    └─ bad checksum
    ");
    assert!(
        nested.is_corrupted(),
        "erased exceptions inspect native sources in nested errors"
    );
    let err = std::io::Error::from(std::io::ErrorKind::InvalidData).raise_typed();
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "an I/O kind does not establish an explicit corruption classification", @"invalid data");
    assert!(
        !err.is_corrupted(),
        "an I/O kind does not establish an explicit corruption classification"
    );
    let unknown = message("repository was not found").raise_typed();
    insta::assert_debug_snapshot!(unknown, "messages do not establish a classification", @"repository was not found");
    assert!(!unknown.is_corrupted(), "messages do not establish a classification");
    let err = Error::from(unknown);
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "corruption is discovered in the error chain", @r#"
        Message {
            message: "repository was not found",
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "corruption is discovered in the error chain", @"repository was not found");
    }
    assert!(!err.is_corrupted());
}

#[test]
fn from_boxed_does_not_repeat_the_wrapped_error_as_its_source() {
    let err = Error::from_boxed(Box::new(message("boxed error")));

    insta::assert_debug_snapshot!(format_args!("{err:#}"), "location-free formatting displays the boxed error as the root", @"boxed error");
    assert!(
        std::error::Error::source(&err).is_none(),
        "a boxed leaf error must not also appear as its own source"
    );
    insta::assert_debug_snapshot!(err.iter_errors().map(ToString::to_string).collect::<Vec<_>>(), "error iteration must yield the boxed error only once", @r#"
    [
        "boxed error",
    ]
    "#);
}

#[test]
fn not_found_is_discovered_in_well_known_errors() {
    let classified = not_found("reference does not exist").and_raise_typed(message("failed to resolve HEAD"));
    insta::assert_debug_snapshot!(classified, "exceptions recognize missing-resource markers", @"
    failed to resolve HEAD
    |
    └─ reference does not exist
    ");
    assert!(
        classified.is_not_found(),
        "exceptions recognize missing-resource markers"
    );
    let err = Error::from(classified);
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "not found is discovered in well known errors", @r#"
        Message {
            message: "failed to resolve HEAD",
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "not found is discovered in well known errors", @"
        failed to resolve HEAD
        |
        └─ reference does not exist
        ");
    }
    assert!(err.is_not_found());

    let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing index")
        .and_raise_typed(message("failed to open repository"));
    insta::assert_debug_snapshot!(io, "exceptions normalize I/O not-found errors", @"
    failed to open repository
    |
    └─ I/O error (NotFound)
    |
    └─ missing index
    ");
    assert!(io.is_not_found(), "exceptions normalize I/O not-found errors");
    let err = Error::from(io);
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "not found is discovered in well known errors", @r#"
        Message {
            message: "failed to open repository",
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "not found is discovered in well known errors", @"
        failed to open repository
        |
        └─ I/O error (NotFound)
        |
        └─ missing index
        ");
    }
    assert!(err.is_not_found());

    let boxed = Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "missing object"));
    let err = Error::from_boxed(boxed);
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "not found is discovered in well known errors", @r#"
    Custom {
        kind: NotFound,
        error: "missing object",
    }
    "#);
    assert!(err.is_not_found());

    let invalid = ErrorWithSource("invalid config", std::io::Error::from(std::io::ErrorKind::NotFound))
        .and_raise_typed(validation("invalid worktree"))
        .erased();
    insta::assert_debug_snapshot!(invalid, "a validation boundary does not hide its missing-resource source", @"
    invalid worktree
    |
    └─ invalid config
    |
    └─ entity not found
    ");
    assert!(
        invalid.is_not_found(),
        "a validation boundary does not hide its missing-resource source"
    );
    let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied).raise_typed();
    insta::assert_debug_snapshot!(denied, "other I/O kinds are not missing resources", @"permission denied");
    assert!(!denied.is_not_found(), "other I/O kinds are not missing resources");
    let unknown = message("permission denied").raise_typed();
    insta::assert_debug_snapshot!(unknown, "messages do not establish a classification", @"permission denied");
    assert!(!unknown.is_not_found(), "messages do not establish a classification");
    let err = Error::from(unknown);
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "not found is discovered in well known errors", @r#"
        Message {
            message: "permission denied",
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "not found is discovered in well known errors", @"permission denied");
    }
    assert!(!err.is_not_found());
}

#[test]
fn equality_with_strings_uses_the_root_errors_display() {
    let exn = message("cause").raise_typed().raise(message("failure"));
    insta::assert_debug_snapshot!(exn, "string equality uses the root while diagnostics retain its cause", @"
    failure
    |
    └─ cause
    ");
    assert_eq!(exn, "failure");
    assert_eq!(exn, String::from("failure"));
    assert_eq!(&exn, "failure");
    assert_ne!(exn, "cause", "children aren't compared");

    let error = Error::from(exn);
    assert_eq!(error, "failure");
    assert_eq!(&error, "failure");
    assert_eq!(error, String::from("failure"));
    assert_ne!(error, "other");

    let nested = Error::from(message("nested cause").raise_typed().raise(message("nested root"))).raise_erased();
    insta::assert_debug_snapshot!(nested, "nested error boundaries retain their context and cause", @"
    nested root
    |
    └─ nested cause
    ");
    assert_eq!(nested, "nested root", "nested error boundaries are transparent");
    assert_eq!(Error::from(nested), "nested root");
}
