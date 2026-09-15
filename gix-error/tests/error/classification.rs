use gix_error::{
    Class, CorruptionError, Error, ErrorExt, ResourceExhaustionError, ResourceExhaustionKind, RetryableError,
    ValidationError, can_retry, can_retry_lenient, message,
};

#[test]
fn classifications_preserve_order_duplicates_and_sources() {
    fn allocation_failure() -> std::collections::TryReserveError {
        Vec::<u8>::new()
            .try_reserve(usize::MAX)
            .expect_err("the maximum capacity cannot be reserved")
    }
    let err = Error::from(
        RetryableError::new(allocation_failure()).and_raise(CorruptionError::new("corrupt input caused allocation")),
    );
    let classifications = err.classify().collect::<Vec<_>>();

    assert_eq!(
        classifications
            .iter()
            .map(gix_error::Classification::class)
            .collect::<Vec<_>>(),
        [
            Class::Corruption,
            Class::Retryable,
            Class::ResourceExhaustion(ResourceExhaustionKind::AllocationFailure),
        ],
        "classification follows the error graph without merging independent meanings"
    );
    assert!(classifications[0].error().is::<CorruptionError>());
    assert!(classifications[1].error().is::<RetryableError>());
    assert!(classifications[2].error().is::<std::collections::TryReserveError>());

    let duplicate = Error::from(
        ValidationError::new("first")
            .raise()
            .chain(ValidationError::new("second")),
    );
    assert_eq!(
        duplicate.classify().map(|item| item.class()).collect::<Vec<_>>(),
        [Class::Validation, Class::Validation],
        "a classification is emitted for each matching error node"
    );
}

#[test]
fn io_errors_are_normalized_without_losing_their_origin() {
    let cases = [
        (std::io::ErrorKind::NotFound, Class::NotFound),
        (
            std::io::ErrorKind::OutOfMemory,
            Class::ResourceExhaustion(ResourceExhaustionKind::AllocationFailure),
        ),
        (
            std::io::ErrorKind::PermissionDenied,
            Class::Io(std::io::ErrorKind::PermissionDenied),
        ),
    ];

    for (io_kind, expected_class) in cases {
        let err = Error::from_error(std::io::Error::from(io_kind));
        let classification = err.classify().next().expect("all I/O errors are classified");
        assert_eq!(classification.class(), expected_class);
        assert_eq!(classification.io_kind(), Some(io_kind));
        assert!(classification.error().is::<std::io::Error>());
    }
}

#[test]
fn allocation_limits_are_resources_only() {
    let err = Error::from_error(ResourceExhaustionError::new(
        ResourceExhaustionKind::AllocationLimit,
        "configured allocation limit exceeded",
    ));

    assert_eq!(
        err.classify().map(|item| item.class()).collect::<Vec<_>>(),
        [Class::ResourceExhaustion(ResourceExhaustionKind::AllocationLimit)]
    );
    assert!(!err.is_corrupted());
    assert!(!err.can_retry());
}

#[test]
fn global_retry_policy_is_conservative() {
    for kind in [std::io::ErrorKind::Interrupted, std::io::ErrorKind::TimedOut] {
        assert!(
            can_retry(&std::io::Error::from(kind)),
            "{kind:?} can be retried globally"
        );
    }
    for kind in [
        std::io::ErrorKind::OutOfMemory,
        std::io::ErrorKind::ConnectionReset,
        std::io::ErrorKind::UnexpectedEof,
    ] {
        assert!(
            !can_retry(&std::io::Error::from(kind)),
            "{kind:?} needs explicit retry policy"
        );
    }
    assert!(can_retry(&RetryableError::new(message("try again"))));
}

