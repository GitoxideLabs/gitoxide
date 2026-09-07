use gix_error::{CorruptionError, Error, ErrorExt, NotFoundError, RetryableError, ValidationError, message};
#[cfg(not(feature = "tree-error"))]
use gix_error::{Exn, IteratorExt, Message};
use std::error::Error as _;

#[test]
fn exn_converts_to_boxed_std_error() {
    let err: Box<dyn std::error::Error + Send + Sync> = message("one").raise().into();
    let err = Error::from_boxed(err);
    assert_eq!(err.probable_cause().to_string(), "one");
}

#[test]
fn erased_validation_error_remains_classified() {
    let native: gix_error::Exn = ValidationError::new("invalid").into();
    let err = Error::from(native);
    assert!(
        err.is_validation(),
        "the auto-chain Error classifies the original ValidationError retained during ChainedError construction"
    );
}

#[cfg(not(feature = "tree-error"))]
#[test]
fn from_exn_error() {
    let err = Error::from(message("one").raise());
    assert_eq!(format!("{err:#}"), "one");
    insta::assert_compact_debug_snapshot!(
        &err,
        "compact Debug exposes the underlying message without caller location",
        @r#"Message("one")"#
    );
    insta::assert_debug_snapshot!(err, @r#"
    Message(
        "one",
    )
    "#);
    assert_eq!(err.source().map(debug_string), None);
}

#[cfg(not(feature = "tree-error"))]
#[test]
fn from_exn_error_tree() {
    let err = Error::from(new_tree_error().raise(message("topmost")));
    assert_eq!(format!("{err:#}").to_string(), "topmost");
    insta::assert_compact_debug_snapshot!(
        err,
        "compact Debug shows only the topmost error after flattening",
        @r#"Message("topmost")"#
    );
    insta::assert_debug_snapshot!(err, "pretty Debug shows only the topmost error after flattening", @r#"
    Message(
        "topmost",
    )
    "#);
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
        "topmost, at gix-error/tests/auto_chain_error.rs:44",
        "E6, at gix-error/tests/auto_chain_error.rs:167",
        "E5, at gix-error/tests/auto_chain_error.rs:159",
        "E4, at gix-error/tests/auto_chain_error.rs:162",
        "E8, at gix-error/tests/auto_chain_error.rs:165",
        "E3, at gix-error/tests/auto_chain_error.rs:151",
        "E10, at gix-error/tests/auto_chain_error.rs:154",
        "E12, at gix-error/tests/auto_chain_error.rs:157",
        "E2, at gix-error/tests/auto_chain_error.rs:161",
        "E7, at gix-error/tests/auto_chain_error.rs:164",
        "E1, at gix-error/tests/auto_chain_error.rs:150",
        "E9, at gix-error/tests/auto_chain_error.rs:153",
        "E11, at gix-error/tests/auto_chain_error.rs:156",
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
        format!("{:#}", err.probable_cause()),
        "E6",
        "we get the top-most error that has most causes"
    );
}

#[test]
fn from_any_error() {
    let err = Error::from_error(message("one"));
    assert_eq!(format!("{err:#}"), "one");
    assert_eq!(debug_string(&err), r#"Message("one")"#);
    insta::assert_debug_snapshot!(err, @r#"
    Message(
        "one",
    )
    "#);
    assert_eq!(err.source().map(debug_string), None);
    assert_eq!(format!("{:#}", err.probable_cause()), "one");
}

#[test]
fn probable_cause_survives_tree_flattening() {
    let err = Error::from(message("bottom").raise().raise(message("middle")).raise(message("top")));
    assert_eq!(format!("{:#}", err.probable_cause()), "bottom");
}

#[cfg(not(feature = "tree-error"))]
pub fn new_tree_error() -> Exn<Message> {
    let e1 = message("E1").raise();
    let e3 = e1.raise(message("E3"));

    let e9 = message("E9").raise();
    let e10 = e9.raise(message("E10"));

    let e11 = message("E11").raise();
    let e12 = e11.raise(message("E12"));

    let e5 = [e3, e10, e12].into_iter().raise(message("E5"));

    let e2 = message("E2").raise();
    let e4 = e2.raise(message("E4"));

    let e7 = message("E7").raise();
    let e8 = e7.raise(message("E8"));

    [e5, e4, e8].into_iter().raise(message("E6"))
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
        .raise()
        .raise(message("network operation failed"));
    assert!(Error::from(retryable).can_retry());

    let dependency_specific = RetryableError::new(message("HTTP/2 stream failed"))
        .raise()
        .raise(message("network operation failed"));
    assert!(Error::from(dependency_specific).can_retry());
}

#[test]
fn corruption_is_discovered_in_the_error_chain() {
    let corrupt = CorruptionError::new("checksum mismatch")
        .raise()
        .raise(message("failed to open object database"));
    assert!(Error::from(corrupt).is_corrupted());
}

#[test]
fn not_found_is_discovered_in_well_known_errors() {
    let missing = NotFoundError::new("reference does not exist")
        .raise()
        .raise(message("failed to resolve HEAD"));
    assert!(Error::from(missing).is_not_found());
    assert!(Error::from_error(std::io::Error::new(std::io::ErrorKind::NotFound, "missing")).is_not_found());
    assert!(
        Error::from_boxed(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "missing object"
        )))
        .is_not_found()
    );
}

#[test]
fn validation_is_discovered_in_the_error_chain() {
    assert!(Error::from_error(ValidationError::new("invalid")).is_validation());
    assert!(Error::from_error(ErrorWithSource(ValidationError::new("invalid"))).is_validation());

    let err = Error::from(ValidationError::new("typed").raise().raise(message("context")));
    assert!(
        err.iter_errors().any(<dyn std::error::Error>::is::<ValidationError>),
        "iter_errors() exposes the stored error types in chain mode"
    );
    assert!(
        err.iter_errors_with_locations()
            .any(|source| source.error().is::<ValidationError>()),
        "iter_errors_with_locations() preserves the stored error types alongside their locations"
    );
}

#[test]
fn classification_survives_raising_a_converted_error() {
    let converted = Error::from_error(ErrorWithSource(ValidationError::new("invalid object header")));
    let raised = Error::from(converted.raise().raise(message("revision parsing failed")));
    assert!(raised.is_validation());
}

#[test]
#[cfg(not(feature = "tree-error"))]
fn raising_a_converted_error_preserves_stored_types() {
    let converted = Error::from(
        ValidationError::new("invalid object header")
            .raise()
            .raise(message("object lookup failed")),
    );
    let converted = Error::from_error(converted);
    let raised = converted.raise().raise(message("revision parsing failed"));
    let raised = Error::from(raised);

    assert!(
        raised.iter_errors().any(<dyn std::error::Error>::is::<ValidationError>),
        "the nested Error retains its typed frames"
    );
    assert!(
        raised
            .iter_errors_with_locations()
            .any(|source| source.error().is::<ValidationError>()),
        "iter_errors_with_locations() recursively exposes typed errors from nested Error values"
    );
    assert!(
        raised.probable_cause().is::<ValidationError>(),
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
