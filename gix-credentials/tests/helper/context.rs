use gix_credentials::protocol::{Context, ContextOptions};

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
    fn quit_is_not_serialized_but_can_be_parsed() {
        let mut buf = Vec::<u8>::new();
        Context {
            quit: Some(true),
            ..Default::default()
        }
        .write_to(&mut buf)
        .unwrap();
        assert_eq!(
            Context::from_bytes(&buf, ContextOptions::default()).unwrap(),
            Context::default()
        );
        assert_eq!(
            Context::from_bytes(b"quit=true\nurl=https://example.com", ContextOptions::default()).unwrap(),
            Context {
                quit: Some(true),
                url: Some("https://example.com".into()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn record_delimiters_are_invalid() {
        for input in [&b"foo\0"[..], b"foo\n", b"foo\r"] {
            let ctx = Context {
                url: Some(input.into()),
                ..Default::default()
            };
            let mut buf = Vec::<u8>::new();
            let err = ctx.write_to(&mut buf).unwrap_err();
            assert_eq!(err.kind(), std::io::ErrorKind::Other);
        }
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
    fn quit_supports_git_config_boolean_values() {
        for true_value in ["1", "42", "-42", "true", "on", "yes"] {
            let input = format!("quit={true_value}");
            assert_eq!(
                Context::from_bytes(input.as_bytes(), ContextOptions::default())
                    .unwrap()
                    .quit,
                Some(true),
                "{input}"
            );
        }
        for false_value in ["0", "false", "off", "no"] {
            let input = format!("quit={false_value}");
            assert_eq!(
                Context::from_bytes(input.as_bytes(), ContextOptions::default())
                    .unwrap()
                    .quit,
                Some(false),
                "{input}"
            );
        }
    }

    #[test]
    fn null_bytes_when_decoding() {
        let err = Context::from_bytes(b"url=https://foo\0", ContextOptions::default()).unwrap_err();
        assert!(err.message.contains("must not contain null bytes or newlines"));
    }

    #[test]
    fn carriage_returns_can_be_allowed() {
        let ctx = Context::from_bytes(
            b"url=https://example.com/with\rreturn\n",
            ContextOptions {
                protect_protocol: false,
            },
        )
        .expect("CR protection is disabled");
        assert_eq!(ctx.url, Some(b"https://example.com/with\rreturn".as_slice().into()));
        assert_eq!(
            ctx.options,
            ContextOptions {
                protect_protocol: false
            },
            "decoding retains the options needed to encode the context again"
        );

        let mut out = Vec::new();
        ctx.write_to(&mut out)
            .expect("retained options allow the same value to be encoded again");
        assert_eq!(out, b"url=https://example.com/with\rreturn\n");
    }
}