#[test]
fn lenient_retry_policy_preserves_the_previous_io_kinds() {
    for kind in [
        std::io::ErrorKind::Interrupted,
        std::io::ErrorKind::UnexpectedEof,
        std::io::ErrorKind::OutOfMemory,
        std::io::ErrorKind::TimedOut,
        std::io::ErrorKind::BrokenPipe,
        std::io::ErrorKind::AddrInUse,
        std::io::ErrorKind::ConnectionAborted,
        std::io::ErrorKind::ConnectionReset,
        std::io::ErrorKind::ConnectionRefused,
    ] {
        assert!(
            can_retry_lenient(&std::io::Error::from(kind)),
            "{kind:?} is retryable under the lenient policy"
        );
    }

    assert!(
        !Error::from_error(std::io::Error::from(std::io::ErrorKind::PermissionDenied)).can_retry_lenient(),
        "the lenient policy still rejects permanent I/O errors"
    );
    assert!(
        Error::from_error(RetryableError::new(message("try again"))).can_retry_lenient(),
        "the lenient policy includes explicitly retryable errors"
    );
    let allocation_failure = Vec::<u8>::new()
        .try_reserve(usize::MAX)
        .expect_err("the maximum capacity cannot be reserved");
    assert!(
        !Error::from_error(allocation_failure).can_retry_lenient(),
        "only I/O OutOfMemory errors are covered by the historical policy"
    );
}

#[test]
fn unknown_errors_are_omitted() {
    assert_eq!(Error::from_error(message("unknown")).classify().count(), 0);
}

#[test]
fn explicit_retryability_is_distinct_from_io_retry_policy() {
    let explicit = RetryableError::new(message("try again")).raise();
    assert!(explicit.is_retryable(), "typed exceptions expose their retry marker");

    let nested = Error::from(
        message("nested operation")
            .raise()
            .chain(message("unrelated cause"))
            .chain(RetryableError::new(message("try again"))),
    );
    for err in [
        explicit.erased(),
        crate::ErrorWithSource("outer operation", nested).raise_erased(),
        message("outer operation")
            .raise()
            .chain(crate::ErrorWithSource(
                "native source",
                RetryableError::new(message("try again")),
            ))
            .erased(),
    ] {
        assert!(
            err.is_retryable(),
            "markers survive erasure, branches, and native sources"
        );
        assert!(
            err.into_error().is_retryable(),
            "conversion preserves explicit retryability"
        );
    }

    for kind in [std::io::ErrorKind::Interrupted, std::io::ErrorKind::TimedOut] {
        let err = std::io::Error::from(kind).and_raise(message("I/O failed"));
        assert!(!err.is_retryable(), "{kind:?} has no explicit retry marker");
        let err = err.into_error();
        assert!(!err.is_retryable(), "conversion must not add a retry marker");
        assert!(err.can_retry(), "the retry policy still accepts {kind:?}");
    }
    let unknown = message("retryable in name only").raise();
    assert!(!unknown.is_retryable(), "messages do not establish a classification");
    assert!(!unknown.into_error().is_retryable());
}

#[test]
fn resource_exhaustion_predicates_normalize_allocation_failures() {
    let allocation = Vec::<u8>::new()
        .try_reserve(usize::MAX)
        .expect_err("the maximum capacity cannot be reserved");
    let nested = Error::from_error(crate::ErrorWithSource(
        "native allocation failure",
        std::io::Error::from(std::io::ErrorKind::OutOfMemory),
    ));
    for cause in [
        ResourceExhaustionError::new(ResourceExhaustionKind::AllocationLimit, "limit exceeded").raise_erased(),
        ResourceExhaustionError::new(ResourceExhaustionKind::AllocationFailure, "allocation failed").raise_erased(),
        allocation.raise_erased(),
        std::io::Error::from(std::io::ErrorKind::OutOfMemory).raise_erased(),
        nested.raise_erased(),
    ] {
        let err = cause.raise(message("operation failed"));
        assert!(
            err.is_resource_exhausted(),
            "all known allocation failures are classified"
        );
        assert!(
            err.into_error().is_resource_exhausted(),
            "conversion retains the resource classification"
        );
    }

    for err in [
        ValidationError::new("invalid input").raise_erased(),
        CorruptionError::new("invalid data").raise_erased(),
        std::io::Error::from(std::io::ErrorKind::PermissionDenied).raise_erased(),
        message("allocation failed in name only").raise_erased(),
    ] {
        assert!(
            !err.is_resource_exhausted(),
            "unrelated failures are not resource exhaustion"
        );
        assert!(!err.into_error().is_resource_exhausted());
    }
}

