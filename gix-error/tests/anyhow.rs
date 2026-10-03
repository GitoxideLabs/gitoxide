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
