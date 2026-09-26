use gix_credentials::program::main;
use gix_error::Result;
use std::io::Cursor;

#[test]
#[cfg(unix)]
fn invalid_non_utf8_action_is_preserved() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let err = main::Action::try_from(OsString::from_vec(vec![0xff])).expect_err("the action is invalid");
    assert_eq!(
        err.values.get("input"),
        Some(&gix_error::MetadataValue::Bytes(vec![0xff].into())),
        "the invalid action is retained"
    );
}

#[test]
fn context_options_apply_to_input_and_output() {
    let input = b"url=https://github.com/with\rreturn\n";
    let mut output = Vec::new();
    let options = gix_credentials::protocol::ContextOptions {
        protect_protocol: false,
    };

    main(
        ["get".into()],
        Cursor::new(input),
        &mut output,
        options,
        |_action, context| -> Result<Option<gix_credentials::protocol::Context>> {
            assert_eq!(
                context.url.as_ref().map(|url| url.as_slice()),
                Some(&input[4..input.len() - 1])
            );
            Ok(Some(gix_credentials::protocol::Context {
                username: Some("user\rname".into()),
                ..Default::default()
            }))
        },
    )
    .expect("carriage returns are allowed");

    assert!(output.windows(9).any(|window| window == b"user\rname"));
}

#[test]
fn protocol_and_host_without_url_is_valid() {
    let input = b"protocol=https\nhost=github.com\n";
    let mut output = Vec::new();

    let mut called = false;
    let result = main(
        ["get".into()],
        Cursor::new(input),
        &mut output,
        gix_credentials::protocol::ContextOptions::default(),
        |_action, context| -> Result<Option<gix_credentials::protocol::Context>> {
            assert_eq!(context.protocol.as_deref(), Some("https"));
            assert_eq!(context.host.as_deref(), Some("github.com"));
            assert_eq!(context.url, None, "the URL isn't automatically populated");
            called = true;

            Ok(None)
        },
    );

    // This should fail because our mock helper returned None (no credentials found)
    // but it should NOT fail because of missing URL
    let err = result.expect_err("missing credentials must fail");
    insta::assert_debug_snapshot!(err, "protocol and host without url is valid", @r#"Credentials for "https://github.com" could not be obtained"#);
    assert!(err.is_not_found());
    assert!(
        called,
        "The helper gets called, but as nothing is provided in the function it ultimately fails"
    );
}

#[test]
fn missing_protocol_with_only_host_or_protocol_fails() {
    let mut error_snapshots = Vec::new();
    for input in ["host=github.com\n", "protocol=https\n"] {
        let mut output = Vec::new();

        let mut called = false;
        let result = main(
            ["get".into()],
            Cursor::new(input),
            &mut output,
            gix_credentials::protocol::ContextOptions::default(),
            |_action, _context| -> Result<Option<gix_credentials::protocol::Context>> {
                called = true;
                Ok(None)
            },
        );

        let err = result.expect_err("incomplete URL must fail validation");
        error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
        assert!(err.is_validation());
        assert!(!called, "the context is lacking, hence nothing gets called");
    }
    insta::assert_debug_snapshot!(error_snapshots, "missing protocol with only host or protocol fails", @"
    [
        Either 'url' field or both 'protocol' and 'host' fields must be provided,
        Either 'url' field or both 'protocol' and 'host' fields must be provided,
    ]
    ");
}

#[test]
fn url_alone_is_valid() {
    let input = b"url=https://github.com\n";
    let mut output = Vec::new();

    let mut called = false;
    let result = main(
        ["get".into()],
        Cursor::new(input),
        &mut output,
        gix_credentials::protocol::ContextOptions::default(),
        |_action, context| -> Result<Option<gix_credentials::protocol::Context>> {
            called = true;
            assert_eq!(context.url.unwrap(), "https://github.com");
            assert_eq!(context.host, None, "not auto-populated");
            assert_eq!(context.protocol, None, "not auto-populated");

            Ok(None)
        },
    );

    // This should fail because our mock helper returned None (no credentials found)
    // but it should NOT fail because of missing URL
    let err = result.expect_err("missing credentials must fail");
    insta::assert_debug_snapshot!(err, "url alone is valid", @r#"Credentials for "https://github.com" could not be obtained"#);
    assert!(err.is_not_found());
    assert!(called);
}
