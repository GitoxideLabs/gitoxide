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
