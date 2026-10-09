#![cfg(feature = "anyhow")]

use gix_error::{Exn, Message, message};

#[test]
fn typed_and_erased_exceptions_propagate_with_the_complete_chain() {
    fn exception() -> Exn<Message> {
        Exn::raise_all([message("left"), message("right")], message("root"))
            .chain(std::io::Error::from(std::io::ErrorKind::TimedOut))
    }

    fn propagate<E: std::error::Error + Send + Sync + 'static>(error: Exn<E>) -> anyhow::Result<()> {
        Err(error)?
    }

    for result in [propagate(exception()), propagate(exception().erased())] {
        let error = result.expect_err("both exception types propagate directly into anyhow");
        assert_eq!(
            error
                .chain()
                .map(|cause| {
                    cause
                        .to_string()
                        .split(", at ")
                        .next()
                        .expect("every diagnostic has a display string")
                        .to_owned()
                })
                .collect::<Vec<_>>(),
            ["root", "left", "right", "timed out"],
            "conversion retains every branch and concrete I/O diagnostic in breadth-first order"
        );
        assert!(
            error.chain().all(|cause| cause.to_string().contains(", at ")),
            "every explicitly raised frame retains its caller location"
        );
        let report = format!("{error:?}");
        for diagnostic in ["root", "left", "right", "timed out"] {
            assert_eq!(
                report.matches(diagnostic).count(),
                1,
                "anyhow reports each diagnostic exactly once"
            );
        }
    }
}

#[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
#[test]
fn public_error_wrapped_in_anyhow_prints_the_complete_chain() {
    fn propagate() -> anyhow::Result<()> {
        Err(Exn::raise_all([message("left"), message("right")], message("root"))
            .chain(std::io::Error::from(std::io::ErrorKind::TimedOut))
            .into_error())?
    }

    let error = propagate().expect_err("public errors propagate directly into anyhow");
    assert!(
        error.downcast_ref::<gix_error::Error>().is_some(),
        "anyhow retains the public gix error type"
    );
    let causes = error.chain().map(ToString::to_string).collect::<Vec<_>>();
    assert_eq!(causes.len(), 4, "anyhow exposes the root and every branch exactly once");
    for (cause, diagnostic) in causes.iter().zip(["root", "left", "right", "timed out"]) {
        assert!(
            cause.starts_with(&format!("{diagnostic}, at ")),
            "each source retains its diagnostic and caller location in breadth-first order: {cause}"
        );
    }
    assert_eq!(error.to_string(), causes[0], "normal Display prints only the root");
    assert_eq!(
        format!("{error:#}"),
        causes.join(": "),
        "alternate Display prints every source once, retaining its caller location"
    );

    let report = format!("{error:?}");
    let report = report
        .split("\n\nStack backtrace:")
        .next()
        .expect("the error report is present");
    assert_eq!(
        report.matches(", at ").count(),
        4,
        "Debug retains every frame's caller location"
    );
    let report = report
        .lines()
        .map(|line| line.split_once(", at ").map_or(line, |(diagnostic, _)| diagnostic))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(report, "anyhow Debug traverses the public error's source chain without duplication", @"
    root

    Caused by:
        0: left
        1: right
        2: timed out
    ");
    insta::assert_snapshot!(format!("{error:#?}"), "alternate anyhow Debug delegates to the public error's complete report without locations", @"
    root

    Caused by:
        0: left
        1: right
        2: timed out
    ");
}

#[test]
fn classification_markers_do_not_become_anyhow_causes() {
    use gix_error::{Class, ClassificationMarker, ErrorExt, tag};

    #[derive(Debug)]
    struct InvalidUsername;

    impl std::fmt::Display for InvalidUsername {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("invalid username")
        }
    }

    impl std::error::Error for InvalidUsername {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(const { &ClassificationMarker::VALIDATION })
        }
    }

    for public in [false, true] {
        if public && !cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
            continue;
        }
        let cases = [
            (
                InvalidUsername.raise_erased().raise(message("outer")).erased(),
                vec!["outer", "invalid username"],
            ),
            (
                tag(InvalidUsername, Class::Retryable).raise_erased(),
                vec!["invalid username"],
            ),
            (
                ClassificationMarker::CONFLICT.raise_erased().chain(InvalidUsername),
                vec!["invalid username"],
            ),
            (
                message("outer")
                    .raise_erased()
                    .chain(tag(InvalidUsername, Class::Retryable))
                    .chain(ClassificationMarker::CONFLICT)
                    .chain(message("sibling")),
                vec!["outer", "sibling", "invalid username"],
            ),
        ];
        for (exception, expected) in cases {
            assert!(
                exception.is_validation(),
                "the native marker classifies the original error"
            );
            let error = if public {
                anyhow::Error::new(exception.into_error())
            } else {
                anyhow::Error::from(exception)
            };
            let causes = error
                .chain()
                .map(|cause| {
                    cause
                        .to_string()
                        .split(", at ")
                        .next()
                        .expect("a diagnostic is present")
                        .to_owned()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                causes, expected,
                "anyhow retains real diagnostics without classification markers"
            );
            let report = format!("{error:?}");
            for diagnostic in expected {
                assert_eq!(
                    report.matches(diagnostic).count(),
                    1,
                    "anyhow prints each real diagnostic once"
                );
            }
            assert!(
                error.chain().all(|cause| cause.to_string().contains(", at ")),
                "real diagnostics retain their caller locations"
            );
        }
    }
}

