use std::{borrow::Cow, collections::BTreeMap, path::Path};

use gix_error::{Class, Error, ErrorExt, Message, MetadataValue, ResourceExhaustionKind, ResultExt, not_found};

#[test]
fn message_debug_keeps_class_and_values_compact() {
    for (class, expected_class) in [
        (Class::Validation, "Validation"),
        (
            Class::ResourceExhaustion(ResourceExhaustionKind::AllocationLimit),
            "ResourceExhaustion(AllocationLimit)",
        ),
    ] {
        let message = Message::new("details")
            .with_class(class)
            .with("offset", 42_u64)
            .with("input", b"bad\xff".as_slice());
        assert_eq!(
            format!("{message:?}"),
            format!(
                r#"Message {{ message: "details", class: {expected_class}, values: {{"input": Bytes("bad\xff"), "offset": U64(42)}} }}"#
            ),
            "compact debug omits Some and preserves ordered, typed values"
        );
        assert_eq!(
            format!("{message:#?}"),
            format!(
                r#"Message {{
    message: "details",
    class: {expected_class},
    values: {{"input": Bytes("bad\xff"), "offset": U64(42)}},
}}"#
            ),
            "pretty debug keeps both the class and the entire values map on single lines"
        );
    }
}

#[test]
fn message_debug_omits_absent_class_and_empty_values() {
    for (message, compact, pretty) in [
        (
            Message::new("details"),
            r#"Message { message: "details" }"#,
            r#"Message {
    message: "details",
}"#,
        ),
        (
            Message::new("details").with_class(Class::Validation),
            r#"Message { message: "details", class: Validation }"#,
            r#"Message {
    message: "details",
    class: Validation,
}"#,
        ),
        (
            Message::new("details").with("offset", 42_u64),
            r#"Message { message: "details", values: {"offset": U64(42)} }"#,
            r#"Message {
    message: "details",
    values: {"offset": U64(42)},
}"#,
        ),
    ] {
        assert_eq!(
            format!("{message:?}"),
            compact,
            "compact debug omits absent fields without hiding populated fields"
        );
        assert_eq!(
            format!("{message:#?}"),
            pretty,
            "pretty debug omits absent fields without hiding populated fields"
        );
    }
}

