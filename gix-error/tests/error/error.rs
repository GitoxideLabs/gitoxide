use crate::ErrorWithSource;
#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
use crate::{debug_string, fixup_paths, new_tree_error};
use gix_error::{CorruptionError, Error, ErrorExt, Message, NotFoundError, RetryableError, ValidationError, message};
#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
use std::error::Error as _;

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
#[test]
fn from_exn_error() {
    let err = Error::from(message("one").raise());
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
        "E6, at gix-error/tests/error/main.rs:25",
        "E5, at gix-error/tests/error/main.rs:17",
        "E4, at gix-error/tests/error/main.rs:20",
        "E8, at gix-error/tests/error/main.rs:23",
        "E3, at gix-error/tests/error/main.rs:9",
        "E10, at gix-error/tests/error/main.rs:12",
        "E12, at gix-error/tests/error/main.rs:15",
        "E2, at gix-error/tests/error/main.rs:19",
        "E7, at gix-error/tests/error/main.rs:22",
        "E1, at gix-error/tests/error/main.rs:8",
        "E9, at gix-error/tests/error/main.rs:11",
        "E11, at gix-error/tests/error/main.rs:14",
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

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
#[test]
fn from_any_error() {
    let err = Error::from_error(message("one"));
    assert_eq!(err, "one");
    assert_eq!(debug_string(&err), r#"Message("one")"#);
    insta::assert_debug_snapshot!(err, @r#"
    Message(
        "one",
    )
    "#);
    assert_eq!(err.source().map(debug_string), None);
    assert_eq!(err.probable_cause().to_string(), "one");
}

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
#[test]
fn from_any_error_with_source() {
    let err = Error::from_error(ErrorWithSource("main", message("one")));
    assert_eq!(err, "main", "display is the error itself");
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
            .raise()
            .chain(ErrorWithSource("explicit child", message("child source"))),
    );
    assert_eq!(
        err.iter_errors().map(ToString::to_string).collect::<Vec<_>>(),
        [
            "root",
            "root source",
            "explicit child",
            "root source leaf",
            "child source",
        ],
        "native sources and explicit frames share one logical breadth-first order"
    );
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
    let nested = Error::from(message("nested root").raise().chain(message("nested child")));
    let err = Error::from(
        message("outer root")
            .raise()
            .chain(nested)
            .chain(message("outer sibling")),
    );

    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_debug_snapshot!(err, @r#"
    outer root
    |
    └─ Message("nested root")
    |
    └─ Message("nested child")
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

    let expected = [
        "outer root",
        r#"Message("nested root")"#,
        "outer sibling",
        "nested root",
        "nested child",
    ];
    assert_eq!(
        err.iter_errors().map(name).collect::<Vec<_>>(),
        expected,
        "a nested Error's root is queued behind the remaining errors at its wrapper's depth"
    );
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
        ValidationError::new("invalid object header"),
    ));
    let err = Error::from(converted.and_raise(message("revision parsing failed")));

    assert!(err.is_validation());
}

#[test]
fn raising_a_converted_error_preserves_stored_types() {
    let converted =
        Error::from(ValidationError::new("invalid object header").and_raise(message("object lookup failed")));
    let converted = Error::from_error(converted);
    let err = Error::from(converted.and_raise(message("revision parsing failed")));

    assert!(
        err.iter_errors().any(<dyn std::error::Error>::is::<ValidationError>),
        "the nested Error retains its typed frames"
    );
    assert!(
        err.iter_errors_with_locations()
            .any(|source| source.error().is::<ValidationError>()),
        "iter_errors_with_locations() recursively exposes typed errors from nested Error values"
    );
    assert!(
        err.probable_cause().is::<ValidationError>(),
        "probable_cause() returns the stored error, not a string-backed copy"
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
fn from_boxed_does_not_repeat_the_wrapped_error_as_its_source() {
    let err = Error::from_boxed(Box::new(message("boxed error")));

    assert_eq!(
        format!("{err:#}"),
        "boxed error",
        "location-free formatting displays the boxed error as the root"
    );
    assert!(
        std::error::Error::source(&err).is_none(),
        "a boxed leaf error must not also appear as its own source"
    );
    assert_eq!(
        err.iter_errors().map(ToString::to_string).collect::<Vec<_>>(),
        ["boxed error"],
        "error iteration must yield the boxed error only once"
    );
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

#[test]
fn equality_with_strings_uses_the_root_errors_display() {
    let exn = message("cause").raise().raise(message("failure"));
    assert_eq!(exn, "failure");
    assert_eq!(exn, String::from("failure"));
    assert_eq!(&exn, "failure");
    assert_ne!(exn, "cause", "children aren't compared");

    let error = Error::from(exn);
    assert_eq!(error, "failure");
    assert_eq!(&error, "failure");
    assert_eq!(error, String::from("failure"));
    assert_ne!(error, "other");

    let nested = Error::from(message("nested cause").raise().raise(message("nested root"))).raise_erased();
    assert_eq!(nested, "nested root", "nested error boundaries are transparent");
    assert_eq!(Error::from(nested), "nested root");
}