#[test]
fn marker_only_anyhow_errors_retain_the_root_fallback() {
    use gix_error::{ClassificationMarker, ErrorExt};

    let error = anyhow::Error::from(
        ClassificationMarker::VALIDATION
            .raise_typed()
            .chain(ClassificationMarker::CONFLICT),
    );
    assert!(
        error.to_string().starts_with("Validation, at "),
        "a marker-only error retains its root fallback"
    );
    assert_eq!(error.chain().count(), 1, "marker-only errors have no extra causes");
}

#[test]
fn nested_marker_only_error_boundaries_do_not_become_causes() {
    use gix_error::{ClassificationMarker, Error, ErrorExt};

    for public in [false, true] {
        if public && !cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
            continue;
        }
        for nested in [
            Error::from_error(ClassificationMarker::VALIDATION),
            Error::from_error(Error::from_error(ClassificationMarker::VALIDATION)),
        ] {
            let exception = message("root").raise_typed().chain(nested).chain(message("sibling"));
            assert!(exception.is_validation(), "nested markers still classify the exception");
            let error = if public {
                let error = exception.into_error();
                assert!(error.is_validation(), "chain conversion retains nested classifications");
                anyhow::Error::new(error)
            } else {
                anyhow::Error::new(exception.into_chain())
            };
            let causes = error
                .chain()
                .map(|cause| {
                    cause
                        .to_string()
                        .split(", at ")
                        .next()
                        .expect("a diagnostic is present")
                        .to_owned()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                causes,
                ["root", "sibling"],
                "source traversal skips marker-only boundaries without losing subsequent frames"
            );
        }
    }

    let nested = ClassificationMarker::VALIDATION
        .raise_typed()
        .chain(message("detail"))
        .into_error();
    let error = anyhow::Error::new(message("root").raise_typed().chain(nested).into_chain());
    assert!(
        error.chain().any(|cause| cause.to_string().contains("detail")),
        "marker-rooted boundaries with real descendants retain their diagnostics"
    );
}

#[test]
fn leading_nested_markers_promote_the_first_real_diagnostic() {
    use gix_error::{ClassificationMarker, Error, ErrorExt};

    for public in [false, true] {
        if public && !cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
            continue;
        }
        for nested in [
            Error::from_error(ClassificationMarker::VALIDATION),
            Error::from_error(Error::from_error(ClassificationMarker::VALIDATION)),
        ] {
            let exception = nested.raise_erased().chain(message("real"));
            let error = if public {
                let error = exception.into_error();
                assert!(error.is_validation(), "root promotion retains stored classifications");
                anyhow::Error::new(error)
            } else {
                anyhow::Error::from(exception)
            };
            assert!(
                error.to_string().starts_with("real, at "),
                "a leading marker-only boundary does not become the root diagnostic"
            );
            assert_eq!(error.chain().count(), 1, "the promoted diagnostic appears only once");
        }
    }

    let error = anyhow::Error::from(Error::from_error(ClassificationMarker::VALIDATION).raise_erased());
    assert!(
        error.to_string().starts_with("Validation, at "),
        "a nested marker-only error retains its root fallback when no diagnostic exists"
    );
    assert_eq!(error.chain().count(), 1, "the nested fallback has no extra causes");
}
