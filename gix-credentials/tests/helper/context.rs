use crate::TestResult;
use bstr::ByteSlice;
use gix_credentials::protocol::{Capability, Context, ContextOptions};

#[test]
fn unsupported_capabilities_are_retained_but_not_announced() -> TestResult {
    let context = Context::from_bytes(
        b"capability[]=state
state[]=helper:opaque-state
continue=1
capability[]=authtype
authtype=Bearer
credential=access-token
ephemeral=1
capability[]=future
",
        Default::default(),
    )?;
    assert_eq!(
        context.capability,
        [
            Capability::Unsupported("state".into()),
            Capability::Authtype,
            Capability::Unsupported("future".into()),
        ],
        "unsupported names remain available for inspection in their original order"
    );
    let expected = b"capability[]=authtype
authtype=Bearer
credential=access-token
ephemeral=1
";
    let mut output = Vec::new();
    context.write_to(&mut output)?;
    assert_eq!(
        output, expected,
        "only supported capabilities and their fields are serialized"
    );
    assert_eq!(
        Context::from_bytes(&output, Default::default())?.capability,
        [Capability::Authtype],
        "unsupported announcements intentionally do not round-trip"
    );
    let next: gix_credentials::helper::NextAction = context.into();
    for action in [next.clone().store(), next.erase()] {
        assert_eq!(
            action.payload().expect("store and erase have a payload"),
            expected.as_bstr(),
            "context-based follow-up actions cannot claim support for discarded capabilities"
        );
    }
    Ok(())
}

#[test]
fn unsupported_capabilities_never_enable_or_inject_fields() -> TestResult {
    for name in [
        "state",
        "authtype",
        "future\ncapability[]=authtype",
        "future\r",
        "future\0",
    ] {
        let context = Context {
            capability: vec![Capability::Unsupported(name.into())],
            authtype: Some("Bearer".into()),
            credential: Some("access-token".into()),
            ephemeral: Some(true),
            ..Default::default()
        };
        let mut output = Vec::new();
        gix_credentials::helper::Action::Get(context).send(&mut output)?;
        assert!(
            output.is_empty(),
            "the unsupported variant is inert regardless of its name or protocol delimiters"
        );
    }
    Ok(())
}

#[test]
fn decoding_unsupported_capabilities_still_validates_the_protocol() {
    for input in [
        b"capability[]=future\0\n".as_slice(),
        b"capability[]=future\rname\n",
        b"capability[]=\xff\n",
    ] {
        assert!(
            Context::from_bytes(input, Default::default()).is_err(),
            "introspection must not bypass protocol and UTF-8 validation"
        );
    }
}

#[test]
fn authtype_fields_roundtrip_only_after_their_capability_and_hide_secrets() -> TestResult {
    let fields = b"authtype=Bearer
credential=access-token
ephemeral=1
";
    for capability in [b"".as_slice(), b"capability[]=unknown\n"] {
        let input = [capability, fields, b"capability[]=authtype\n"].concat();
        let context = Context::from_bytes(&input, Default::default())?;
        assert_eq!(
            context.authtype, None,
            "a later or unknown capability does not enable fields retroactively"
        );
        assert_eq!(context.credential, None, "unnegotiated credentials are discarded");
        assert_eq!(
            context.ephemeral, None,
            "the lifetime flag requires the same capability"
        );
    }
    let input = [b"capability[]=authtype\n".as_slice(), fields].concat();
    let mut context = Context::from_bytes(&input, Default::default())?;
    assert_eq!(
        context.to_bstring(),
        input,
        "capabilities precede all dependent fields during serialization"
    );
    let redacted = context.clone().redacted();
    assert_eq!(
        redacted.credential.as_deref(),
        Some("<redacted>"),
        "token credentials are secrets"
    );
    let err = gix_credentials::protocol::helper_outcome_to_result(
        None,
        gix_credentials::helper::Action::Get(context.clone()),
    )
    .expect_err("a missing helper outcome produces context for diagnostics");
    assert!(
        !err.to_string().contains("access-token"),
        "missing-credentials diagnostics must not expose credentials"
    );
    context.clear_secrets();
    assert_eq!(
        context.credential, None,
        "clearing passwords also clears pre-encoded credentials"
    );
    Ok(())
}

#[test]
fn authtype_protocol_fields_reject_record_injection() {
    for value in ["Bearer\nusername=injected", "Bearer\rusername=injected", "Bearer\0"] {
        for context in [
            Context {
                capability: vec![Capability::Authtype],
                authtype: Some(value.into()),
                ..Default::default()
            },
            Context {
                capability: vec![Capability::Authtype],
                credential: Some(value.into()),
                ..Default::default()
            },
        ] {
            assert!(
                context.write_to(Vec::new()).is_err(),
                "credential fields cannot inject protocol records"
            );
        }
    }
}

#[test]
fn authentication_challenges_survive_a_protocol_roundtrip() -> TestResult {
    let input = b"protocol=https
host=github.com
wwwauth[]=Basic realm=\"GitHub\" domain_hint=\"example\"
wwwauth[]=Bearer realm=\"\xff\"
";
    let context = Context::from_bytes(input, ContextOptions::default())?;
    let mut output = Vec::new();
    context.write_to(&mut output)?;
    assert_eq!(
        output.as_bstr(),
        input.as_bstr(),
        "helpers need every authentication challenge in server order to select a stored account without prompting"
    );
    Ok(())
}