#[test]
fn metadata_value_debug_stays_compact_in_pretty_output() {
    for (value, expected) in [
        (MetadataValue::Bool(true), "Bool(true)"),
        (MetadataValue::I64(i64::MIN), "I64(-9223372036854775808)"),
        (MetadataValue::U64(u64::MAX), "U64(18446744073709551615)"),
        (MetadataValue::F64(1.5), "F64(1.5)"),
        (MetadataValue::String("line\nbreak".into()), r#"String("line\nbreak")"#),
        (
            MetadataValue::Bytes(b"ref\xff".as_slice().into()),
            r#"Bytes("ref\xff")"#,
        ),
        (MetadataValue::Path(Path::new("objects").into()), r#"Path("objects")"#),
    ] {
        assert_eq!(
            format!("{value:?}"),
            expected,
            "debug retains the variant and escaped payload"
        );
        assert_eq!(
            format!("{value:#?}"),
            expected,
            "pretty debug does not expand scalar values"
        );
    }
}

#[test]
fn scalar_values_are_lossless_and_keys_are_local_to_a_context() {
    let metadata = Message::new("details")
        .with("signed", i64::MIN)
        .with("unsigned", u64::MAX)
        .with("float", 1.5)
        .with("bytes", b"ref\xff".as_slice())
        .with("path", Path::new("objects"))
        .with("text", "line\nbreak")
        .with("flag", false)
        .with("flag", true);
    assert_eq!(metadata.values["signed"], MetadataValue::I64(i64::MIN));
    assert_eq!(metadata.values["unsigned"], MetadataValue::U64(u64::MAX));
    assert_eq!(metadata.values["float"], MetadataValue::F64(1.5));
    assert_eq!(
        metadata.values["bytes"],
        MetadataValue::Bytes(b"ref\xff".as_slice().into())
    );
    assert_eq!(
        metadata.values["path"],
        MetadataValue::Path(Path::new("objects").into())
    );
    assert_eq!(
        metadata.values["flag"],
        MetadataValue::Bool(true),
        "the last value replaces its predecessor"
    );
    insta::assert_debug_snapshot!(format_args!("{}", Message::new("details")
            .with("z", "line\nbreak")
            .with("a", 2)
            ), "keys are ordered and text is escaped", @r#"details, "a"=2, "z"="line\nbreak""#);
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(b"ref\xff".to_vec()));
        assert_eq!(
            MetadataValue::from(path.as_path()),
            MetadataValue::Path(path),
            "paths retain native bytes"
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        let path = std::path::PathBuf::from(std::ffi::OsString::from_wide(&[0xd800]));
        assert_eq!(
            MetadataValue::from(path.as_path()),
            MetadataValue::Path(path),
            "paths retain native code units"
        );
    }

    insta::assert_snapshot!(metadata, "validate Display", @r#"details, "bytes"="ref\xff", "flag"=true, "float"=1.5, "path"="objects", "signed"=-9223372036854775808, "text"="line\nbreak", "unsigned"=18446744073709551615"#);
    insta::assert_debug_snapshot!(metadata, "validate Display", @r#"
    Message {
        message: "details",
        values: {"bytes": Bytes("ref\xff"), "flag": Bool(true), "float": F64(1.5), "path": Path("objects"), "signed": I64(-9223372036854775808), "text": String("line\nbreak"), "unsigned": U64(18446744073709551615)},
    }
    "#);
}

#[test]
fn metadata_contexts_preserve_causes_and_remain_separate_through_conversion() {
    let missing = not_found("missing").and_raise_typed(Message::new("lookup").with("path", "first"));
    let retry =
        std::io::Error::from(std::io::ErrorKind::TimedOut).and_raise_typed(Message::new("read").with("path", "second"));
    let err = Error::from_error(super::ErrorWithSource("custom", missing.into_error()))
        .raise_typed()
        .chain(retry);
    let paths = |values: &gix_error::Metadata| values["path"].clone();
    assert_eq!(
        err.metadata().map(paths).collect::<Vec<_>>(),
        [MetadataValue::from("second"), MetadataValue::from("first")],
        "native sources and explicit child contexts share the existing traversal order"
    );
    assert!(err.is_not_found() && err.can_retry());

    insta::assert_snapshot!(format!("{:#}", err.error()), "the stored error's alternate display omits locations", @"custom");
    insta::assert_debug_snapshot!(err, @r#"
    custom
    |
    └─ lookup, "path"="first"
    |
    └─ missing
    |
    └─ read, "path"="second"
    |
    └─ timed out
    "#);

    let err = err.into_error();
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(err, "context leaves classifications intact", @r#"
        ErrorWithSource(
            "custom",
            Message {
                message: "lookup",
                values: {"path": String("first")},
            },
        )
        "#);
    } else {
        insta::assert_debug_snapshot!(err, "context leaves classifications intact", @r#"
        custom
        |
        └─ lookup, "path"="first"
        |
        └─ missing
        |
        └─ read, "path"="second"
        |
        └─ timed out
        "#);
    }
    assert!(
        err.is_not_found() && err.can_retry(),
        "context leaves classifications intact"
    );
    assert_eq!(
        err.metadata().map(paths).collect::<Vec<_>>(),
        [MetadataValue::from("second"), MetadataValue::from("first")]
    );
    assert!(
        err.classify()
            .find(|class| class.class() == Class::NotFound)
            .expect("the missing resource is classified")
            .error()
            .is::<Message>(),
        "the concrete cause remains accessible"
    );

    let ok: Result<(), std::io::Error> = Ok(());
    assert!(
        ok.or_raise_typed(|| -> Message { panic!("context must be lazy") })
            .is_ok()
    );
}

#[test]
fn classified_context_preserves_the_real_callee() {
    let error = std::io::Error::from(std::io::ErrorKind::PermissionDenied)
        .and_raise_typed(gix_error::retryable("try reading again").with("path", Path::new("HEAD")));
    insta::assert_debug_snapshot!(error, "the message context supplies explicit retryability", @r#"
    try reading again, "path"="HEAD"
    |
    └─ permission denied
    "#);
    assert!(
        error.is_retryable(),
        "the message context supplies explicit retryability"
    );
    assert!(
        error.probable_cause().is::<std::io::Error>(),
        "classified context does not replace the callee"
    );
    assert_eq!(
        error.iter_errors().count(),
        2,
        "the context and callee remain distinct diagnostics"
    );
    let error = error.into_error();
    let classifications = error.classify().collect::<Vec<_>>();
    assert_eq!(
        classifications
            .iter()
            .map(gix_error::types::Classification::class)
            .collect::<Vec<_>>(),
        [Class::Retryable],
        "only the explicitly classified context yields a classification"
    );
    assert!(
        classifications[0].error().is::<Message>(),
        "the message context establishes its class"
    );
    assert_eq!(
        classifications[0].io_kind(),
        None,
        "the context does not impersonate its callee"
    );
    assert_eq!(
        error
            .downcast_any_ref::<std::io::Error>()
            .expect("the original callee remains available")
            .kind(),
        std::io::ErrorKind::PermissionDenied,
        "the callee retains its native I/O origin"
    );
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(error, "conversion retains the real cause", @r#"
        Message {
            message: "try reading again",
            class: Retryable,
            values: {"path": Path("HEAD")},
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(error, "conversion retains the real cause", @r#"
        try reading again, "path"="HEAD"
        |
        └─ permission denied
        "#);
    }
    assert!(
        error.probable_cause().is::<std::io::Error>(),
        "conversion retains the real cause"
    );
}

#[test]
fn message_classifications_survive_markers_native_sources_and_nested_branches() {
    let nested = gix_error::Exn::raise_all(
        [
            gix_error::not_found("first").with("path", "a").raise_typed(),
            gix_error::not_found("second").with("path", "b").raise_typed(),
        ],
        Message::new("lookup"),
    );
    let error = gix_error::ClassificationMarker::with_source(
        Class::Retryable,
        super::ErrorWithSource("native", std::io::Error::other(nested.into_error())),
    )
    .and_raise_typed(Message::new("outer"));
    let expected = [Class::Retryable, Class::NotFound, Class::NotFound];
    assert_eq!(
        error.classify().map(|item| item.class()).collect::<Vec<_>>(),
        expected,
        "unclassified contexts are skipped, while each generic cause retains its classification"
    );
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(error, "predicates inspect generic causes and markers together", @r#"
        outer
        |
        └─ native
        |
        └─ I/O error (Other)
        |
        └─ lookup
        |
        └─ first, "path"="a"
        |
        └─ second, "path"="b"
        "#);
    } else {
        insta::assert_debug_snapshot!(error, "predicates inspect generic causes and markers together", @r#"
        outer
        |
        └─ native
        |
        └─ I/O error (Other)
        |
        └─ lookup
        |
        └─ first, "path"="a"
        |
        └─ second, "path"="b"
        "#);
    }
    assert!(
        error.is_not_found() && error.is_retryable(),
        "predicates inspect generic causes and markers together"
    );
    insta::assert_debug_snapshot!(format_args!("{}", error.probable_cause()), "a generic aggregate remains a causal boundary", @"lookup");
    let error = error.erased().into_error();
    assert_eq!(
        error.classify().map(|item| item.class()).collect::<Vec<_>>(),
        expected,
        "conversion preserves order, duplicate classes, and native sources"
    );
    assert_eq!(
        error.metadata().map(|values| values.get("path")).collect::<Vec<_>>(),
        [Some(&MetadataValue::from("a")), Some(&MetadataValue::from("b"))],
        "metadata skips empty dictionaries and markers without merging or reordering contexts"
    );
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(error, "classification markers remain transparent", @r#"
        Message {
            message: "outer",
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(error, "classification markers remain transparent", @r#"
        outer
        |
        └─ native
        |
        └─ I/O error (Other)
        |
        └─ lookup
        |
        └─ first, "path"="a"
        |
        └─ second, "path"="b"
        "#);
    }
    assert!(
        error.downcast_any_ref::<gix_error::ClassificationMarker>().is_none(),
        "classification markers remain transparent"
    );
    assert!(
        error.probable_cause().is::<Message>(),
        "the generic aggregate is not transparent"
    );
    let classified_messages = error
        .classify()
        .filter_map(|item| {
            item.error()
                .downcast_ref::<Message>()
                .map(|error| error.message.as_ref())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        classified_messages,
        ["first", "second"],
        "classification origins are the generic diagnostics themselves"
    );
}

#[test]
fn messages_are_visible_in_reports_unlike_markers() {
    let error = gix_error::ClassificationMarker::with_source(
        Class::Retryable,
        gix_error::not_found("missing reference").with("path", Path::new("HEAD")),
    )
    .and_raise_typed(gix_error::message("lookup failed"));
    insta::assert_debug_snapshot!(error, "the generic diagnostic appears exactly once and its marker remains hidden", @r#"
    lookup failed
    |
    └─ missing reference, "path"="HEAD"
    "#);
    assert!(
        error.probable_cause().is::<Message>(),
        "generic diagnostics participate in cause selection"
    );
}

#[test]
fn metadata_skips_empty_dictionaries_for_plain_messages() {
    let err = Message::new("details")
        .raise_typed()
        .raise(gix_error::message("context"));
    insta::assert_debug_snapshot!(err, "messages without metadata remain visible diagnostics", @"
    context
    |
    └─ details
    ");
    assert!(
        err.metadata().next().is_none(),
        "messages without values contribute no metadata dictionaries"
    );
    assert_eq!(err.iter_errors().count(), 2, "plain messages remain visible errors");

    let err = err.erased();
    assert!(
        err.metadata().next().is_none(),
        "type erasure does not expose empty dictionaries"
    );

    let err = err.into_error();
    assert!(
        err.metadata().next().is_none(),
        "conversion does not expose empty dictionaries"
    );
    assert_eq!(
        err.iter_errors().count(),
        2,
        "conversion preserves messages even when they have no values"
    );
}

#[test]
fn message_builders_preserve_messages_and_replace_the_class() {
    let error = Message::new("details");
    assert!(
        matches!(error.message, Cow::Borrowed("details")),
        "static messages stay borrowed"
    );
    assert_eq!(error.class, None, "new contexts have no implicit classification");
    assert!(error.values.is_empty(), "values are optional");
    insta::assert_debug_snapshot!(error, "unclassified contexts are omitted", @r#"
    Message {
        message: "details",
    }
    "#);
    assert_eq!(
        gix_error::classify(&error).count(),
        0,
        "unclassified contexts are omitted"
    );

    let error = error
        .with_class(Class::Validation)
        .with("input", b"bad\xff".as_slice())
        .with_class(Class::Corruption);
    assert_eq!(
        error.class,
        Some(Class::Corruption),
        "the last class replaces its predecessor"
    );
    assert_eq!(
        error.values["input"],
        MetadataValue::Bytes(b"bad\xff".as_slice().into()),
        "class changes retain diagnostic values"
    );
    insta::assert_debug_snapshot!(error, "reclassification does not manufacture a causal chain", @r#"
    Message {
        message: "details",
        class: Corruption,
        values: {"input": Bytes("bad\xff")},
    }
    "#);
    assert_eq!(
        gix_error::classify(&error).count(),
        1,
        "reclassification does not manufacture a causal chain"
    );

    let error: Message = gix_error::message("missing")
        .with_class(Class::NotFound)
        .with("path", Path::new("HEAD"));
    assert_eq!(
        error.class,
        Some(Class::NotFound),
        "messages can be classified without changing their type"
    );
    assert!(
        matches!(error.message, Cow::Borrowed("missing")),
        "builders keep borrowed storage"
    );
    assert_eq!(
        error.values["path"],
        MetadataValue::Path("HEAD".into()),
        "messages accept metadata"
    );

    let error: Message = gix_error::message!("object {} is missing", 42)
        .with("object", 42)
        .with_class(Class::NotFound);
    assert!(
        matches!(error.message, Cow::Owned(_)),
        "formatted messages retain owned storage"
    );
    insta::assert_debug_snapshot!(error, "builders preserve the formatted diagnostic", @r#"
    Message {
        message: "object 42 is missing",
        class: NotFound,
        values: {"object": I64(42)},
    }
    "#);
    assert_eq!(
        error.values["object"],
        MetadataValue::I64(42),
        "value builders can precede classification"
    );
}

#[test]
fn message_constructors_start_without_a_class_or_values() {
    for error in [
        Message::from("details"),
        Message::from(String::from("details")),
        Message::from(Cow::Borrowed("details")),
        Message::new("details"),
        gix_error::message("details"),
        gix_error::message!("{}", "details"),
    ] {
        insta::allow_duplicates! { insta::assert_debug_snapshot!(format_args!("{}", error.message), "conversion preserves the diagnostic", @"details"); };
        assert_eq!(
            error.class, None,
            "converting a message does not infer a classification"
        );
        assert!(error.values.is_empty(), "conversion does not invent metadata");
    }
}

#[test]
fn class_constructors_create_visible_diagnostics_without_synthetic_sources() {
    let mut diagnostics = Vec::new();
    let cases = [
        (gix_error::validation(String::from("details")), Class::Validation),
        (gix_error::corruption("details"), Class::Corruption),
        (gix_error::not_found("details"), Class::NotFound),
        (gix_error::retryable("details"), Class::Retryable),
        (
            gix_error::allocation_limit("details"),
            Class::ResourceExhaustion(ResourceExhaustionKind::AllocationLimit),
        ),
        (
            gix_error::allocation_failure("details"),
            Class::ResourceExhaustion(ResourceExhaustionKind::AllocationFailure),
        ),
        (
            gix_error::resource_exhaustion(ResourceExhaustionKind::AllocationFailure, "details"),
            Class::ResourceExhaustion(ResourceExhaustionKind::AllocationFailure),
        ),
    ];
    for (error, class) in cases {
        assert_eq!(error.class, Some(class), "each constructor selects its named class");
        insta::allow_duplicates! { insta::assert_debug_snapshot!(format_args!("{}", error), "classification adds no diagnostic noise", @"details"); };
        assert!(
            std::error::Error::source(&error).is_none(),
            "a class is not a synthetic cause"
        );
        let mut classifications = gix_error::classify(&error);
        let classification = classifications.next().expect("the message supplies its own class");
        assert_eq!(
            classification.class(),
            class,
            "borrowed classification recognizes the message"
        );
        assert_eq!(
            classification.io_kind(),
            None,
            "a classified message does not claim a real I/O origin"
        );
        assert!(
            std::ptr::eq(
                classification
                    .error()
                    .downcast_ref::<Message>()
                    .expect("the original type is retained"),
                &error,
            ),
            "the diagnostic itself establishes the classification"
        );
        assert!(classifications.next().is_none(), "each message has at most one class");

        let exn = error.with("path", Path::new("HEAD")).raise_typed();
        assert_eq!(
            exn.classify().next().expect("raised errors retain their class").class(),
            class
        );
        assert_eq!(exn.iter_errors().count(), 1, "a classified message is one visible node");
        diagnostics.push(gix_testtools::redact_debug_snapshot(&exn, &[]));
        assert!(exn.probable_cause().is::<Message>(), "a message leaf remains the cause");
        let error = exn.erased().into_error();
        assert_eq!(
            error
                .classify()
                .next()
                .expect("converted errors retain their class")
                .class(),
            class
        );
        diagnostics.push(gix_testtools::redact_debug_snapshot(&error, &[]));
        assert_eq!(
            error.is_validation(),
            class == Class::Validation,
            "validation predicates inspect messages"
        );
        assert_eq!(
            error.is_corrupted(),
            class == Class::Corruption,
            "corruption predicates inspect messages"
        );
        assert_eq!(
            error.is_not_found(),
            class == Class::NotFound,
            "not-found predicates inspect messages"
        );
        assert_eq!(
            error.is_retryable(),
            class == Class::Retryable,
            "retry predicates inspect messages"
        );
        assert_eq!(
            error.is_resource_exhausted(),
            matches!(class, Class::ResourceExhaustion(_)),
            "resource predicates inspect messages"
        );
        assert_eq!(
            error.metadata().next().expect("messages expose their values")["path"],
            MetadataValue::Path("HEAD".into())
        );
        assert!(
            error.probable_cause().is::<Message>(),
            "conversion preserves the concrete cause"
        );
    }
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(diagnostics, "class constructors create visible diagnostics without synthetic sources", @r#"
        [
            details, "path"="HEAD",
            Message {
                message: "details",
                class: Validation,
                values: {"path": Path("HEAD")},
            },
            details, "path"="HEAD",
            Message {
                message: "details",
                class: Corruption,
                values: {"path": Path("HEAD")},
            },
            details, "path"="HEAD",
            Message {
                message: "details",
                class: NotFound,
                values: {"path": Path("HEAD")},
            },
            details, "path"="HEAD",
            Message {
                message: "details",
                class: Retryable,
                values: {"path": Path("HEAD")},
            },
            details, "path"="HEAD",
            Message {
                message: "details",
                class: ResourceExhaustion(AllocationLimit),
                values: {"path": Path("HEAD")},
            },
            details, "path"="HEAD",
            Message {
                message: "details",
                class: ResourceExhaustion(AllocationFailure),
                values: {"path": Path("HEAD")},
            },
            details, "path"="HEAD",
            Message {
                message: "details",
                class: ResourceExhaustion(AllocationFailure),
                values: {"path": Path("HEAD")},
            },
        ]
        "#);
    } else {
        insta::assert_debug_snapshot!(diagnostics, "class constructors create visible diagnostics without synthetic sources", @r#"
        [
            details, "path"="HEAD",
            details, "path"="HEAD",
            details, "path"="HEAD",
            details, "path"="HEAD",
            details, "path"="HEAD",
            details, "path"="HEAD",
            details, "path"="HEAD",
            details, "path"="HEAD",
            details, "path"="HEAD",
            details, "path"="HEAD",
            details, "path"="HEAD",
            details, "path"="HEAD",
            details, "path"="HEAD",
            details, "path"="HEAD",
        ]
        "#);
    }
}

#[test]
fn offending_input_does_not_require_a_validation_class_or_an_extra_cause() {
    let input = b"ref: invalid\xff\n".as_slice();
    let error = gix_error::corruption("Malformed reference")
        .with("input", input)
        .raise_typed();
    assert_eq!(error.iter_errors().count(), 1, "class and input describe one failure");
    let error = error.erased().into_error();
    assert_eq!(
        error.classify().map(|class| class.class()).collect::<Vec<_>>(),
        [Class::Corruption],
        "attaching input does not invent a validation failure"
    );
    assert_eq!(
        error.metadata().next().expect("the cause retains its input")["input"],
        MetadataValue::Bytes(input.into()),
        "type erasure preserves the original bytes"
    );
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(error, "the diagnostic remains the cause", @r#"
        Message {
            message: "Malformed reference",
            class: Corruption,
            values: {"input": Bytes("ref: invalid\xff\n")},
        }
        "#);
    } else {
        insta::assert_debug_snapshot!(error, "the diagnostic remains the cause", @r#"Malformed reference, "input"="ref: invalid\xff\n""#);
    }
    assert!(
        error.probable_cause().is::<Message>(),
        "the diagnostic remains the cause"
    );
}

#[test]
fn metadata_alias_matches_fields_and_borrowed_iterators() {
    use gix_error::Metadata;

    let values: BTreeMap<Cow<'static, str>, MetadataValue> =
        BTreeMap::from([(Cow::Borrowed("path"), MetadataValue::from("HEAD"))]);
    let values: Metadata = values;
    let context = Message {
        values,
        ..Message::new("context")
    };
    let values: &BTreeMap<Cow<'static, str>, MetadataValue> = &context.values;
    assert_eq!(
        values["path"],
        MetadataValue::from("HEAD"),
        "the public alias and values field are compatible with the underlying map"
    );

    let exception = context.raise_typed();
    insta::assert_debug_snapshot!(exception, "metadata inspection borrows the values displayed with the context", @r#"context, "path"="HEAD""#);
    let dictionary: &Metadata = exception
        .metadata()
        .next()
        .expect("the message context contributes metadata");
    assert!(
        std::ptr::eq(dictionary, &exception.error().values),
        "Exn::metadata() borrows the context's actual values field"
    );

    let error = exception.into_error();
    let context = error
        .downcast_any_ref::<Message>()
        .expect("conversion retains the message context");
    let dictionary: &Metadata = error
        .metadata()
        .next()
        .expect("the converted context contributes metadata");
    assert!(
        std::ptr::eq(dictionary, &context.values),
        "Error::metadata() borrows the converted context's actual values field"
    );
}
