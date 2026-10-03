use std::{borrow::Cow, collections::BTreeMap, path::Path};

use gix_error::{
    Class, Error, ErrorExt, Message, MetadataValue, ResourceExhaustionKind, ResultExt, classify, corruption, not_found,
};

#[test]
fn input_builder_preserves_representation_and_only_replaces_input() {
    for input in [
        MetadataValue::from(b"bad\xff".as_slice()),
        MetadataValue::from(Path::new("HEAD")),
        MetadataValue::from("invalid"),
        MetadataValue::from(i64::MIN),
        MetadataValue::from(u64::MAX),
        MetadataValue::from(1.5),
        MetadataValue::from(false),
    ] {
        for class in [None, Some(Class::Validation), Some(Class::Corruption)] {
            let mut context = Message::new("invalid input").with("offset", 42_u64);
            context.class = class;
            let expected = context.values.clone();
            let mut message = context.with_input("previous").with_input(input.clone());
            assert_eq!(message.message, "invalid input", "the diagnostic text is unchanged");
            assert_eq!(
                message.class, class,
                "input does not assign or replace a classification"
            );
            assert_eq!(
                message.values.remove("input"),
                Some(input.clone()),
                "new input replaces the previous value without changing its representation"
            );
            assert_eq!(message.values, expected, "unrelated metadata is unchanged");
        }
    }
}

#[test]
#[cfg(any(unix, windows))]
fn command_status_records_program_and_exit_code() {
    #[cfg(unix)]
    let status = {
        use std::os::unix::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(23 << 8)
    };
    #[cfg(windows)]
    let status = {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(23)
    };
    let command = std::process::Command::new("editor");
    let message = Message::new("editor failed")
        .validation()
        .with_input("file")
        .with("program", Path::new("old-editor"))
        .with("exit_status", "old-status")
        .with("exit_code", 1)
        .with_command_status(&command, status)
        .with("stdout", "old-output")
        .with_command_output(
            &command,
            std::process::Output {
                status,
                stdout: b"output\xff".to_vec(),
                stderr: Vec::new(),
            },
        );

    assert_eq!(message.message, "editor failed", "the diagnostic text is unchanged");
    assert_eq!(
        message.class,
        Some(Class::Validation),
        "the classification is unchanged"
    );
    assert_eq!(
        message.values["input"],
        MetadataValue::from("file"),
        "unrelated metadata is retained"
    );
    assert_eq!(
        message.values["program"],
        MetadataValue::Path(Path::new("editor").into()),
        "the program is recorded as a native path without resolution"
    );
    assert_eq!(
        message.values["exit_status"],
        MetadataValue::from(status.to_string()),
        "the full status display replaces the previous value"
    );
    assert_eq!(
        message.values["exit_code"],
        MetadataValue::I64(23),
        "the exit code replaces the previous value"
    );
    assert_eq!(
        message.values["stdout"],
        MetadataValue::from(b"output\xff".as_slice()),
        "captured output replaces previous values without text decoding"
    );
    assert_eq!(
        message.values["stderr"],
        MetadataValue::from(Vec::<u8>::new()),
        "an already-captured empty stream is recorded as bytes"
    );
}

#[test]
#[cfg(unix)]
fn command_status_without_exit_code_removes_stale_code() {
    use std::os::unix::process::ExitStatusExt;

    let status = std::process::ExitStatus::from_raw(15);
    let command = std::process::Command::new("editor");
    let message = Message::new("editor terminated")
        .with("exit_code", 23)
        .with_command_status(&command, status);

    assert_eq!(
        message.values["exit_status"],
        MetadataValue::from(status.to_string()),
        "signal termination retains the full status display"
    );
    assert!(
        !message.values.contains_key("exit_code"),
        "termination without an exit code must not retain a code from an earlier status"
    );
}

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
            .with_input(b"bad\xff".as_slice());
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
fn merged_metadata_combines_contexts_with_more_specific_values_winning() {
    use gix_error::Metadata;

    let input = b"bad\xff".as_slice();
    let exception = Message::new("parse")
        .with_input(input)
        .with("path", Path::new("HEAD"))
        .raise_typed()
        .raise(
            Message::new("decode")
                .with_input("less specific")
                .with("encoding", "bytes"),
        )
        .raise(
            Message::new("configuration")
                .with("key", "gpg.format")
                .with_input("outer"),
        );
    let expected = Metadata::from([
        ("encoding".into(), MetadataValue::from("bytes")),
        ("input".into(), MetadataValue::from(input)),
        ("key".into(), MetadataValue::from("gpg.format")),
        ("path".into(), MetadataValue::from(Path::new("HEAD"))),
    ]);
    let mut merged = exception.metadata_merged();
    assert_eq!(
        merged, expected,
        "causes override contexts while unrelated fields and value types survive"
    );
    merged.clear();
    assert_eq!(
        exception.metadata().count(),
        3,
        "the returned dictionary is independent of the exception"
    );
    assert_eq!(
        exception.erased().metadata_merged(),
        expected,
        "erasure preserves merged metadata"
    );

    let error = Message::new("parse")
        .with_input(input)
        .raise()
        .and_raise(Message::new("configuration").with("key", "gpg.format"));
    let expected = Metadata::from([
        ("input".into(), MetadataValue::from(input)),
        ("key".into(), MetadataValue::from("gpg.format")),
    ]);
    assert_eq!(
        error.metadata_merged(),
        expected,
        "public errors combine callee input with caller keys"
    );
    assert_eq!(
        error.into_exn().metadata_merged(),
        expected,
        "conversion preserves merged metadata"
    );
}