#[test]
fn authentication_challenges_cannot_inject_protocol_fields() {
    for value in [
        b"Basic\0realm=example".as_slice(),
        b"Basic\nusername=other",
        b"Basic\rusername=other",
    ] {
        let context = Context {
            www_authenticate: vec![value.into()],
            ..Default::default()
        };
        assert!(
            context.write_to(Vec::new()).is_err(),
            "server challenges must obey the same protocol protection as other fields"
        );
    }
}

#[test]
fn encode_decode_roundtrip_works_only_for_serializing_fields() {
    for ctx in [
        Context {
            protocol: Some("https".into()),
            host: Some("github.com".into()),
            path: Some("byron/gitoxide".into()),
            username: Some("user".into()),
            password: Some("pass".into()),
            url: Some("https://github.com/byron/gitoxide".into()),
            ..Default::default()
        },
        Context::default(),
    ] {
        let mut buf = Vec::<u8>::new();
        ctx.write_to(&mut buf).unwrap();
        let actual = Context::from_bytes(&buf, ContextOptions::default()).unwrap();
        assert_eq!(actual, ctx, "ctx should encode itself losslessly");
    }
}

mod write_to {
    use gix_credentials::protocol::{Context, ContextOptions};

    #[test]
    fn quit_is_not_serialized_but_can_be_parsed() -> gix_testtools::TestResult {
        let mut buf = Vec::<u8>::new();
        Context {
            quit: Some(true),
            ..Default::default()
        }
        .write_to(&mut buf)?;
        assert_eq!(
            Context::from_bytes(&buf, ContextOptions::default())?,
            Context::default()
        );
        assert_eq!(
            Context::from_bytes(b"quit=true\nurl=https://example.com", ContextOptions::default())?,
            Context {
                quit: Some(true),
                url: Some("https://example.com".into()),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn record_delimiters_are_invalid() {
        let mut error_snapshots = Vec::new();
        for input in [&b"foo\0"[..], b"foo\n", b"foo\r"] {
            let ctx = Context {
                url: Some(input.into()),
                ..Default::default()
            };
            let mut buf = Vec::<u8>::new();
            let err = ctx.write_to(&mut buf).unwrap_err();
            error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
            assert_eq!(err.kind(), std::io::ErrorKind::Other);
        }
        insta::assert_debug_snapshot!(error_snapshots, "record delimiters are invalid", @r#"
        [
            Custom {
                kind: Other,
                error: "url"="foo\0" must not contain null bytes or newlines neither in key nor in value., input="foo\0",
            },
            Custom {
                kind: Other,
                error: "url"="foo\n" must not contain null bytes or newlines neither in key nor in value., input="foo\n",
            },
            Custom {
                kind: Other,
                error: "url"="foo\r" must not contain null bytes or newlines neither in key nor in value., input="foo\r",
            },
        ]
        "#);
    }

    #[test]
    fn carriage_returns_can_be_allowed() {
        let ctx = Context {
            options: ContextOptions {
                protect_protocol: false,
            },
            url: Some(b"https://example.com/with\rreturn".as_slice().into()),
            ..Default::default()
        };
        let mut buf = Vec::new();
        ctx.write_to(&mut buf).expect("CR protection is disabled");
        assert_eq!(buf, b"url=https://example.com/with\rreturn\n");
    }
}

mod from_bytes {
    use gix_credentials::protocol::{Context, ContextOptions};

    #[test]
    fn empty_newlines_cause_skipping_remaining_input() {
        let input = b"protocol=https
host=example.com\n
password=secr3t
username=bob";
        assert_eq!(
            Context::from_bytes(input, ContextOptions::default()).unwrap(),
            Context {
                protocol: Some("https".into()),
                host: Some("example.com".into()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn unknown_field_names_are_skipped() {
        let input = b"protocol=https
unknown=value
username=bob";
        assert_eq!(
            Context::from_bytes(input, ContextOptions::default()).unwrap(),
            Context {
                protocol: Some("https".into()),
                username: Some("bob".into()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn quit_supports_git_config_boolean_values() -> gix_testtools::TestResult {
        for true_value in ["1", "42", "-42", "true", "on", "yes"] {
            let input = format!("quit={true_value}");
            assert_eq!(
                Context::from_bytes(input.as_bytes(), ContextOptions::default())?.quit,
                Some(true),
                "{input}"
            );
        }
        for false_value in ["0", "false", "off", "no"] {
            let input = format!("quit={false_value}");
            assert_eq!(
                Context::from_bytes(input.as_bytes(), ContextOptions::default())?.quit,
                Some(false),
                "{input}"
            );
        }
        Ok(())
    }

    #[test]
    fn null_bytes_when_decoding() {
        let err = Context::from_bytes(b"url=https://foo\0", ContextOptions::default()).unwrap_err();
        insta::assert_debug_snapshot!(err, "null bytes when decoding", @r#""url"="https://foo\0" must not contain null bytes or newlines neither in key nor in value., input="https://foo\0""#);
    }

    #[test]
    fn carriage_returns_can_be_allowed() -> gix_testtools::TestResult {
        let ctx = Context::from_bytes(
            b"url=https://example.com/with\rreturn\n",
            ContextOptions {
                protect_protocol: false,
            },
        )?;
        assert_eq!(ctx.url, Some(b"https://example.com/with\rreturn".as_slice().into()));
        assert_eq!(
            ctx.options,
            ContextOptions {
                protect_protocol: false
            },
            "decoding retains the options needed to encode the context again"
        );

        let mut out = Vec::new();
        ctx.write_to(&mut out)?;
        assert_eq!(out, b"url=https://example.com/with\rreturn\n");
        Ok(())
    }
}
