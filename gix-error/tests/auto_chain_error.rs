#[cfg(not(feature = "tree-error"))]
use gix_error::Exn;
use gix_error::{Class, ClassificationMarker, Error, ErrorExt, Message, corruption, message, not_found, validation};
use std::error::Error as _;

#[test]
fn exn_converts_to_boxed_std_error() {
    let err: Box<dyn std::error::Error + Send + Sync> = message("one").raise_typed().into();
    let err = err
        .downcast_ref::<Error>()
        .expect("conversion retains the gix error boundary type");
    insta::assert_debug_snapshot!(err, "exn converts to boxed std error", @"one");
    insta::assert_debug_snapshot!(format_args!("{}", err.probable_cause()), "boxed errors preserve the selected cause", @"one");
}

#[test]
fn erased_validation_error_remains_classified() {
    let err = validation("invalid").raise_erased().into_error();
    insta::assert_debug_snapshot!(err, "the auto-chain Error classifies the original Message retained during ChainedError construction", @"invalid");
    assert!(
        err.is_validation(),
        "the auto-chain Error classifies the original Message retained during ChainedError construction"
    );
}

#[cfg(not(feature = "tree-error"))]
#[test]
fn from_exn_error() {
    let err = Error::from(message("one").raise_typed());
    insta::assert_debug_snapshot!(format_args!("{err:#}"), "alternate Display exposes the converted root diagnostic", @r#"
        one
    "#);
    insta::assert_compact_debug_snapshot!(
        &err,
        "compact Debug includes the diagnostic and caller location",
        @"one, at gix-error/tests/auto_chain_error.rs:29"
    );
    insta::assert_debug_snapshot!(err, @"one");
    assert_eq!(err.source().map(debug_string), None);
}

#[cfg(not(feature = "tree-error"))]
#[test]
fn from_exn_error_tree() {
    let err = Error::from(new_tree_error().raise(message("topmost")));
    insta::assert_debug_snapshot!(format_args!("{}", format!("{err:#}")), "alternate Display includes the aggregate and all causes", @"topmost: E6: E5: E4: E8: E3: E10: E12: E2: E7: E1: E9: E11");
    insta::assert_compact_debug_snapshot!(
        err,
        "compact Debug reports every cause with its caller location",
        @"
    topmost, at gix-error/tests/auto_chain_error.rs:45

    Caused by:
        0: E6, at gix-error/tests/auto_chain_error.rs:226
        1: E5, at gix-error/tests/auto_chain_error.rs:218
        2: E4, at gix-error/tests/auto_chain_error.rs:221
        3: E8, at gix-error/tests/auto_chain_error.rs:224
        4: E3, at gix-error/tests/auto_chain_error.rs:210
        5: E10, at gix-error/tests/auto_chain_error.rs:213
        6: E12, at gix-error/tests/auto_chain_error.rs:216
        7: E2, at gix-error/tests/auto_chain_error.rs:220
        8: E7, at gix-error/tests/auto_chain_error.rs:223
        9: E1, at gix-error/tests/auto_chain_error.rs:209
        10: E9, at gix-error/tests/auto_chain_error.rs:212
        11: E11, at gix-error/tests/auto_chain_error.rs:215
    "
    );
    insta::assert_debug_snapshot!(err, "pretty Debug reports every cause without caller locations", @"
    topmost

    Caused by:
        0: E6
        1: E5
        2: E4
        3: E8
        4: E3
        5: E10
        6: E12
        7: E2
        8: E7
        9: E1
        10: E9
        11: E11
    ");
    insta::assert_debug_snapshot!(
        err.iter_errors().map(|err| fixup_paths(err.to_string())).collect::<Vec<_>>(),
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
        "topmost, at gix-error/tests/auto_chain_error.rs:45",
        "E6, at gix-error/tests/auto_chain_error.rs:226",
        "E5, at gix-error/tests/auto_chain_error.rs:218",
        "E4, at gix-error/tests/auto_chain_error.rs:221",
        "E8, at gix-error/tests/auto_chain_error.rs:224",
        "E3, at gix-error/tests/auto_chain_error.rs:210",
        "E10, at gix-error/tests/auto_chain_error.rs:213",
        "E12, at gix-error/tests/auto_chain_error.rs:216",
        "E2, at gix-error/tests/auto_chain_error.rs:220",
        "E7, at gix-error/tests/auto_chain_error.rs:223",
        "E1, at gix-error/tests/auto_chain_error.rs:209",
        "E9, at gix-error/tests/auto_chain_error.rs:212",
        "E11, at gix-error/tests/auto_chain_error.rs:215",
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
    insta::assert_debug_snapshot!(format_args!("{:#}", err.probable_cause()), "the first causal branch selects its aggregate", @r#"
        E6
    "#);
}

#[test]
fn from_any_error() {
    let err = Error::from_error(message("one"));
    insta::assert_debug_snapshot!(format_args!("{err:#}"), "wrapping a native error preserves its diagnostic", @"one");
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    insta::assert_compact_debug_snapshot!(&err, "wrapping a native error preserves its diagnostic", @"one, at gix-error/tests/auto_chain_error.rs:159");
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_compact_debug_snapshot!(&err, "wrapping a native error preserves its diagnostic", @r#"
    Message { message: "one" }
    "#);
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    insta::assert_debug_snapshot!(err, @"one");
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_debug_snapshot!(err, @r#"
    Message {
        message: "one",
    }
    "#);
    assert_eq!(err.source().map(debug_string), None);
    insta::assert_debug_snapshot!(format_args!("{:#}", err.probable_cause()), "wrapping a native error preserves its diagnostic", @"one");
}

#[test]
fn probable_cause_survives_tree_flattening() {
    let err = Error::from(
        message("bottom")
            .raise_typed()
            .raise(message("middle"))
            .raise(message("top")),
    );
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(err, "probable cause survives tree flattening", @"
        top

        Caused by:
            0: middle
            1: bottom
        ");
    } else {
        insta::assert_debug_snapshot!(err, "probable cause survives tree flattening", @"
        top

        Caused by:
            0: middle
            1: bottom
        ");
    }
    insta::assert_debug_snapshot!(format_args!("{:#}", err.probable_cause()), "flattening retains the selected leaf diagnostic", @"bottom");
}

#[cfg(not(feature = "tree-error"))]
pub fn new_tree_error() -> Exn<Message> {
    let e1 = message("E1").raise_typed();
    let e3 = e1.raise(message("E3"));

    let e9 = message("E9").raise_typed();
    let e10 = e9.raise(message("E10"));

    let e11 = message("E11").raise_typed();
    let e12 = e11.raise(message("E12"));

    let e5 = Exn::raise_all([e3, e10, e12], message("E5"));

    let e2 = message("E2").raise_typed();
    let e4 = e2.raise(message("E4"));

    let e7 = message("E7").raise_typed();
    let e8 = e7.raise(message("E8"));

    Exn::raise_all([e5, e4, e8], message("E6"))
}

pub fn debug_string(input: impl std::fmt::Debug) -> String {
    fixup_paths(format!("{input:?}"))
}

fn fixup_paths(input: String) -> String {
    if cfg!(windows) { input.replace('\\', "/") } else { input }
}

#[test]
fn retryability_is_discovered_in_the_error_chain() {
    let retryable = std::io::Error::new(std::io::ErrorKind::TimedOut, "too slow")
        .and_raise_typed(message("network operation failed"));
    let err = Error::from(retryable);
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "retryability is discovered in the error chain", @"
        network operation failed

        Caused by:
            0: I/O error (TimedOut)
            1: too slow
        ");
    } else {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "retryability is discovered in the error chain", @"
        network operation failed

        Caused by:
            0: I/O error (TimedOut)
            1: too slow
        ");
    }
    assert!(err.can_retry());

    let dependency_specific = ClassificationMarker::with_source(Class::Retryable, message("HTTP/2 stream failed"))
        .and_raise_typed(message("network operation failed"));
    let err = Error::from(dependency_specific);
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "retryability is discovered in the error chain", @"
        network operation failed

        Caused by:
            0: HTTP/2 stream failed
        ");
    } else {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "retryability is discovered in the error chain", @"
        network operation failed

        Caused by:
            0: HTTP/2 stream failed
        ");
    }
    assert!(err.can_retry());
}

#[test]
fn corruption_is_discovered_in_the_error_chain() {
    let corrupt = corruption("checksum mismatch").and_raise_typed(message("failed to open object database"));
    let err = Error::from(corrupt);
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "corruption is discovered in the error chain", @"
        failed to open object database

        Caused by:
            0: checksum mismatch
        ");
    } else {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "corruption is discovered in the error chain", @"
        failed to open object database

        Caused by:
            0: checksum mismatch
        ");
    }
    assert!(err.is_corrupted());
}

#[test]
fn not_found_is_discovered_in_well_known_errors() {
    let missing = not_found("reference does not exist").and_raise_typed(message("failed to resolve HEAD"));
    let err = Error::from(missing);
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "not found is discovered in well known errors", @"
        failed to resolve HEAD

        Caused by:
            0: reference does not exist
        ");
    } else {
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "not found is discovered in well known errors", @"
        failed to resolve HEAD

        Caused by:
            0: reference does not exist
        ");
    }
    assert!(err.is_not_found());
    let err = Error::from_error(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "not found is discovered in well known errors", @"
    I/O error (NotFound)

    Caused by:
        0: missing
    ");
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "not found is discovered in well known errors", @r#"
    Custom {
        kind: NotFound,
        error: "missing",
    }
    "#);
    assert!(err.is_not_found());
    let err = Error::from_boxed(Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "missing object",
    )));
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "not found is discovered in well known errors", @"
    I/O error (NotFound)

    Caused by:
        0: missing object
    ");
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "not found is discovered in well known errors", @r#"
    Custom {
        kind: NotFound,
        error: "missing object",
    }
    "#);
    assert!(err.is_not_found());
}

#[test]
fn validation_is_discovered_in_the_error_chain() {
    let err = Error::from_error(validation("invalid"));
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "validation is discovered in the error chain", @"invalid");
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "validation is discovered in the error chain", @r#"
    Message {
        message: "invalid",
        class: Validation,
    }
    "#);
    assert!(err.is_validation());
    let err = Error::from_error(ErrorWithSource(validation("invalid")));
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "validation is discovered in the error chain", @"
    invalid

    Caused by:
        0: invalid
    ");
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "validation is discovered in the error chain", @r#"
    ErrorWithSource(
        Message {
            message: "invalid",
            class: Validation,
        },
    )
    "#);
    assert!(err.is_validation());

    let err = Error::from(validation("typed").and_raise_typed(message("context")));
    assert!(
        err.iter_errors().any(<dyn std::error::Error>::is::<Message>),
        "iter_errors() exposes the stored error types in chain mode"
    );
    assert!(
        err.iter_errors_with_locations()
            .any(|source| source.error().is::<Message>()),
        "iter_errors_with_locations() preserves the stored error types alongside their locations"
    );
}

#[test]
fn classification_survives_raising_a_converted_error() {
    let converted = Error::from_error(ErrorWithSource(validation("invalid object header")));
    let raised = Error::from(converted.and_raise_typed(message("revision parsing failed")));
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(raised, "classification survives raising a converted error", @"
        revision parsing failed

        Caused by:
            0: invalid object header
            1: invalid object header
        ");
    } else {
        insta::assert_debug_snapshot!(raised, "classification survives raising a converted error", @"
        revision parsing failed

        Caused by:
            0: invalid object header
            1: invalid object header
        ");
    }
    assert!(raised.is_validation());
}

#[test]
#[cfg(not(feature = "tree-error"))]
fn raising_a_converted_error_preserves_stored_types() {
    let converted = Error::from(validation("invalid object header").and_raise_typed(message("object lookup failed")));
    let converted = Error::from_error(converted);
    let raised = converted.and_raise_typed(message("revision parsing failed"));
    insta::assert_debug_snapshot!(
        raised,
        "raising a converted Error retains all nested context",
        @"
    revision parsing failed

    Caused by:
        0: object lookup failed
        1: invalid object header
    ");
    let raised = Error::from(raised);

    assert!(
        raised.iter_errors().any(<dyn std::error::Error>::is::<Message>),
        "the nested Error retains its typed frames"
    );
    assert!(
        raised
            .iter_errors_with_locations()
            .any(|source| source.error().is::<Message>()),
        "iter_errors_with_locations() recursively exposes typed errors from nested Error values"
    );
    insta::assert_debug_snapshot!(raised, "probable_cause() returns the stored error, not a string-backed copy", @"
    revision parsing failed

    Caused by:
        0: object lookup failed
        1: invalid object header
    ");
    assert!(
        raised.probable_cause().is::<Message>(),
        "probable_cause() returns the stored error, not a string-backed copy"
    );
}

#[derive(Debug)]
struct ErrorWithSource<E>(E);

impl<E: std::fmt::Display> std::fmt::Display for ErrorWithSource<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ErrorWithSource<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}
