use gix_error::{
    Class, ClassificationMarker, Error, ErrorExt, Message, ResourceExhaustionKind, classify, corruption, message,
    not_found, resource_exhaustion, validation,
};

#[test]
fn classifications_preserve_order_duplicates_and_sources() {
    fn allocation_failure() -> std::collections::TryReserveError {
        Vec::<u8>::new()
            .try_reserve(usize::MAX)
            .expect_err("the maximum capacity cannot be reserved")
    }
    let err = Error::from(
        ClassificationMarker::with_source(Class::Retryable, allocation_failure())
            .and_raise(corruption("corrupt input caused allocation")),
    );
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(err, "classification retains each independent cause", @r#"
        Message {
            message: "corrupt input caused allocation",
            class: Corruption,
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(err, "classification retains each independent cause", @"
        corrupt input caused allocation
        |
        └─ memory allocation failed because the computed capacity exceeded the collection's maximum
        ");
    }
    let classifications = err.classify().collect::<Vec<_>>();

    assert_eq!(
        classifications
            .iter()
            .map(gix_error::types::Classification::class)
            .collect::<Vec<_>>(),
        [
            Class::Corruption,
            Class::Retryable,
            Class::ResourceExhaustion(ResourceExhaustionKind::AllocationFailure),
        ],
        "classification follows the error graph without merging independent meanings"
    );
    assert!(classifications[0].error().is::<Message>());
    assert!(classifications[1].error().is::<ClassificationMarker>());
    assert!(classifications[2].error().is::<std::collections::TryReserveError>());

    let duplicate = Error::from(validation("first").raise().chain(validation("second")));
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(duplicate, "separate invalid inputs retain separate diagnostics", @r#"
        Message {
            message: "first",
            class: Validation,
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(duplicate, "separate invalid inputs retain separate diagnostics", @"
        first
        |
        └─ second
        ");
    }
    assert_eq!(
        duplicate.classify().map(|item| item.class()).collect::<Vec<_>>(),
        [Class::Validation, Class::Validation],
        "a classification is emitted for each matching error node"
    );
}

#[test]
fn io_errors_are_normalized_without_losing_their_origin() {
    let mut diagnostics = Vec::new();
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
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        let classification = err.classify().next().expect("all I/O errors are classified");
        assert_eq!(classification.class(), expected_class);
        assert_eq!(classification.io_kind(), Some(io_kind));
        assert!(classification.error().is::<std::io::Error>());
    }
    insta::assert_debug_snapshot!(diagnostics, "io errors are normalized without losing their origin", @"
    [
        Kind(
            NotFound,
        ),
        Kind(
            OutOfMemory,
        ),
        Kind(
            PermissionDenied,
        ),
    ]
    ");
}

#[test]
fn allocation_limits_are_resources_only() {
    let err = Error::from_error(resource_exhaustion(
        ResourceExhaustionKind::AllocationLimit,
        "configured allocation limit exceeded",
    ));

    assert_eq!(
        err.classify().map(|item| item.class()).collect::<Vec<_>>(),
        [Class::ResourceExhaustion(ResourceExhaustionKind::AllocationLimit)]
    );
    insta::assert_debug_snapshot!(err, "allocation limits are resources only", @r#"
    Message {
        message: "configured allocation limit exceeded",
        class: ResourceExhaustion(AllocationLimit),
    }
    "#);
    assert!(!err.is_corrupted());
    assert!(!err.can_retry());
}

#[test]
fn borrowed_retry_policy_is_conservative() {
    let mut inline_error_diagnostics = Vec::new();
    for kind in [std::io::ErrorKind::Interrupted, std::io::ErrorKind::TimedOut] {
        let err = std::io::Error::from(kind);
        inline_error_diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert!(classify(&err).can_retry(), "{kind:?} can be retried conservatively");
    }
    for kind in [
        std::io::ErrorKind::OutOfMemory,
        std::io::ErrorKind::ConnectionReset,
        std::io::ErrorKind::UnexpectedEof,
    ] {
        let err = std::io::Error::from(kind);
        inline_error_diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert!(!classify(&err).can_retry(), "{kind:?} needs explicit retry policy");
    }
    let err = ClassificationMarker::with_source(Class::Retryable, message("try again"));
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "borrowed retry policy is conservative", @r#"
    Message {
        message: "try again",
    }
    "#);
    assert!(classify(&err).can_retry());
    insta::assert_debug_snapshot!(inline_error_diagnostics, "borrowed retry policy is conservative", @"
    [
        Kind(
            Interrupted,
        ),
        Kind(
            TimedOut,
        ),
        Kind(
            OutOfMemory,
        ),
        Kind(
            ConnectionReset,
        ),
        Kind(
            UnexpectedEof,
        ),
    ]
    ");
}

#[test]
fn lenient_retry_policy_preserves_the_previous_io_kinds() {
    let mut inline_error_diagnostics = Vec::new();
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
        let err = std::io::Error::from(kind);
        inline_error_diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert!(
            classify(&err).can_retry_lenient(),
            "{kind:?} is retryable under the lenient policy"
        );
    }

    let err = Error::from_error(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "the lenient policy still rejects permanent I/O errors", @"
    Kind(
        PermissionDenied,
    )
    ");
    assert!(
        !err.can_retry_lenient(),
        "the lenient policy still rejects permanent I/O errors"
    );
    let err = Error::from_error(ClassificationMarker::with_source(
        Class::Retryable,
        message("try again"),
    ));
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "the lenient policy includes explicitly retryable errors", @r#"
    Message {
        message: "try again",
    }
    "#);
    assert!(
        err.can_retry_lenient(),
        "the lenient policy includes explicitly retryable errors"
    );
    let allocation_failure = Vec::<u8>::new()
        .try_reserve(usize::MAX)
        .expect_err("the maximum capacity cannot be reserved");
    let err = Error::from_error(allocation_failure);
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "only I/O OutOfMemory errors are covered by the historical policy", @"
    TryReserveError {
        kind: CapacityOverflow,
    }
    ");
    assert!(
        !err.can_retry_lenient(),
        "only I/O OutOfMemory errors are covered by the historical policy"
    );
    insta::assert_debug_snapshot!(inline_error_diagnostics, "lenient retry policy preserves the previous io kinds", @"
    [
        Kind(
            Interrupted,
        ),
        Kind(
            UnexpectedEof,
        ),
        Kind(
            OutOfMemory,
        ),
        Kind(
            TimedOut,
        ),
        Kind(
            BrokenPipe,
        ),
        Kind(
            AddrInUse,
        ),
        Kind(
            ConnectionAborted,
        ),
        Kind(
            ConnectionReset,
        ),
        Kind(
            ConnectionRefused,
        ),
    ]
    ");
}

#[test]
fn unknown_errors_are_omitted() {
    let err = Error::from_error(message("unknown"));
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "unknown errors are omitted", @r#"
    Message {
        message: "unknown",
    }
    "#);
    assert_eq!(err.classify().count(), 0);
}

#[test]
fn explicit_retryability_is_distinct_from_io_retry_policy() {
    let mut diagnostics = Vec::new();
    let explicit = ClassificationMarker::with_source(Class::Retryable, message("try again")).raise();
    insta::assert_debug_snapshot!(explicit, "typed exceptions expose their retry marker", @"try again");
    assert!(explicit.is_retryable(), "typed exceptions expose their retry marker");

    let nested = Error::from(
        message("nested operation")
            .raise()
            .chain(message("unrelated cause"))
            .chain(ClassificationMarker::with_source(
                Class::Retryable,
                message("try again"),
            )),
    );
    for err in [
        explicit.erased(),
        crate::ErrorWithSource("outer operation", nested).raise_erased(),
        message("outer operation")
            .raise()
            .chain(crate::ErrorWithSource(
                "native source",
                ClassificationMarker::with_source(Class::Retryable, message("try again")),
            ))
            .erased(),
    ] {
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
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
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert!(!err.is_retryable(), "{kind:?} has no explicit retry marker");
        let err = err.into_error();
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert!(!err.is_retryable(), "conversion must not add a retry marker");
        assert!(err.can_retry(), "the retry policy still accepts {kind:?}");
    }
    let unknown = message("retryable in name only").raise();
    insta::assert_debug_snapshot!(unknown, "messages do not establish a classification", @"retryable in name only");
    assert!(!unknown.is_retryable(), "messages do not establish a classification");
    assert!(!unknown.into_error().is_retryable());
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(diagnostics, "explicit retryability is distinct from io retry policy", @r#"
        [
            try again,
            outer operation
            |
            └─ nested operation
            |
            └─ unrelated cause
            |
            └─ try again,
            outer operation
            |
            └─ native source
            |
            └─ try again,
            I/O failed
            |
            └─ operation interrupted,
            Message {
                message: "I/O failed",
            },
            I/O failed
            |
            └─ timed out,
            Message {
                message: "I/O failed",
            },
        ]
        "#);
    } else {
        insta::assert_debug_snapshot!(diagnostics, "explicit retryability is distinct from io retry policy", @"
        [
            try again,
            outer operation
            |
            └─ nested operation
            |
            └─ unrelated cause
            |
            └─ try again,
            outer operation
            |
            └─ native source
            |
            └─ try again,
            I/O failed
            |
            └─ operation interrupted,
            I/O failed
            |
            └─ operation interrupted,
            I/O failed
            |
            └─ timed out,
            I/O failed
            |
            └─ timed out,
        ]
        ");
    }
}

#[test]
fn resource_exhaustion_predicates_normalize_allocation_failures() {
    let mut diagnostics = Vec::new();
    let allocation = Vec::<u8>::new()
        .try_reserve(usize::MAX)
        .expect_err("the maximum capacity cannot be reserved");
    let nested = Error::from_error(crate::ErrorWithSource(
        "native allocation failure",
        std::io::Error::from(std::io::ErrorKind::OutOfMemory),
    ));
    for cause in [
        resource_exhaustion(ResourceExhaustionKind::AllocationLimit, "limit exceeded").raise_erased(),
        resource_exhaustion(ResourceExhaustionKind::AllocationFailure, "allocation failed").raise_erased(),
        allocation.raise_erased(),
        std::io::Error::from(std::io::ErrorKind::OutOfMemory).raise_erased(),
        nested.raise_erased(),
    ] {
        let err = cause.raise(message("operation failed"));
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
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
        validation("invalid input").raise_erased(),
        corruption("invalid data").raise_erased(),
        std::io::Error::from(std::io::ErrorKind::PermissionDenied).raise_erased(),
        message("allocation failed in name only").raise_erased(),
    ] {
        assert!(
            !err.is_resource_exhausted(),
            "unrelated failures are not resource exhaustion"
        );
        assert!(!err.into_error().is_resource_exhausted());
    }
    insta::assert_debug_snapshot!(diagnostics, "resource exhaustion predicates normalize allocation failures", @"
    [
        operation failed
        |
        └─ limit exceeded,
        operation failed
        |
        └─ allocation failed,
        operation failed
        |
        └─ memory allocation failed because the computed capacity exceeded the collection's maximum,
        operation failed
        |
        └─ out of memory,
        operation failed
        |
        └─ native allocation failure
        |
        └─ out of memory,
    ]
    ");
}

#[test]
fn exceptions_expose_ordered_classifications_without_conversion() {
    let nested = Error::from(
        validation("nested input")
            .raise()
            .chain(std::io::Error::from(std::io::ErrorKind::OutOfMemory)),
    );
    let err = crate::ErrorWithSource("root", std::io::Error::from(std::io::ErrorKind::NotFound))
        .raise()
        .chain(nested)
        .chain(validation("sibling input"));
    insta::assert_debug_snapshot!(err, "native sources and nested errors retain their diagnostics", @"
    root
    |
    └─ entity not found
    |
    └─ nested input
    |   |
    |   └─ out of memory
    |
    └─ sibling input
    ");
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
            .map(gix_error::types::Classification::class)
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
    insta::assert_debug_snapshot!(format_args!("{}", classifications[1]
            .error()
            .downcast_ref::<Message>()
            .expect("retain the sibling type")
            ), "breadth-first classification visits the direct sibling before the nested validation error", @"sibling input");
    insta::assert_debug_snapshot!(format_args!("{}", classifications[2]
            .error()
            .downcast_ref::<Message>()
            .expect("retain the nested type")
            ), "nested error boundaries preserve the original validation error and its message", @"nested input");

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
    let err = message("unclassified").raise();
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "unrecognized errors are omitted from classifications", @"unclassified");
    assert_eq!(
        err.classify().count(),
        0,
        "unrecognized errors are omitted from classifications"
    );
}

#[test]
fn exceptions_expose_retry_policies_without_conversion() {
    let mut diagnostics = Vec::new();
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
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert_eq!(err.can_retry(), conservative, "the conservative policy for {kind:?}");
        assert_eq!(err.can_retry_lenient(), lenient, "the lenient policy for {kind:?}");
        assert!(
            !err.is_retryable(),
            "I/O policy must not create an explicit retry marker"
        );
        let err = err.into_error();
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
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

    let explicit =
        Error::from_error(message("context"))
            .raise()
            .chain(Error::from_error(ClassificationMarker::with_source(
                Class::Retryable,
                message("try again"),
            )));
    insta::assert_debug_snapshot!(explicit, "both policies accept explicit markers", @"
    context
    |
    └─ try again
    ");
    assert!(
        explicit.can_retry() && explicit.can_retry_lenient(),
        "both policies accept explicit markers"
    );
    let allocation = Vec::<u8>::new()
        .try_reserve(usize::MAX)
        .expect_err("the maximum capacity cannot be reserved")
        .raise();
    insta::assert_debug_snapshot!(allocation, "allocation failures outside I/O retain their non-retryable classification", @"memory allocation failed because the computed capacity exceeded the collection's maximum");
    assert!(
        !allocation.can_retry() && !allocation.can_retry_lenient(),
        "allocation failures outside I/O retain their non-retryable classification"
    );
    let unknown = message("unknown").raise();
    insta::assert_debug_snapshot!(unknown, "exceptions expose retry policies without conversion", @"unknown");
    assert!(!unknown.can_retry() && !unknown.can_retry_lenient());
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(diagnostics, "exceptions expose retry policies without conversion", @r#"
            [
                native wrapper
                |
                └─ operation interrupted,
                ErrorWithSource(
                    "native wrapper",
                    Kind(
                        Interrupted,
                    ),
                ),
                native wrapper
                |
                └─ timed out,
                ErrorWithSource(
                    "native wrapper",
                    Kind(
                        TimedOut,
                    ),
                ),
                native wrapper
                |
                └─ unexpected end of file,
                ErrorWithSource(
                    "native wrapper",
                    Kind(
                        UnexpectedEof,
                    ),
                ),
                native wrapper
                |
                └─ out of memory,
                ErrorWithSource(
                    "native wrapper",
                    Kind(
                        OutOfMemory,
                    ),
                ),
                native wrapper
                |
                └─ broken pipe,
                ErrorWithSource(
                    "native wrapper",
                    Kind(
                        BrokenPipe,
                    ),
                ),
                native wrapper
                |
                └─ address in use,
                ErrorWithSource(
                    "native wrapper",
                    Kind(
                        AddrInUse,
                    ),
                ),
                native wrapper
                |
                └─ connection aborted,
                ErrorWithSource(
                    "native wrapper",
                    Kind(
                        ConnectionAborted,
                    ),
                ),
                native wrapper
                |
                └─ connection reset,
                ErrorWithSource(
                    "native wrapper",
                    Kind(
                        ConnectionReset,
                    ),
                ),
                native wrapper
                |
                └─ connection refused,
                ErrorWithSource(
                    "native wrapper",
                    Kind(
                        ConnectionRefused,
                    ),
                ),
                native wrapper
                |
                └─ permission denied,
                ErrorWithSource(
                    "native wrapper",
                    Kind(
                        PermissionDenied,
                    ),
                ),
                native wrapper
                |
                └─ entity not found,
                ErrorWithSource(
                    "native wrapper",
                    Kind(
                        NotFound,
                    ),
                ),
            ]
        "#);
    } else {
        insta::assert_debug_snapshot!(diagnostics, "exceptions expose retry policies without conversion", @"
        [
            native wrapper
            |
            └─ operation interrupted,
            native wrapper
            |
            └─ operation interrupted,
            native wrapper
            |
            └─ timed out,
            native wrapper
            |
            └─ timed out,
            native wrapper
            |
            └─ unexpected end of file,
            native wrapper
            |
            └─ unexpected end of file,
            native wrapper
            |
            └─ out of memory,
            native wrapper
            |
            └─ out of memory,
            native wrapper
            |
            └─ broken pipe,
            native wrapper
            |
            └─ broken pipe,
            native wrapper
            |
            └─ address in use,
            native wrapper
            |
            └─ address in use,
            native wrapper
            |
            └─ connection aborted,
            native wrapper
            |
            └─ connection aborted,
            native wrapper
            |
            └─ connection reset,
            native wrapper
            |
            └─ connection reset,
            native wrapper
            |
            └─ connection refused,
            native wrapper
            |
            └─ connection refused,
            native wrapper
            |
            └─ permission denied,
            native wrapper
            |
            └─ permission denied,
            native wrapper
            |
            └─ entity not found,
            native wrapper
            |
            └─ entity not found,
        ]
        ");
    }
}

#[test]
fn custom_io_payloads_retain_all_classifications() {
    let mut diagnostics = Vec::new();
    let cases: [(Box<dyn std::error::Error + Send + Sync>, Class); 5] = [
        (Box::new(not_found("missing object")), Class::NotFound),
        (Box::new(validation("invalid input")), Class::Validation),
        (Box::new(corruption("malformed data")), Class::Corruption),
        (
            Box::new(ClassificationMarker::with_source(
                Class::Retryable,
                message("try again"),
            )),
            Class::Retryable,
        ),
        (
            Box::new(resource_exhaustion(
                ResourceExhaustionKind::AllocationLimit,
                "limit exceeded",
            )),
            Class::ResourceExhaustion(ResourceExhaustionKind::AllocationLimit),
        ),
    ];
    for (payload, class) in cases {
        let err = crate::ErrorWithSource("custom backend failed", std::io::Error::other(payload));
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert_eq!(classify(&err).is_not_found(), class == Class::NotFound);
        assert_eq!(classify(&err).is_validation(), class == Class::Validation);
        assert_eq!(classify(&err).is_corrupted(), class == Class::Corruption);
        assert_eq!(classify(&err).is_retryable(), class == Class::Retryable);
        assert_eq!(
            classify(&err).is_resource_exhausted(),
            matches!(class, Class::ResourceExhaustion(_)),
            "all borrowed predicates reach custom errors through their sources"
        );
        assert_eq!(
            classify(&err).can_retry(),
            class == Class::Retryable,
            "borrowed retry inspection reaches the I/O payload"
        );
        assert_eq!(
            classify(&err).can_retry_lenient(),
            class == Class::Retryable,
            "both retry policies reach the I/O payload"
        );
        let err = err.raise();
        assert_eq!(
            err.classify().map(|item| item.class()).collect::<Vec<_>>(),
            [Class::Io(std::io::ErrorKind::Other), class],
            "the custom wrapper retains both the I/O error and its classified payload"
        );
        let err = err.into_error();
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert_eq!(err.is_not_found(), class == Class::NotFound);
        assert_eq!(err.is_validation(), class == Class::Validation);
        assert_eq!(err.is_corrupted(), class == Class::Corruption);
        assert_eq!(err.is_retryable(), class == Class::Retryable);
        assert_eq!(err.can_retry(), class == Class::Retryable);
        assert_eq!(err.can_retry_lenient(), class == Class::Retryable);
        assert_eq!(
            err.is_resource_exhausted(),
            matches!(class, Class::ResourceExhaustion(_))
        );
    }
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(diagnostics, "custom io payloads retain all classifications", @r#"
        [
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "missing object",
                        class: NotFound,
                    },
                },
            ),
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "missing object",
                        class: NotFound,
                    },
                },
            ),
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "invalid input",
                        class: Validation,
                    },
                },
            ),
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "invalid input",
                        class: Validation,
                    },
                },
            ),
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "malformed data",
                        class: Corruption,
                    },
                },
            ),
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "malformed data",
                        class: Corruption,
                    },
                },
            ),
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "try again",
                    },
                },
            ),
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "try again",
                    },
                },
            ),
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "limit exceeded",
                        class: ResourceExhaustion(AllocationLimit),
                    },
                },
            ),
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "limit exceeded",
                        class: ResourceExhaustion(AllocationLimit),
                    },
                },
            ),
        ]
        "#);
    } else {
        insta::assert_debug_snapshot!(diagnostics, "custom io payloads retain all classifications", @r#"
        [
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "missing object",
                        class: NotFound,
                    },
                },
            ),
            custom backend failed
            |
            └─ I/O error (Other)
            |
            └─ missing object,
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "invalid input",
                        class: Validation,
                    },
                },
            ),
            custom backend failed
            |
            └─ I/O error (Other)
            |
            └─ invalid input,
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "malformed data",
                        class: Corruption,
                    },
                },
            ),
            custom backend failed
            |
            └─ I/O error (Other)
            |
            └─ malformed data,
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "try again",
                    },
                },
            ),
            custom backend failed
            |
            └─ I/O error (Other)
            |
            └─ try again,
            ErrorWithSource(
                "custom backend failed",
                Custom {
                    kind: Other,
                    error: Message {
                        message: "limit exceeded",
                        class: ResourceExhaustion(AllocationLimit),
                    },
                },
            ),
            custom backend failed
            |
            └─ I/O error (Other)
            |
            └─ limit exceeded,
        ]
        "#);
    }
}

