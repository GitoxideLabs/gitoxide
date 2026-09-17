use std::path::Path;

use gix_error::{Error, ErrorExt, Metadata, NotFoundError, ResultExt, Value};

#[test]
fn scalar_values_are_lossless_and_keys_are_local_to_a_context() {
    let metadata = Metadata::new("details")
        .with("signed", i64::MIN)
        .with("unsigned", u64::MAX)
        .with("float", 1.5)
        .with("bytes", b"ref\xff".as_slice())
        .with("path", Path::new("objects"))
        .with("text", "line\nbreak")
        .with("flag", false)
        .with("flag", true);
    assert_eq!(metadata.values["signed"], Value::I64(i64::MIN));
    assert_eq!(metadata.values["unsigned"], Value::U64(u64::MAX));
    assert_eq!(metadata.values["float"], Value::F64(1.5));
    assert_eq!(metadata.values["bytes"], Value::Bytes(b"ref\xff".as_slice().into()));
    assert_eq!(metadata.values["path"], Value::Path(Path::new("objects").into()));
    assert_eq!(
        metadata.values["flag"],
        Value::Bool(true),
        "the last value replaces its predecessor"
    );
    assert_eq!(
        Metadata::new("details")
            .with("z", "line\nbreak")
            .with("a", 2)
            .to_string(),
        "details, \"a\"=2, \"z\"=\"line\\nbreak\"",
        "keys are ordered and text is escaped"
    );
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(b"ref\xff".to_vec()));
        assert_eq!(
            Value::from(path.as_path()),
            Value::Path(path),
            "paths retain native bytes"
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        let path = std::path::PathBuf::from(std::ffi::OsString::from_wide(&[0xd800]));
        assert_eq!(
            Value::from(path.as_path()),
            Value::Path(path),
            "paths retain native code units"
        );
    }

    insta::assert_snapshot!(metadata, "validate Display", @r#"details, "bytes"="ref\xff", "flag"=true, "float"=1.5, "path"="objects", "signed"=-9223372036854775808, "text"="line\nbreak", "unsigned"=18446744073709551615"#);
    insta::assert_debug_snapshot!(metadata, "validate Display", @r#"
    Metadata {
        message: "details",
        values: {
            "bytes": Bytes(
                "ref\xff",
            ),
            "flag": Bool(
                true,
            ),
            "float": F64(
                1.5,
            ),
            "path": Path(
                "objects",
            ),
            "signed": I64(
                -9223372036854775808,
            ),
            "text": String(
                "line\nbreak",
            ),
            "unsigned": U64(
                18446744073709551615,
            ),
        },
    }
    "#);
}

#[test]
fn metadata_contexts_preserve_causes_and_remain_separate_through_conversion() {
    let missing = NotFoundError::new("missing").and_raise(Metadata::new("lookup").with("path", "first"));
    let retry =
        std::io::Error::from(std::io::ErrorKind::TimedOut).and_raise(Metadata::new("read").with("path", "second"));
    let err = Error::from_error(super::ErrorWithSource("custom", missing.into_error()))
        .raise()
        .chain(retry);
    let paths = |metadata: &Metadata| metadata.values["path"].clone();
    assert_eq!(
        err.metadata().map(paths).collect::<Vec<_>>(),
        [Value::from("second"), Value::from("first")],
        "native sources and explicit child contexts share the existing traversal order"
    );
    assert!(err.is_not_found() && err.can_retry());

    insta::assert_snapshot!(err, @"custom");
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
    assert!(
        err.is_not_found() && err.can_retry(),
        "context leaves classifications intact"
    );
    assert_eq!(
        err.metadata().map(paths).collect::<Vec<_>>(),
        [Value::from("second"), Value::from("first")]
    );
    assert!(
        err.downcast_any_ref::<NotFoundError>().is_some(),
        "the concrete cause remains accessible"
    );

    let ok: Result<(), std::io::Error> = Ok(());
    assert!(ok.or_raise(|| -> Metadata { panic!("context must be lazy") }).is_ok());
}
