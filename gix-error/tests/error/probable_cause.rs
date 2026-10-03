use std::error::Error as StdError;

use gix_error::{
    Class, ClassificationMarker, Error, ErrorExt, Exn, Message, ResourceExhaustionKind, TestError, corruption, message,
    not_found, resource_exhaustion, validation,
};

use crate::ErrorWithSource;

const VALIDATION: ClassificationMarker = ClassificationMarker::VALIDATION;
const NOT_FOUND: ClassificationMarker = ClassificationMarker::NOT_FOUND;

fn check<T: StdError + 'static, E: StdError + Send + Sync + 'static>(exn: Exn<E>) -> impl std::fmt::Debug {
    let report = gix_testtools::redact_debug_snapshot(&exn, &[]);
    let expected = format!("{:#}", exn.probable_cause());
    let assert_cause = |cause: &(dyn StdError + 'static)| {
        assert!(
            cause.is::<T>(),
            "selection retains the expected concrete cause: {cause:#}"
        );
        assert_eq!(
            format!("{cause:#}"),
            expected,
            "conversions and added context preserve the selected diagnostic"
        );
    };
    assert_cause(exn.probable_cause());
    assert_cause(exn.frame().probable_cause().unwrap_or_else(|| exn.frame().error()));

    let exn = exn.erased().erased();
    assert_cause(exn.probable_cause());
    let frame: gix_error::exn::Frame = exn.into();
    assert_cause(frame.probable_cause().unwrap_or_else(|| frame.error()));
    let exn = Exn::from(frame);
    assert_cause(exn.probable_cause());

    let error = exn.into_error();
    assert_cause(error.probable_cause());
    let error = Error::from_boxed(Box::new(error));
    assert_cause(error.probable_cause());
    let contextual = error.and_raise_typed(message("additional context"));
    assert_cause(contextual.probable_cause());
    assert_cause(contextual.into_error().probable_cause());
    (report, expected)
}

fn aggregate(reverse: bool) -> Exn<Message> {
    let children = if reverse { ["right", "left"] } else { ["left", "right"] };
    Exn::raise_all(children.map(|child| message(child).raise_typed()), message("aggregate"))
}

#[test]
fn meaningful_native_sources_remain_selectable() {
    let diagnostics = vec![
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(
                ErrorWithSource("operation", ErrorWithSource("decoder", message("bad byte"))).raise_typed(),
            ),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(ErrorWithSource("decode failed", validation("invalid object header")).raise_typed()),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(ErrorWithSource("lookup failed", not_found("reference is missing")).raise_typed()),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(ErrorWithSource("read failed", corruption("checksum mismatch")).raise_typed()),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(
                ErrorWithSource(
                    "allocation failed",
                    resource_exhaustion(ResourceExhaustionKind::AllocationLimit, "object exceeds limit"),
                )
                .raise_typed(),
            ),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(
                ErrorWithSource(
                    "request failed",
                    ClassificationMarker::with_source(Class::Retryable, message("connection reset")),
                )
                .raise_typed(),
            ),
            &[],
        ),
    ];
    insta::assert_debug_snapshot!(diagnostics, "meaningful native sources remain selectable", @r#"
    [
        (
            operation
            
            Caused by:
                0: decoder
                1: bad byte,
            "bad byte",
        ),
        (
            decode failed
            
            Caused by:
                0: invalid object header,
            "invalid object header",
        ),
        (
            lookup failed
            
            Caused by:
                0: reference is missing,
            "reference is missing",
        ),
        (
            read failed
            
            Caused by:
                0: checksum mismatch,
            "checksum mismatch",
        ),
        (
            allocation failed
            
            Caused by:
                0: object exceeds limit,
            "object exceeds limit",
        ),
        (
            request failed
            
            Caused by:
                0: connection reset,
            "connection reset",
        ),
    ]
    "#);
}

#[test]
fn genuine_classified_children_are_not_metadata() {
    let diagnostics = vec![
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(message("context").raise_typed().chain(validation("invalid input"))),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(
                message("aggregate")
                    .raise_typed()
                    .chain(validation("invalid input"))
                    .chain(not_found("missing input")),
            ),
            &[],
        ),
    ];
    insta::assert_debug_snapshot!(diagnostics, "genuine classified children are not metadata", @r#"
    [
        (
            context
            
            Caused by:
                0: invalid input,
            "invalid input",
        ),
        (
            aggregate
            
            Caused by:
                0: invalid input
                1: missing input,
            "aggregate",
        ),
    ]
    "#);
}