#[test]
fn classification_markers_preserve_categories_and_origins() {
    let mut diagnostics = Vec::new();
    for class in [
        Class::Validation,
        Class::Corruption,
        Class::NotFound,
        Class::Retryable,
        Class::ResourceExhaustion(ResourceExhaustionKind::AllocationLimit),
        Class::ResourceExhaustion(ResourceExhaustionKind::AllocationFailure),
        Class::Io(std::io::ErrorKind::TimedOut),
    ] {
        let marker = ClassificationMarker::with_class(class);
        let err = crate::ErrorWithSource("specific diagnostic", marker);
        let classification = classify(&err).next().expect("the source supplies a classification");
        assert_eq!(
            classification.class(),
            class,
            "the marker supplies its explicit category"
        );
        assert_eq!(
            classification.io_kind(),
            None,
            "a classification marker does not invent an I/O origin"
        );
        assert!(
            std::ptr::eq(
                classification
                    .error()
                    .downcast_ref::<ClassificationMarker>()
                    .expect("the original marker is retained"),
                &err.1,
            ),
            "classification identifies the original marker rather than a fabricated causal error"
        );

        let err = err.and_raise(message("outer context")).erased();
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert_eq!(
            err.is_validation(),
            class == Class::Validation,
            "validation survives erasure"
        );
        assert_eq!(
            err.is_corrupted(),
            class == Class::Corruption,
            "corruption survives erasure"
        );
        assert_eq!(err.is_not_found(), class == Class::NotFound, "absence survives erasure");
        assert_eq!(
            err.is_retryable(),
            class == Class::Retryable,
            "retry markers remain explicit"
        );
        assert_eq!(
            err.is_resource_exhausted(),
            matches!(class, Class::ResourceExhaustion(_)),
            "resource classifications preserve their kinds"
        );
        assert_eq!(
            err.can_retry(),
            matches!(class, Class::Retryable | Class::Io(std::io::ErrorKind::TimedOut)),
            "retry policies recognize the marker's classification"
        );
        for err in [
            err.into_error(),
            std::io::Error::other(ClassificationMarker::with_class(class))
                .raise()
                .into_error(),
        ] {
            assert!(
                err.downcast_any_ref::<ClassificationMarker>().is_none(),
                "public downcasts omit markers in converted errors and I/O payloads"
            );
            assert_eq!(
                err.classify().last().expect("the marker is classified").class(),
                class,
                "conversion and I/O wrapping preserve classification"
            );
        }
    }
    insta::assert_debug_snapshot!(diagnostics, "classification markers preserve categories and origins", @"
    [
        outer context
        |
        └─ specific diagnostic,
        outer context
        |
        └─ specific diagnostic,
        outer context
        |
        └─ specific diagnostic,
        outer context
        |
        └─ specific diagnostic,
        outer context
        |
        └─ specific diagnostic,
        outer context
        |
        └─ specific diagnostic,
        outer context
        |
        └─ specific diagnostic,
    ]
    ");
}

#[test]
fn markers_remain_transparent_alongside_classified_errors() {
    let marker = ClassificationMarker::VALIDATION;
    assert!(
        std::error::Error::source(&marker).is_none(),
        "class-only markers do not fabricate further causes"
    );
    let err = crate::ErrorWithSource("specific diagnostic", marker)
        .and_raise(validation("context with input").with("input", b"bad".as_slice()));
    let classifications = err.classify().collect::<Vec<_>>();
    assert_eq!(
        classifications.len(),
        2,
        "real errors and markers are classified independently"
    );
    assert!(
        classifications[0].error().is::<Message>(),
        "the real error retains its type"
    );
    assert!(
        classifications[1].error().is::<ClassificationMarker>(),
        "the marker retains its type"
    );
    assert_eq!(
        classifications
            .iter()
            .map(gix_error::types::Classification::class)
            .collect::<Vec<_>>(),
        [Class::Validation, Class::Validation],
        "classifications remain ordered and are not deduplicated"
    );
    insta::assert_debug_snapshot!(err, "markers supply classifications without becoming diagnostic errors", @r#"
    context with input, "input"="bad"
    |
    └─ specific diagnostic
    "#);
    assert!(
        err.downcast_any_ref::<ClassificationMarker>().is_none(),
        "markers supply classifications without becoming diagnostic errors"
    );
    assert!(
        err.probable_cause()
            .is::<crate::ErrorWithSource<ClassificationMarker>>(),
        "the marker never replaces its typed diagnostic owner"
    );
    let err = err.into_error();
    assert!(
        err.downcast_any_ref::<ClassificationMarker>().is_none(),
        "conversion preserves marker transparency"
    );
    assert!(
        err.downcast_any_ref::<Message>().is_some(),
        "conversion preserves genuine classified errors for downcasting"
    );
    assert!(
        err.probable_cause()
            .is::<crate::ErrorWithSource<ClassificationMarker>>(),
        "conversion preserves the typed cause rather than its marker"
    );
    insta::assert_debug_snapshot!(err.iter_errors().map(ToString::to_string).collect::<Vec<_>>(), "only real diagnostics remain visible in diagnostic iteration", @r#"
    [
        "context with input, \"input\"=\"bad\"",
        "specific diagnostic",
    ]
    "#);
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_debug_snapshot!(err, @r#"
    context with input, "input"="bad"
    |
    └─ specific diagnostic
    "#);
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    insta::assert_debug_snapshot!(err, "chain Debug retains the classified message's concrete type", @r#"
    Message {
        message: "context with input",
        class: Validation,
        values: {"input": Bytes("bad")},
    }
    "#);
}

#[test]
fn source_markers_hide_the_wrapper_but_preserve_the_original_error() {
    let source = std::io::Error::from(std::io::ErrorKind::NotFound);
    let err = ClassificationMarker::with_source(Class::Retryable, source).raise();
    let expected_classes = [Class::Retryable, Class::NotFound];
    assert_eq!(
        err.classify().map(|item| item.class()).collect::<Vec<_>>(),
        expected_classes,
        "a source-bearing marker adds a classification without replacing its source's classification"
    );
    insta::assert_debug_snapshot!(err, "source-bearing markers are hidden from public downcasts", @"entity not found");
    assert!(
        err.downcast_any_ref::<ClassificationMarker>().is_none(),
        "source-bearing markers are hidden from public downcasts"
    );
    assert!(
        err.downcast_any_ref::<std::io::Error>().is_some(),
        "the original source remains available for downcasting"
    );
    insta::assert_debug_snapshot!(err.iter_errors().map(ToString::to_string).collect::<Vec<_>>(), "only the original error is yielded", @r#"
    [
        "entity not found",
    ]
    "#);
    let err = err.into_error();
    assert!(
        err.downcast_any_ref::<ClassificationMarker>().is_none(),
        "conversion retains the wrapper's transparency"
    );
    assert!(
        err.downcast_any_ref::<std::io::Error>().is_some(),
        "conversion preserves the original source type"
    );
    let classifications = err.classify().collect::<Vec<_>>();
    assert_eq!(
        classifications
            .iter()
            .map(gix_error::types::Classification::class)
            .collect::<Vec<_>>(),
        expected_classes,
        "conversion preserves both classifications"
    );
    assert!(
        classifications[0].error().is::<ClassificationMarker>(),
        "classification retains the hidden wrapper as the origin of its explicit category"
    );
    assert_eq!(
        classifications[1].io_kind(),
        Some(std::io::ErrorKind::NotFound),
        "the source retains its real I/O origin"
    );
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_debug_snapshot!(err, @"entity not found");
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    insta::assert_debug_snapshot!(err, "chain Debug exposes the source without its marker", @"
    Kind(
        NotFound,
    )
    ");
}

#[test]
fn io_payloads_retain_custom_errors_and_nested_branches() {
    let mut diagnostics = Vec::new();
    for custom_wrapper in [false, true] {
        let nested = message("nested operation failed")
            .raise()
            .chain(validation("invalid input"))
            .chain(ClassificationMarker::with_source(
                Class::Retryable,
                message("try again"),
            ))
            .into_error();
        let payload: Box<dyn std::error::Error + Send + Sync> = if custom_wrapper {
            Box::new(crate::ErrorWithSource("custom payload", nested))
        } else {
            Box::new(nested)
        };
        let io = std::io::Error::other(payload);
        diagnostics.push(gix_testtools::redact_debug_snapshot(&io, &[]));
        assert!(
            classify(&io).can_retry(),
            "borrowed inspection reaches every branch in an I/O payload"
        );
        assert!(classify(&io).is_validation());
        let err = Error::from_error(io);
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        assert!(err.can_retry(), "conversion retains retryable branches");
        assert!(err.is_validation(), "conversion retains other classifications too");
        if custom_wrapper {
            assert!(
                err.downcast_any_ref::<crate::ErrorWithSource<Error>>().is_some(),
                "the custom payload remains available for downcasting"
            );
        }
    }
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(diagnostics, "io payloads retain custom errors and nested branches", @r#"
        [
            Custom {
                kind: Other,
                error: Message {
                    message: "nested operation failed",
                },
            },
            Custom {
                kind: Other,
                error: Message {
                    message: "nested operation failed",
                },
            },
            Custom {
                kind: Other,
                error: ErrorWithSource(
                    "custom payload",
                    Message {
                        message: "nested operation failed",
                    },
                ),
            },
            Custom {
                kind: Other,
                error: ErrorWithSource(
                    "custom payload",
                    Message {
                        message: "nested operation failed",
                    },
                ),
            },
        ]
        "#);
    } else {
        insta::assert_debug_snapshot!(diagnostics, "io payloads retain custom errors and nested branches", @r#"
        [
            Custom {
                kind: Other,
                error: nested operation failed
                |
                └─ invalid input
                |
                └─ try again,
            },
            Custom {
                kind: Other,
                error: nested operation failed
                |
                └─ invalid input
                |
                └─ try again,
            },
            Custom {
                kind: Other,
                error: ErrorWithSource(
                    "custom payload",
                    nested operation failed
                    |
                    └─ invalid input
                    |
                    └─ try again,
                ),
            },
            Custom {
                kind: Other,
                error: ErrorWithSource(
                    "custom payload",
                    nested operation failed
                    |
                    └─ invalid input
                    |
                    └─ try again,
                ),
            },
        ]
        "#);
    }
}