#[test]
fn exceptions_expose_ordered_classifications_without_conversion() {
    let nested = Error::from(
        ValidationError::new("nested input")
            .raise()
            .chain(std::io::Error::from(std::io::ErrorKind::OutOfMemory)),
    );
    let err = crate::ErrorWithSource("root", std::io::Error::from(std::io::ErrorKind::NotFound))
        .raise()
        .chain(nested)
        .chain(ValidationError::new("sibling input"));
    let expected = [
        Class::NotFound,
        Class::Validation,
        Class::Validation,
        Class::ResourceExhaustion(ResourceExhaustionKind::AllocationFailure),
    ];
    let classifications = err.classify().collect::<Vec<_>>();
    assert_eq!(
        classifications
            .iter()
            .map(gix_error::Classification::class)
            .collect::<Vec<_>>(),
        expected,
        "native sources and nested errors share breadth-first ordering without deduplicating classes"
    );
    assert_eq!(
        classifications[0].io_kind(),
        Some(std::io::ErrorKind::NotFound),
        "classifying the root's native source retains its original I/O kind"
    );
    assert_eq!(
        classifications[3].io_kind(),
        Some(std::io::ErrorKind::OutOfMemory),
        "normalizing a nested I/O error to resource exhaustion retains its original I/O kind"
    );
    assert_eq!(
        classifications[1]
            .error()
            .downcast_ref::<ValidationError>()
            .expect("retain the sibling type")
            .to_string(),
        "sibling input",
        "breadth-first classification visits the direct sibling before the nested validation error"
    );
    assert_eq!(
        classifications[2]
            .error()
            .downcast_ref::<ValidationError>()
            .expect("retain the nested type")
            .to_string(),
        "nested input",
        "nested error boundaries preserve the original validation error and its message"
    );

    let err = err.erased();
    assert_eq!(
        err.classify().map(|item| item.class()).collect::<Vec<_>>(),
        expected,
        "type erasure preserves classification order and duplicate classes"
    );
    assert_eq!(
        err.into_error().classify().map(|item| item.class()).collect::<Vec<_>>(),
        expected,
        "conversion to Error preserves classification order and duplicate classes"
    );
    assert_eq!(
        message("unclassified").raise().classify().count(),
        0,
        "unrecognized errors are omitted from classifications"
    );
}

#[test]
fn exceptions_expose_retry_policies_without_conversion() {
    use std::io::ErrorKind::*;

    for (kind, conservative, lenient) in [
        (Interrupted, true, true),
        (TimedOut, true, true),
        (UnexpectedEof, false, true),
        (OutOfMemory, false, true),
        (BrokenPipe, false, true),
        (AddrInUse, false, true),
        (ConnectionAborted, false, true),
        (ConnectionReset, false, true),
        (ConnectionRefused, false, true),
        (PermissionDenied, false, false),
        (NotFound, false, false),
    ] {
        let err =
            crate::ErrorWithSource("native wrapper", Error::from_error(std::io::Error::from(kind))).raise_erased();
        assert_eq!(err.can_retry(), conservative, "the conservative policy for {kind:?}");
        assert_eq!(err.can_retry_lenient(), lenient, "the lenient policy for {kind:?}");
        assert!(
            !err.is_retryable(),
            "I/O policy must not create an explicit retry marker"
        );
        let err = err.into_error();
        assert_eq!(
            err.can_retry(),
            conservative,
            "conversion preserves the conservative policy"
        );
        assert_eq!(
            err.can_retry_lenient(),
            lenient,
            "conversion preserves the lenient policy"
        );
    }

    let explicit = Error::from_error(message("context"))
        .raise()
        .chain(Error::from_error(RetryableError::new(message("try again"))));
    assert!(
        explicit.can_retry() && explicit.can_retry_lenient(),
        "both policies accept explicit markers"
    );
    let allocation = Vec::<u8>::new()
        .try_reserve(usize::MAX)
        .expect_err("the maximum capacity cannot be reserved")
        .raise();
    assert!(
        !allocation.can_retry() && !allocation.can_retry_lenient(),
        "allocation failures outside I/O retain their non-retryable classification"
    );
    let unknown = message("unknown").raise();
    assert!(!unknown.can_retry() && !unknown.can_retry_lenient());
}