#[test]
fn native_markers_do_not_replace_their_diagnostic_owner() {
    let diagnostics = vec![
        gix_testtools::redact_debug_snapshot(
            &check::<ErrorWithSource<ClassificationMarker>, _>(
                ErrorWithSource("invalid object 42", VALIDATION).raise_typed(),
            ),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<ErrorWithSource<ClassificationMarker>, _>(
                ErrorWithSource("missing reference HEAD", NOT_FOUND).and_raise_typed(message("resolve revision")),
            ),
            &[],
        ),
    ];
    insta::assert_debug_snapshot!(diagnostics, "native markers do not replace their diagnostic owner", @r#"
    [
        (
            invalid object 42,
            "invalid object 42",
        ),
        (
            resolve revision
            
            Caused by:
                0: missing reference HEAD,
            "missing reference HEAD",
        ),
    ]
    "#);
}

#[test]
fn explicit_markers_do_not_replace_a_real_cause_or_create_a_branch() {
    let mut diagnostics = Vec::new();
    diagnostics.push(gix_testtools::redact_debug_snapshot(
        &check::<Message, _>(message("specific diagnostic").raise_typed().chain(VALIDATION)),
        &[],
    ));
    diagnostics.push(gix_testtools::redact_debug_snapshot(
        &check::<Message, _>(
            message("specific diagnostic")
                .raise_typed()
                .chain(VALIDATION)
                .chain(NOT_FOUND),
        ),
        &[],
    ));
    for marker_first in [false, true] {
        let exn = message("context").raise_typed();
        let exn = if marker_first {
            exn.chain(VALIDATION).chain(message("real cause"))
        } else {
            exn.chain(message("real cause")).chain(VALIDATION)
        };
        diagnostics.push(gix_testtools::redact_debug_snapshot(&check::<Message, _>(exn), &[]));
    }
    diagnostics.push(gix_testtools::redact_debug_snapshot(
        &check::<Message, _>(
            ErrorWithSource("decode failed", validation("invalid header"))
                .raise_typed()
                .chain(VALIDATION),
        ),
        &[],
    ));
    diagnostics.push(gix_testtools::redact_debug_snapshot(
        &check::<Message, _>(
            ErrorWithSource("context", VALIDATION)
                .raise_typed()
                .chain(message("real cause")),
        ),
        &[],
    ));
    insta::assert_debug_snapshot!(diagnostics, "explicit markers do not replace a real cause or create a branch", @r#"
    [
        (
            specific diagnostic,
            "specific diagnostic",
        ),
        (
            specific diagnostic,
            "specific diagnostic",
        ),
        (
            context
            
            Caused by:
                0: real cause,
            "real cause",
        ),
        (
            context
            
            Caused by:
                0: real cause,
            "real cause",
        ),
        (
            decode failed
            
            Caused by:
                0: invalid header,
            "invalid header",
        ),
        (
            context
            
            Caused by:
                0: real cause,
            "real cause",
        ),
    ]
    "#);
}

#[test]
fn flat_aggregates_are_invariant_under_context_and_sibling_order() {
    let mut diagnostics = Vec::new();
    for reverse in [false, true] {
        for depth in 0..3 {
            let mut exn = aggregate(reverse);
            for _ in 0..depth {
                exn = exn.raise(message("outer context"));
            }
            diagnostics.push(gix_testtools::redact_debug_snapshot(&check::<Message, _>(exn), &[]));
        }
    }
    insta::assert_debug_snapshot!(diagnostics, "flat aggregates are invariant under context and sibling order", @r#"
    [
        (
            aggregate
            
            Caused by:
                0: left
                1: right,
            "aggregate",
        ),
        (
            outer context
            
            Caused by:
                0: aggregate
                ├─0: left
                └─1: right,
            "aggregate",
        ),
        (
            outer context
            
            Caused by:
                0: outer context
                1: aggregate
                ├─0: left
                └─1: right,
            "aggregate",
        ),
        (
            aggregate
            
            Caused by:
                0: right
                1: left,
            "aggregate",
        ),
        (
            outer context
            
            Caused by:
                0: aggregate
                ├─0: right
                └─1: left,
            "aggregate",
        ),
        (
            outer context
            
            Caused by:
                0: outer context
                1: aggregate
                ├─0: right
                └─1: left,
            "aggregate",
        ),
    ]
    "#);
}

#[test]
fn nested_aggregates_stop_at_the_first_causal_branch() {
    let mut diagnostics = Vec::new();
    for reverse in [false, true] {
        let nested = aggregate(reverse).into_error();
        diagnostics.push(gix_testtools::redact_debug_snapshot(
            // Typed raising deliberately retains the nested Error boundary under test.
            &check::<Message, _>(nested.raise_typed().raise(message("outer context"))),
            &[],
        ));
        diagnostics.push(gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(
                Exn::raise_all(
                    [
                        aggregate(reverse),
                        message("third cause").raise_typed().raise(message("sibling context")),
                    ],
                    message("outer aggregate"),
                )
                .raise(message("outer context")),
            ),
            &[],
        ));
        diagnostics.push(gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(
                aggregate(reverse)
                    .into_error()
                    .raise_typed()
                    .raise(message("outer context"))
                    .chain(VALIDATION),
            ),
            &[],
        ));
    }
    insta::assert_debug_snapshot!(diagnostics, "nested aggregates stop at the first causal branch", @r#"
    [
        (
            outer context
            
            Caused by:
                0: aggregate
                1: left
                2: right,
            "aggregate",
        ),
        (
            outer context
            
            Caused by:
                0: outer aggregate
                ├─0: aggregate
                │ ├─0: left
                │ └─1: right
                └─1: sibling context
                  └─0: third cause,
            "outer aggregate",
        ),
        (
            outer context
            
            Caused by:
                0: aggregate
                1: left
                2: right,
            "aggregate",
        ),
        (
            outer context
            
            Caused by:
                0: aggregate
                1: right
                2: left,
            "aggregate",
        ),
        (
            outer context
            
            Caused by:
                0: outer aggregate
                ├─0: aggregate
                │ ├─0: right
                │ └─1: left
                └─1: sibling context
                  └─0: third cause,
            "outer aggregate",
        ),
        (
            outer context
            
            Caused by:
                0: aggregate
                1: right
                2: left,
            "aggregate",
        ),
    ]
    "#);
}

#[test]
fn nested_error_boundaries_and_explicit_children_both_count() {
    let mut diagnostics = Vec::new();
    for with_context in [false, true] {
        let nested = aggregate(false).into_error();
        let exn = nested.raise_typed().chain(message("explicit sibling"));
        assert!(
            exn.frame().probable_cause().is_none(),
            "the stored boundary itself is selected when its nested graph and explicit child form a branch"
        );
        assert!(
            std::ptr::eq(exn.probable_cause(), exn.frame().error()),
            "selecting a boundary at a branch must not recursively replace it with one nested cause"
        );
        if with_context {
            diagnostics.push(gix_testtools::redact_debug_snapshot(
                &check::<Error, _>(exn.raise(message("outer context"))),
                &[],
            ));
        } else {
            diagnostics.push(gix_testtools::redact_debug_snapshot(&check::<Error, _>(exn), &[]));
        }
    }
    diagnostics.push(gix_testtools::redact_debug_snapshot(
        &check::<ErrorWithSource<Error>, _>(
            ErrorWithSource("native aggregate", aggregate(false).into_error())
                .raise_typed()
                .chain(message("explicit sibling")),
        ),
        &[],
    ));
    diagnostics.push(gix_testtools::redact_debug_snapshot(
        &check::<Message, _>(
            Error::from_error(validation("nested real cause"))
                .raise_typed()
                .chain(VALIDATION),
        ),
        &[],
    ));
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(diagnostics, "nested error boundaries and explicit children both count", @r#"
        [
            (
                aggregate
                
                Caused by:
                    0: left
                    1: right
                    2: explicit sibling,
                "aggregate: left: right",
            ),
            (
                outer context
                
                Caused by:
                    0: aggregate
                    ├─0: left
                    ├─1: right
                    └─2: explicit sibling,
                "aggregate: left: right",
            ),
            (
                native aggregate
                
                Caused by:
                    0: aggregate
                    ├─0: left
                    └─1: right
                    1: explicit sibling,
                "native aggregate",
            ),
            (
                nested real cause,
                "nested real cause",
            ),
        ]
        "#);
    } else {
        insta::assert_debug_snapshot!(diagnostics, "nested error boundaries and explicit children both count", @r#"
        [
            (
                aggregate
                
                Caused by:
                    0: left
                    1: right
                    2: explicit sibling,
                "Message { message: \"aggregate\" }\n\nCaused by:\n    0: Message { message: \"left\" }\n    1: Message { message: \"right\" }",
            ),
            (
                outer context
                
                Caused by:
                    0: aggregate
                    ├─0: left
                    ├─1: right
                    └─2: explicit sibling,
                "Message { message: \"aggregate\" }\n\nCaused by:\n    0: Message { message: \"left\" }\n    1: Message { message: \"right\" }",
            ),
            (
                native aggregate
                
                Caused by:
                    0: aggregate
                    ├─0: left
                    └─1: right
                    1: explicit sibling,
                "native aggregate",
            ),
            (
                nested real cause,
                "nested real cause",
            ),
        ]
        "#);
    }
}

#[test]
fn marker_frames_are_transparent_without_losing_real_descendants() {
    let diagnostics = vec![
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(VALIDATION.raise_typed().chain(validation("real cause"))),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(
                message("context").raise_typed().chain(
                    VALIDATION
                        .raise_typed()
                        .chain(validation("real cause"))
                        .chain(NOT_FOUND)
                        .raise(VALIDATION),
                ),
            ),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(
                message("aggregate")
                    .raise_typed()
                    .chain(VALIDATION.raise_typed().chain(message("left")).chain(message("right"))),
            ),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(
                message("aggregate")
                    .raise_typed()
                    .chain(VALIDATION.raise_typed().chain(message("left")))
                    .chain(message("right")),
            ),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(
                message("specific diagnostic")
                    .raise_typed()
                    .chain(VALIDATION.raise_typed().chain(NOT_FOUND)),
            ),
            &[],
        ),
    ];
    insta::assert_debug_snapshot!(diagnostics, "marker frames are transparent without losing real descendants", @r#"
    [
        (
            real cause,
            "real cause",
        ),
        (
            context
            
            Caused by:
                0: real cause,
            "real cause",
        ),
        (
            aggregate
            
            Caused by:
                0: left
                1: right,
            "aggregate",
        ),
        (
            aggregate
            
            Caused by:
                0: left
                1: right,
            "aggregate",
        ),
        (
            specific diagnostic,
            "specific diagnostic",
        ),
    ]
    "#);
}

#[test]
fn nested_marker_boundaries_are_transparent_without_losing_real_descendants() {
    let diagnostics = vec![
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(
                message("specific diagnostic")
                    .raise_typed()
                    .chain(Error::from_error(Error::from_error(VALIDATION))),
            ),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(
                message("context").raise_typed().chain(
                    VALIDATION
                        .raise_typed()
                        .chain(validation("nested real cause"))
                        .into_error(),
                ),
            ),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<Message, _>(
                message("aggregate").raise_typed().chain(
                    Error::from_error(VALIDATION)
                        .raise_typed()
                        .chain(message("left"))
                        .chain(message("right")),
                ),
            ),
            &[],
        ),
        gix_testtools::redact_debug_snapshot(
            &check::<ErrorWithSource<Error>, _>(
                ErrorWithSource("specific diagnostic", Error::from_error(VALIDATION)).raise_typed(),
            ),
            &[],
        ),
    ];
    insta::assert_debug_snapshot!(diagnostics, "nested marker boundaries are transparent without losing real descendants", @r#"
    [
        (
            specific diagnostic,
            "specific diagnostic",
        ),
        (
            context
            
            Caused by:
                0: nested real cause,
            "nested real cause",
        ),
        (
            aggregate
            
            Caused by:
                0: left
                1: right,
            "aggregate",
        ),
        (
            specific diagnostic,
            "specific diagnostic",
        ),
    ]
    "#);
}

#[test]
fn io_payloads_participate_in_the_logical_graph() {
    let mut diagnostics = Vec::new();
    let io = std::io::Error::from(std::io::ErrorKind::NotFound);
    diagnostics.push(gix_testtools::redact_debug_snapshot(
        &check::<std::io::Error, _>(ErrorWithSource("read failed", io).raise_typed()),
        &[],
    ));
    diagnostics.push(gix_testtools::redact_debug_snapshot(
        &check::<Message, _>(
            std::io::Error::new(std::io::ErrorKind::InvalidData, validation("invalid I/O payload"))
                .and_raise_typed(message("read failed")),
        ),
        &[],
    ));
    diagnostics.push(gix_testtools::redact_debug_snapshot(
        &check::<ErrorWithSource<ClassificationMarker>, _>(
            std::io::Error::other(ErrorWithSource("useful I/O payload", VALIDATION)).raise_typed(),
        ),
        &[],
    ));
    diagnostics.push(gix_testtools::redact_debug_snapshot(
        &check::<std::io::Error, _>(std::io::Error::other(VALIDATION).raise_typed()),
        &[],
    ));
    diagnostics.push(gix_testtools::redact_debug_snapshot(
        &check::<Message, _>(
            std::io::Error::other(aggregate(false).into_error()).and_raise_typed(message("read failed")),
        ),
        &[],
    ));
    let io = std::io::Error::other(aggregate(false).into_error());
    diagnostics.push(gix_testtools::redact_debug_snapshot(
        &check::<std::io::Error, _>(io.raise_typed().chain(message("explicit sibling"))),
        &[],
    ));
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(diagnostics, "io payloads participate in the logical graph", @r#"
        [
            (
                read failed
                
                Caused by:
                    0: entity not found,
                "entity not found",
            ),
            (
                read failed
                
                Caused by:
                    0: I/O error (InvalidData)
                    1: invalid I/O payload,
                "invalid I/O payload",
            ),
            (
                I/O error (Other)
                
                Caused by:
                    0: useful I/O payload,
                "useful I/O payload",
            ),
            (
                I/O error (Other),
                "Validation",
            ),
            (
                read failed
                
                Caused by:
                    0: I/O error (Other)
                    1: aggregate
                    2: left
                    3: right,
                "aggregate",
            ),
            (
                I/O error (Other)
                
                Caused by:
                    0: aggregate
                    ├─0: left
                    └─1: right
                    1: explicit sibling,
                "aggregate: left: right",
            ),
        ]
        "#);
    } else {
        insta::assert_debug_snapshot!(diagnostics, "io payloads participate in the logical graph", @r#"
        [
            (
                read failed
                
                Caused by:
                    0: entity not found,
                "entity not found",
            ),
            (
                read failed
                
                Caused by:
                    0: I/O error (InvalidData)
                    1: invalid I/O payload,
                "invalid I/O payload",
            ),
            (
                I/O error (Other)
                
                Caused by:
                    0: useful I/O payload,
                "useful I/O payload",
            ),
            (
                I/O error (Other),
                "Validation",
            ),
            (
                read failed
                
                Caused by:
                    0: I/O error (Other)
                    1: aggregate
                    2: left
                    3: right,
                "aggregate",
            ),
            (
                I/O error (Other)
                
                Caused by:
                    0: aggregate
                    ├─0: left
                    └─1: right
                    1: explicit sibling,
                "Message { message: \"aggregate\" }\n\nCaused by:\n    0: Message { message: \"left\" }\n    1: Message { message: \"right\" }",
            ),
        ]
        "#);
    }
}

#[test]
fn root_fallback_retains_the_stored_error() {
    let mut diagnostics = Vec::new();
    for exn in [
        message("standalone").raise_erased(),
        VALIDATION.raise_erased(),
        VALIDATION.raise_typed().chain(NOT_FOUND).erased(),
        VALIDATION
            .raise_typed()
            .chain(message("left"))
            .chain(message("right"))
            .erased(),
    ] {
        assert!(
            exn.frame().probable_cause().is_none(),
            "without a unique causal descendant the frame delegates to stored-root fallback"
        );
        assert!(
            std::ptr::eq(exn.probable_cause(), exn.frame().error()),
            "root fallback preserves the original stored error after erasure"
        );
        diagnostics.push(gix_testtools::redact_debug_snapshot(&exn, &[]));
        let error = exn.into_error();
        assert!(
            std::ptr::eq(error.probable_cause(), error.error()),
            "conversion uses the stored root, not a synthetic wrapper, as fallback"
        );
    }
    insta::assert_debug_snapshot!(diagnostics, "root fallback retains the stored error", @"
    [
        standalone,
        Validation,
        Validation,
        left
        right,
    ]
    ");
}

#[test]
fn markers_remain_classified_but_are_hidden_from_inspection() {
    let exn = ErrorWithSource("specific diagnostic", VALIDATION)
        .raise_typed()
        .chain(NOT_FOUND);
    let expected = exn.iter_errors().map(ToString::to_string).collect::<Vec<_>>();
    insta::assert_debug_snapshot!(expected, "public error iteration omits classification-only markers", @r#"
    [
        "specific diagnostic",
    ]
    "#);
    assert!(
        exn.downcast_any_ref::<ClassificationMarker>().is_none(),
        "public downcasts omit classification-only markers"
    );
    assert!(
        exn.error()
            .source()
            .expect("the diagnostic exposes its marker")
            .is::<ClassificationMarker>(),
        "native source access retains classification metadata"
    );
    insta::assert_debug_snapshot!(format_args!("{}", exn.probable_cause()), "cause selection retains the specific diagnostic", @"specific diagnostic");
    assert_eq!(
        exn.classify()
            .map(|classification| classification.class())
            .collect::<Vec<_>>(),
        [Class::Validation, Class::NotFound],
        "cause selection does not consume or hide classification evidence"
    );
    insta::assert_debug_snapshot!(&exn, "diagnostic formatting omits classification-only leaves", @"specific diagnostic");
    let error = exn.into_error();
    assert_eq!(
        error.iter_errors().map(ToString::to_string).collect::<Vec<_>>(),
        expected,
        "conversion preserves diagnostic iteration without classification-only markers"
    );
    assert_eq!(
        error
            .classify()
            .map(|classification| classification.class())
            .collect::<Vec<_>>(),
        [Class::Validation, Class::NotFound],
        "both representations retain marker classifications"
    );
    assert_eq!(
        error
            .iter_errors_with_locations()
            .map(|source| format!("{source:#}"))
            .collect::<Vec<_>>(),
        expected,
        "display sources omit classification-only markers too"
    );
}

#[test]
fn marker_reports_retain_context_without_classification_noise() {
    let exn = ErrorWithSource("specific diagnostic", VALIDATION)
        .and_raise_typed(message("operation failed"))
        .chain(NOT_FOUND);
    insta::assert_debug_snapshot!(&exn, "exception reports preserve context and omit classification markers", @"
    operation failed

    Caused by:
        0: specific diagnostic
    ");
    let error = TestError::from(exn);
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_debug_snapshot!(error, "test reports show the error tree without locations or classification markers", @"
    operation failed

    Caused by:
        0: specific diagnostic
    ");
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    insta::assert_debug_snapshot!(error, "test reports show the error chain without locations or classification markers", @"
    operation failed

    Caused by:
        0: specific diagnostic
    ");
}

#[test]
fn marker_reports_retain_real_children_and_classified_errors() {
    insta::allow_duplicates! {
    for nested in [false, true] {
        let children = VALIDATION
            .raise_typed()
            .chain(validation("invalid input"))
            .chain(not_found("missing resource"));
        let exn = message("operation failed").raise_typed();
        let exn = if nested {
            exn.chain(children.into_error())
        } else {
            exn.chain(children)
        };
        insta::assert_debug_snapshot!(&exn, "marker frames promote their real children in order", @"
        operation failed

        Caused by:
            0: invalid input
            1: missing resource
        ");
        let error = TestError::from(exn);
        #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
        insta::assert_debug_snapshot!(error, "genuine classified errors remain in test error trees", @"
        operation failed

        Caused by:
            0: invalid input
            1: missing resource
        ");
        #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
        insta::assert_debug_snapshot!(error, "genuine classified errors remain in test error chains", @"
        operation failed

        Caused by:
            0: invalid input
            1: missing resource
        ");
    }
    }
}

#[test]
fn marker_reports_fall_back_to_the_root_when_no_diagnostic_exists() {
    let exn = VALIDATION.raise_typed().chain(NOT_FOUND);
    assert_eq!(
        exn.iter_errors().count(),
        0,
        "markers are hidden even without other errors"
    );
    insta::assert_debug_snapshot!(&exn, "a marker-only exception still has a diagnostic", @"Validation");
    insta::assert_debug_snapshot!(TestError::from(exn), "a marker-only test failure still has a diagnostic", @"Validation");
}

#[test]
fn marker_reports_do_not_repeat_promoted_native_sources() {
    let exn = VALIDATION
        .raise_typed()
        .chain(ErrorWithSource("specific diagnostic", message("native detail")))
        .into_error()
        .and_raise_typed(message("operation failed"));
    insta::assert_debug_snapshot!(exn, "a promoted nested boundary emits each native source once", @"
    operation failed

    Caused by:
        0: specific diagnostic
        1: native detail
    ");
}

#[test]
fn marker_reports_promote_a_real_cause_above_a_marker_root() {
    let exn = VALIDATION.raise_typed().chain(validation("invalid input"));
    insta::assert_debug_snapshot!(&exn, "a marker root is transparent to diagnostic rendering", @"invalid input");
    insta::assert_debug_snapshot!(TestError::from(exn), "test reports preserve a real cause beneath a marker root", @"invalid input");
}

#[derive(Debug)]
#[repr(transparent)]
struct Native<E>(E);

impl<E> std::fmt::Display for Native<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("native wrapper")
    }
}

impl<E: StdError + 'static> StdError for Native<E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.0)
    }
}

#[test]
fn native_sources_sharing_their_owners_address_keep_their_origin() {
    let mut diagnostics = Vec::new();
    let exn = Native(Native(validation("same-address cause"))).raise_typed();
    assert_eq!(
        std::ptr::from_ref(exn.error()).cast::<()>(),
        std::ptr::from_ref(&exn.error().0.0).cast::<()>(),
        "the fixture places distinct native sources at their owner's address"
    );
    diagnostics.push(gix_testtools::redact_debug_snapshot(&check::<Message, _>(exn), &[]));
    insta::assert_debug_snapshot!(diagnostics, "native sources sharing their owners address keep their origin", @r#"
    [
        (
            native wrapper
            
            Caused by:
                0: native wrapper
                1: same-address cause,
            "same-address cause",
        ),
    ]
    "#);
}

#[test]
fn selection_does_not_score_descendants_below_a_branch() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Debug)]
    struct CountedSource(Arc<AtomicUsize>);

    impl std::fmt::Display for CountedSource {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("unneeded descendant")
        }
    }

    impl StdError for CountedSource {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            self.0.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let exn = message("aggregate")
        .raise_typed()
        .chain(CountedSource(Arc::clone(&calls)))
        .chain(message("other cause"));
    insta::assert_debug_snapshot!(format_args!("{}", exn.probable_cause()), "a branch selects its aggregate", @"aggregate");
    assert_eq!(
        calls.load(Ordering::Relaxed),
        0,
        "cause selection does not inspect branch descendants"
    );
    insta::assert_debug_snapshot!(exn, "the aggregate remains the cause without inspecting its descendants during selection", @"
    aggregate

    Caused by:
        0: unneeded descendant
        1: other cause
    ");
    let error = exn.into_error();
    calls.store(0, Ordering::Relaxed);
    insta::assert_debug_snapshot!(format_args!("{}", error.probable_cause()), "conversion preserves the branch", @"aggregate");
    assert_eq!(
        calls.load(Ordering::Relaxed),
        0,
        "converted selection does not inspect branch descendants"
    );
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(error, "the aggregate remains the cause without inspecting its descendants during selection", @"
        aggregate

        Caused by:
            0: unneeded descendant
            1: other cause
        ");
    } else {
        insta::assert_debug_snapshot!(error, "the aggregate remains the cause without inspecting its descendants during selection", @"
        aggregate

        Caused by:
            0: unneeded descendant
            1: other cause
        ");
    }
}

#[test]
fn converted_marker_roots_format_their_real_diagnostic() {
    let exn = VALIDATION.raise_typed();
    let exn = exn.chain(message("specific diagnostic"));
    let location = exn.frame().children()[0].location();
    let expected = if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        format!("specific diagnostic, at {}:{}", location.file(), location.line())
    } else {
        "specific diagnostic".into()
    };
    let error = exn.into_error();
    assert_eq!(
        error.to_string(),
        expected,
        "converted display selects the diagnostic and retains its own location"
    );
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_debug_snapshot!(error, "converted tree Debug promotes the diagnostic above its marker root", @"specific diagnostic");
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    insta::assert_debug_snapshot!(error, "converted chain Debug retains the diagnostic's concrete type", @"specific diagnostic");
    insta::assert_debug_snapshot!(format_args!("{}", VALIDATION.raise_typed().chain(message("specific diagnostic"))), "exception display also promotes its first real diagnostic", @"specific diagnostic");
}

#[test]
fn converted_marker_only_errors_retain_their_root_diagnostic() {
    let exn = VALIDATION.raise_typed().chain(NOT_FOUND);
    let location = exn.frame().location();
    let expected = if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        format!("Validation, at {}:{}", location.file(), location.line())
    } else {
        "Validation".into()
    };
    let error = exn.into_error();
    assert_eq!(error.to_string(), expected, "display retains the stored marker root");
    let expected = if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        format!("{VALIDATION:#?}")
    } else {
        "Validation".into()
    };
    assert_eq!(
        format!("{error:#?}"),
        expected,
        "Debug retains the stored marker root when no diagnostic exists"
    );
}

#[test]
fn markers_preserve_typed_diagnostics() {
    let mut diagnostics = Vec::new();
    let exn = ErrorWithSource("operation interrupted", ClassificationMarker::RETRYABLE)
        .and_raise_typed(message("verification failed"));
    insta::assert_debug_snapshot!(&exn, "a class-only marker leaves its typed owner's diagnostic intact", @"
    verification failed

    Caused by:
        0: operation interrupted
    ");
    diagnostics.push(gix_testtools::redact_debug_snapshot(
        &check::<ErrorWithSource<ClassificationMarker>, _>(exn),
        &[],
    ));

    let exn = ClassificationMarker::with_source(Class::Retryable, validation("invalid input"))
        .and_raise_typed(message("verification failed"));
    assert!(
        exn.is_retryable() && exn.is_validation(),
        "the marker adds its classification without replacing the typed error's classification"
    );
    insta::assert_debug_snapshot!(&exn, "a source marker exposes the typed diagnostic exactly once", @"
    verification failed

    Caused by:
        0: invalid input
    ");
    diagnostics.push(gix_testtools::redact_debug_snapshot(&check::<Message, _>(exn), &[]));
    insta::assert_debug_snapshot!(diagnostics, "markers preserve typed diagnostics", @r#"
    [
        (
            verification failed
            
            Caused by:
                0: operation interrupted,
            "operation interrupted",
        ),
        (
            verification failed
            
            Caused by:
                0: invalid input,
            "invalid input",
        ),
    ]
    "#);
}

#[test]
fn source_markers_preserve_nested_aggregates_and_reports() {
    let mut diagnostics = Vec::new();
    let exn = ClassificationMarker::with_source(Class::Retryable, aggregate(false).into_error())
        .and_raise_typed(message("operation failed"));
    assert!(
        exn.is_retryable(),
        "the transparent source wrapper retains its classification"
    );
    insta::assert_debug_snapshot!(&exn, "the source's aggregate and both causes remain visible", @"
    operation failed

    Caused by:
        0: aggregate
        1: left
        2: right
    ");
    diagnostics.push(gix_testtools::redact_debug_snapshot(&check::<Message, _>(exn), &[]));
    insta::assert_debug_snapshot!(diagnostics, "source markers preserve nested aggregates and reports", @r#"
    [
        (
            operation failed
            
            Caused by:
                0: aggregate
                1: left
                2: right,
            "aggregate",
        ),
    ]
    "#);
}

#[test]
fn source_marker_test_reports_do_not_repeat_the_wrapped_error() {
    let marker = ClassificationMarker::with_source(Class::Retryable, message("operation interrupted"));
    insta::assert_debug_snapshot!(&marker, "direct Debug delegates to the source without classification noise", @r#"
    Message {
        message: "operation interrupted",
    }
    "#);
    let exn = marker.raise_typed();
    insta::assert_debug_snapshot!(&exn, "a source-backed root displays the source without its wrapper", @"operation interrupted");
    insta::assert_debug_snapshot!(TestError::from(exn), "test reports display the wrapped diagnostic once", @"operation interrupted");
}

#[test]
fn source_markers_preserve_only_the_first_real_diagnostics_callsite() {
    let exn = ClassificationMarker::with_source(
        Class::Retryable,
        ClassificationMarker::with_source(
            Class::Validation,
            ErrorWithSource("visible diagnostic", message("native tail")),
        ),
    )
    .raise_typed();
    let location = exn.frame().location();
    insta::assert_compact_debug_snapshot!(exn, "the exception report retains the marker callsite on its first real diagnostic", @"
    visible diagnostic, at gix-error/tests/error/probable_cause.rs:1336

    Caused by:
        0: native tail, at gix-error/tests/error/probable_cause.rs:1336
    ");

    let error = exn.into_error();
    {
        let mut diagnostics = error.iter_errors_with_locations();
        let first = diagnostics.next().expect("the wrapped diagnostic remains visible");
        assert!(
            first.error().is::<ErrorWithSource<Message>>(),
            "the source marker wrappers are omitted"
        );
        assert_eq!(
            first.location(),
            Some(location),
            "the first real diagnostic inherits the callsite through both transparent wrappers"
        );
        let tail = diagnostics.next().expect("the native tail remains visible");
        insta::assert_debug_snapshot!(format_args!("{}", tail.error()), "source traversal is preserved", @"native tail");
        assert!(
            tail.location().is_none(),
            "ordinary native sources do not inherit their diagnostic owner's callsite"
        );
        assert!(diagnostics.next().is_none(), "each real diagnostic is yielded once");
    }
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_compact_debug_snapshot!(TestError::from(error), "normal test reports retain the original callsite after conversion", @"
        visible diagnostic, at gix-error/tests/error/probable_cause.rs:1336

        Caused by:
            0: native tail
        ");
    } else {
        insta::assert_compact_debug_snapshot!(TestError::from(error), "normal test reports retain the original callsite after conversion", @"
        visible diagnostic, at gix-error/tests/error/probable_cause.rs:1336

        Caused by:
            0: native tail, at gix-error/tests/error/probable_cause.rs:1336
        ");
    }
}