#[test]
fn merged_metadata_visits_native_sources_nested_errors_and_sibling_causes() {
    let first = Message::new("first").with_input(b"first".as_slice()).raise_typed();
    let second = Message::new("second")
        .with_input(b"second\xff".as_slice())
        .raise_typed();
    let exception = gix_error::Exn::raise_all([first, second], Message::new("outer").with_input("outer"));
    let expected = gix_error::Metadata::from([("input".into(), MetadataValue::from(b"second\xff".as_slice()))]);
    assert_eq!(
        exception.metadata_merged(),
        expected,
        "later-visited sibling causes break ties"
    );
    let error = exception.into_error();
    assert_eq!(
        error.metadata_merged(),
        expected,
        "tree and chain representations agree on sibling precedence"
    );

    let error = std::io::Error::other(error)
        .and_raise_typed(Message::new("read").with_input("less specific").with("key", "example"));
    let mut expected = expected;
    expected.insert("key".into(), MetadataValue::from("example"));
    assert_eq!(
        error.metadata_merged(),
        expected,
        "native sources expose metadata from nested public errors"
    );
    assert_eq!(
        error.into_error().metadata_merged(),
        expected,
        "conversion retains native and nested metadata"
    );
}

#[test]
fn merged_metadata_is_empty_without_values() {
    let exception = std::io::Error::from(std::io::ErrorKind::NotFound).and_raise_typed(Message::new("no metadata"));
    assert!(
        exception.metadata_merged().is_empty(),
        "plain contexts and native errors contribute no values"
    );
    assert!(
        exception.into_error().metadata_merged().is_empty(),
        "conversion does not invent metadata"
    );
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

    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    insta::assert_snapshot!(format!("{:#}", err.error()), "the stored error's alternate display omits locations", @r#"custom: lookup, "path"="first": missing"#);
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    insta::assert_snapshot!(format!("{:#}", err.error()), "the stored error's alternate display omits locations", @r#"
    custom
    "#);
    insta::assert_debug_snapshot!(err, @r#"
    custom

    Caused by:
        0: lookup, "path"="first"
        1: missing
        2: read, "path"="second"
        └─0: timed out
    "#);

    let err = err.into_error();
    if cfg!(all(feature = "auto-chain-error", not(feature = "tree-error"))) {
        insta::assert_debug_snapshot!(err, "context leaves classifications intact", @r#"
        custom

        Caused by:
            0: read, "path"="second"
            1: timed out
            2: lookup, "path"="first"
            3: missing
        "#);
    } else {
        insta::assert_debug_snapshot!(err, "context leaves classifications intact", @r#"
        custom

        Caused by:
            0: lookup, "path"="first"
            1: missing
            2: read, "path"="second"
            └─0: timed out
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

    Caused by:
        0: permission denied
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
        [Class::Retryable, Class::PermissionDenied],
        "both classified context and native permission denial retain their classifications"
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
        try reading again, "path"="HEAD"

        Caused by:
            0: permission denied
        "#);
    } else {
        insta::assert_debug_snapshot!(error, "conversion retains the real cause", @r#"
        try reading again, "path"="HEAD"

        Caused by:
            0: permission denied
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
            not_found("first").with("path", "a").raise_typed(),
            not_found("second").with("path", "b").raise_typed(),
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
    insta::assert_debug_snapshot!(error, "predicates inspect generic causes and markers together", @r#"
    outer

    Caused by:
        0: native
        1: I/O error (Other)
        2: lookup
        3: first, "path"="a"
        4: second, "path"="b"
    "#);
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
        outer

        Caused by:
            0: native
            1: I/O error (Other)
            2: lookup
            3: first, "path"="a"
            4: second, "path"="b"
        "#);
    } else {
        insta::assert_debug_snapshot!(error, "classification markers remain transparent", @r#"
        outer

        Caused by:
            0: native
            1: I/O error (Other)
            2: lookup
            3: first, "path"="a"
            4: second, "path"="b"
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
        not_found("missing reference").with("path", Path::new("HEAD")),
    )
    .and_raise_typed(gix_error::message("lookup failed"));
    insta::assert_debug_snapshot!(error, "the generic diagnostic appears exactly once and its marker remains hidden", @r#"
    lookup failed

    Caused by:
        0: missing reference, "path"="HEAD"
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

    Caused by:
        0: details
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
    assert_eq!(classify(&error).count(), 0, "unclassified contexts are omitted");

    let error = error
        .with_class(Class::Validation)
        .with_input(b"bad\xff".as_slice())
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
        classify(&error).count(),
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
fn corrupted_error_raises_a_message_without_extra_causes() {
    for diagnostic in [
        gix_error::message("details"),
        gix_error::message!("invalid record at {}", 42),
    ] {
        let expected_message = diagnostic.message.clone();
        let diagnostic = diagnostic.with_class(Class::NotFound).with("offset", 42_u64);
        let line = line!() + 1;
        let error: Error = diagnostic.corrupted_error();

        assert!(error.is_corrupted(), "the raised error is classified as corruption");
        assert_eq!(
            error
                .classify()
                .map(|classification| classification.class())
                .collect::<Vec<_>>(),
            [Class::Corruption],
            "corruption replaces the previous classification"
        );
        let diagnostic = error
            .downcast_any_ref::<Message>()
            .expect("the message remains downcastable");
        assert_eq!(diagnostic.message, expected_message, "raising preserves the message");
        assert_eq!(
            matches!(diagnostic.message, Cow::Borrowed(_)),
            matches!(expected_message, Cow::Borrowed(_)),
            "raising retains borrowed or owned message storage"
        );
        assert_eq!(
            diagnostic.values["offset"],
            MetadataValue::U64(42),
            "raising preserves diagnostic metadata"
        );
        assert_eq!(error.iter_errors().count(), 1, "raising adds no synthetic cause");
        let source = error
            .iter_errors_with_locations()
            .next()
            .expect("the message is present");
        let location = source.location().expect("raising records the caller's location");
        assert_eq!(
            location.file(),
            file!(),
            "the location belongs to the caller, not the helper"
        );
        assert_eq!(location.line(), line, "the location records the corrupted_error() call");
    }
}

#[test]
fn error_builders_track_the_caller() {
    macro_rules! check {
        ($method:ident($($arg:expr),*)) => {{
            let line = line!();
            let error = gix_error::message("details").$method($($arg),*);
            let source = error
                .iter_errors_with_locations()
                .next()
                .expect("the raised message is present");
            let location = source.location().expect("raising records the caller's location");
            assert_eq!(
                location.file(),
                file!(),
                "{} records the caller's file, not the builder's file",
                stringify!($method)
            );
            assert_eq!(
                location.line(),
                line,
                "{} records the method call's line, not an internal raise() call",
                stringify!($method)
            );
        }};
    }

    check!(corrupted_error());
    check!(validation_error());
    check!(not_found_error());
    check!(retryable_error());
    check!(resource_exhaustion_error(ResourceExhaustionKind::AllocationLimit));
    check!(resource_exhaustion_error(ResourceExhaustionKind::AllocationFailure));
    check!(allocation_limit_error());
    check!(allocation_failure_error());
}

#[test]
fn common_class_builders_work_with_bail_and_ensure() {
    use gix_error::{ExnMessageResult, Result, bail, ensure};

    fn corrupted(offset: usize) -> Result {
        bail!("invalid record at {offset}".corrupted());
    }

    fn invalid(count: usize) -> ExnMessageResult {
        ensure!(count > 0, "count must be positive, got {count}".validation());
        Ok(())
    }

    let error = corrupted(42).expect_err("the record is malformed");
    assert!(error.is_corrupted(), "bail preserves the corruption class");
    assert!(
        !error.is_validation(),
        "stored corruption is not caller-input validation"
    );
    assert_eq!(
        error.error().to_string(),
        "invalid record at 42",
        "classification does not affect display"
    );
    let error = invalid(0).expect_err("zero violates the input constraint");
    assert!(error.is_validation(), "ensure preserves validation in typed exceptions");
    assert!(!error.is_corrupted(), "invalid input does not imply stored corruption");
    assert_eq!(
        error.error().to_string(),
        "count must be positive, got 0",
        "format arguments are preserved"
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
        (corruption("details"), Class::Corruption),
        (not_found("details"), Class::NotFound),
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
        let mut classifications = classify(&error);
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

#[test]
fn offending_input_does_not_require_a_validation_class_or_an_extra_cause() {
    let input = b"ref: invalid\xff\n".as_slice();
    let error = corruption("Malformed reference").with_input(input).raise_typed();
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
    insta::assert_debug_snapshot!(error, "the diagnostic remains the cause", @r#"Malformed reference, "input"="ref: invalid\xff\n""#);
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
