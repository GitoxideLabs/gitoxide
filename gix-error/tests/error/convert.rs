#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
use gix_error::OptionExt;
use gix_error::{Error, ErrorExt, Exn, Result, ResultExt, message, not_found, validation};

#[test]
fn public_error_round_trip_preserves_frames_and_native_sources() {
    let native = std::io::Error::new(std::io::ErrorKind::TimedOut, not_found("payload").with("id", 42));
    let err = Exn::raise_all(
        [
            native.raise().erased(),
            validation("child").raise().raise(message("branch")).erased(),
            message("nested leaf").raise().into_error().raise().erased(),
        ],
        message("root"),
    )
    .erased();
    let frames: Vec<_> = err
        .iter()
        .map(|frame| (frame.error().to_string(), frame.location()))
        .collect();
    let mut public = err.into_error();
    let before: Vec<_> = public
        .iter_errors_with_locations()
        .map(|source| (source.error().to_string(), source.location()))
        .collect();
    for _ in 0..2 {
        let err = public.into_exn();
        assert_eq!(
            err.iter()
                .map(|frame| (frame.error().to_string(), frame.location()))
                .collect::<Vec<_>>(),
            frames,
            "native sources never become explicit frames"
        );
        assert_eq!(
            err.downcast_any_ref::<std::io::Error>()
                .expect("the native error remains available")
                .kind(),
            std::io::ErrorKind::TimedOut,
            "concrete error types survive conversion in either representation"
        );
        assert!(err.can_retry(), "retry detection survives conversion");
        assert!(err.is_not_found(), "I/O payload classifications survive conversion");
        assert_eq!(err.metadata().count(), 1, "payload metadata is retained exactly once");
        assert_eq!(
            err.probable_cause().to_string(),
            "root",
            "the aggregate remains the probable cause"
        );
        public = err.into_error();
        assert_eq!(
            public
                .iter_errors_with_locations()
                .map(|source| (source.error().to_string(), source.location()))
                .collect::<Vec<_>>(),
            before,
            "round trips retain error order, native sources, and caller locations"
        );
    }
    let children: Vec<_> = public.into_exn().drain_children().collect();
    assert_eq!(
        children.len(),
        3,
        "explicit children can be rearranged after crossing the boundary"
    );
    assert_eq!(
        children[1].frame().children().len(),
        1,
        "nested frame relationships are retained"
    );
}

#[test]
fn concrete_public_error_retains_its_source() {
    let err = gix_error::Error::from_error(crate::ErrorWithSource("outer", validation("inner")));
    let err = err.into_exn();
    assert!(
        err.frame().children().is_empty(),
        "native sources stay borrowed from their owner"
    );
    assert!(
        err.downcast_any_ref::<crate::ErrorWithSource>().is_some(),
        "the concrete outer error is retained"
    );
    assert_eq!(
        err.probable_cause().to_string(),
        "inner",
        "the native source remains the probable cause"
    );
}

#[test]
fn standard_sources_survive_wrapping_public_errors() {
    let failed: Result = Err(message("checksum mismatch").raise().into());
    let err = failed
        .or_raise(|| message("verification failed"))
        .expect_err("verification retains the failed checksum")
        .into_error();
    let failed: Result = Err(err);
    let err = Error::from_error(
        failed
            .or_erased()
            .expect_err("internal erasure retains the failure")
            .into_error(),
    );
    let source = std::error::Error::source(&err).expect("standard error chains cross public error wrappers");
    assert!(
        source.to_string().starts_with("checksum mismatch"),
        "the original cause remains visible through native source traversal"
    );
    assert!(source.source().is_none(), "the cause is not duplicated");
}

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
#[test]
fn adding_context_reuses_public_error_frames() {
    for add_context in [
        |error: Error| error.and_raise(message("context")).erased(),
        |error: Error| {
            Err::<(), _>(error)
                .or_raise(|| message("context"))
                .expect_err("the failed result gains context")
                .erased()
        },
        |error: Error| {
            Err::<(), _>(error)
                .or_raise_erased(|| message("context"))
                .expect_err("the failed result gains erased context")
        },
    ] {
        let native = crate::ErrorWithSource("native", not_found("missing").with("path", "HEAD"));
        let original = native
            .raise()
            .chain(validation("invalid").with("input", b"bad".as_slice()))
            .raise(message("aggregate").with("operation", "read"));
        let original_error = std::ptr::from_ref(original.error());
        let frames: Vec<_> = original
            .iter()
            .map(|frame| (frame.error().to_string(), frame.location()))
            .collect();

        let error = add_context(original.into_error());
        assert_eq!(
            error.frame().location().file(),
            file!(),
            "the new context records its caller, not a shared helper"
        );
        let child = &error.frame().children()[0];
        assert!(
            std::ptr::eq(
                child
                    .error()
                    .downcast_ref::<gix_error::Message>()
                    .expect("the original root is stored directly"),
                original_error,
            ),
            "adding context keeps the original diagnostic allocation"
        );
        assert_eq!(
            child
                .iter_frames()
                .map(|frame| (frame.error().to_string(), frame.location()))
                .collect::<Vec<_>>(),
            frames,
            "all explicit frames keep their order and original caller locations"
        );
        assert_eq!(
            child.children()[0].children().len(),
            1,
            "the original branch is retained"
        );
        assert!(
            error.downcast_any_ref::<crate::ErrorWithSource>().is_some(),
            "native recovery errors remain accessible"
        );
        assert!(
            error.is_not_found() && error.is_validation(),
            "native and explicit causes retain their classifications"
        );
        assert_eq!(
            error.metadata().count(),
            3,
            "each original metadata dictionary is retained once"
        );
        assert_eq!(
            error.probable_cause().to_string(),
            "native",
            "cause selection stops at the original branch"
        );
    }
}

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
#[test]
fn erasing_public_errors_reuses_their_frames() {
    for erase in [
        |error: Error| error.raise_erased(),
        |error: Error| {
            Err::<(), _>(error)
                .or_erased()
                .expect_err("the failed result is erased")
        },
        |error: Error| {
            None::<()>
                .ok_or_raise_erased(|| error)
                .expect_err("the missing value raises the existing error")
        },
        Error::into_exn,
    ] {
        let original = validation("invalid").with("input", b"bad".as_slice()).raise();
        let frame = std::ptr::from_ref(original.frame());
        let location = original.frame().location();
        let error = erase(original.into_error());
        assert!(
            std::ptr::eq(error.frame(), frame),
            "erasure reuses the owned frame allocation"
        );
        assert_eq!(
            error.frame().location(),
            location,
            "erasure preserves the original raise site"
        );
        assert!(
            error.frame().error().is::<gix_error::Message>(),
            "erasure does not introduce an Error wrapper"
        );
        assert!(error.is_validation(), "the original classification is retained");
        assert_eq!(error.metadata().count(), 1, "the original metadata is retained once");
        assert_eq!(
            error.error().to_string(),
            error.frame().error().to_string(),
            "the erased typed accessor remains valid"
        );
    }
}

#[test]
fn raising_a_public_error_with_its_type_keeps_the_wrapper() {
    let error = validation("invalid").raise().into_error();
    let raised: Exn<Error> = error.raise();
    assert!(
        raised.frame().error().is::<Error>(),
        "typed construction keeps the promised Error payload"
    );
    assert!(
        raised.error().is_validation(),
        "the typed accessor retains the original error"
    );
}
