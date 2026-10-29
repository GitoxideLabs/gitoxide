mod destructure_url_in_place {
    use crate::TestResult;
    use gix_credentials::protocol::Context;

    fn url_ctx(url: &str) -> Context {
        Context {
            url: Some(url.into()),
            ..Default::default()
        }
    }

    fn assert_eq_parts(
        url: &str,
        proto: &str,
        user: impl Into<Option<&'static str>>,
        host: &str,
        path: impl Into<Option<&'static str>>,
        use_http_path: bool,
    ) {
        let mut ctx = url_ctx(url);
        ctx.destructure_url_in_place(use_http_path).expect("splitting works");
        assert_eq!(ctx.protocol.expect("set proto"), proto);
        match user.into() {
            Some(expected) => assert_eq!(ctx.username.expect("set user"), expected),
            None => assert!(ctx.username.is_none()),
        }
        assert_eq!(ctx.host.expect("set host"), host);
        match path.into() {
            Some(expected) => assert_eq!(ctx.path.expect("set path"), expected),
            None => assert!(ctx.path.is_none()),
        }
    }

    #[test]
    fn parts_are_verbatim_with_non_http_url() {
        // path is always used for non-http
        assert_eq_parts("ssh://user@host:21/path", "ssh", "user", "host:21", "path", false);
        assert_eq_parts("ssh://host.org/path", "ssh", None, "host.org", "path", true);
    }

    #[test]
    fn passwords_are_placed_in_context_too() -> TestResult {
        let mut ctx = url_ctx("http://user:password@host/path");
        ctx.destructure_url_in_place(false)?;
        assert_eq!(ctx.password.as_deref(), Some("password"));
        Ok(())
    }

    #[test]
    fn http_and_https_ignore_the_path_by_default() {
        assert_eq_parts(
            "http://user@example.com/path",
            "http",
            Some("user"),
            "example.com",
            None,
            false,
        );
        assert_eq_parts(
            "https://github.com/byron/gitoxide",
            "https",
            None,
            "github.com",
            None,
            false,
        );
        assert_eq_parts(
            "https://github.com/byron/gitoxide/",
            "https",
            None,
            "github.com",
            "byron/gitoxide",
            true,
        );
    }

    #[test]
    fn http_path_is_decoded_when_used() {
        assert_eq_parts("https://example.com/a%2Fb/", "https", None, "example.com", "a/b", true);
    }

    #[test]
    fn component_paths_retain_existing_http_path_semantics() -> TestResult {
        for protocol in ["https", "ssh"] {
            for use_http_path in [false, true] {
                for (path, normalized) in [("/repo/", Some("repo")), ("/", None)] {
                    let mut ctx = Context {
                        protocol: Some(protocol.into()),
                        host: Some("example.com".into()),
                        path: Some(path.into()),
                        ..Default::default()
                    };
                    ctx.destructure_url_in_place(use_http_path)?;
                    let expected = if protocol == "https" && !use_http_path {
                        Some(path)
                    } else {
                        normalized
                    };
                    assert_eq!(
                        ctx.path.as_deref().map(Vec::as_slice),
                        expected.map(str::as_bytes),
                        "supplied HTTP paths are retained when disabled; used paths are relative and root paths absent"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn an_explicit_url_still_replaces_supplied_components() -> TestResult {
        let mut ctx = Context {
            url: Some("https://url-user:url-password@url.example/repo".into()),
            protocol: Some("http".into()),
            host: Some("components.example".into()),
            username: Some("component-user".into()),
            password: Some("component-password".into()),
            ..Default::default()
        };
        ctx.destructure_url_in_place(true)?;
        assert_eq!(
            ctx.protocol.as_deref(),
            Some("https"),
            "an explicit URL defines the protocol"
        );
        assert_eq!(
            ctx.host.as_deref(),
            Some("url.example"),
            "an explicit URL defines the host"
        );
        assert_eq!(
            ctx.username.as_deref(),
            Some("url-user"),
            "an explicit URL defines the user"
        );
        assert_eq!(
            ctx.password.as_deref(),
            Some("url-password"),
            "an explicit URL defines the password"
        );
        Ok(())
    }

    #[test]
    fn protocol_and_host_with_path_without_url_constructs_full_url() -> TestResult {
        let mut ctx = Context {
            protocol: Some("https".into()),
            host: Some("github.com".into()),
            path: Some("org/repo".into()),
            username: Some("user".into()),
            password: Some("pass-to-be-ignored".into()),
            ..Default::default()
        };
        ctx.destructure_url_in_place(false)?;

        assert_eq!(
            ctx.url.unwrap(),
            "https://user@github.com/org/repo",
            "URL should be constructed from all provided fields, except password"
        );
        // Original fields should be preserved
        assert_eq!(ctx.protocol.as_deref(), Some("https"));
        assert_eq!(ctx.host.as_deref(), Some("github.com"));
        assert_eq!(ctx.path.unwrap(), "org/repo");
        Ok(())
    }

    #[test]
    fn missing_protocol_or_host_without_url_fails() {
        let mut ctx_no_protocol = Context {
            host: Some("github.com".into()),
            ..Default::default()
        };
        insta::assert_debug_snapshot!(ctx_no_protocol.destructure_url_in_place(false).expect_err("missing protocol or host without url fails"), "missing protocol or host without url fails", @"Either 'url' field or both 'protocol' and 'host' fields must be provided");

        let mut ctx_no_host = Context {
            protocol: Some("https".into()),
            ..Default::default()
        };
        assert!(ctx_no_host.destructure_url_in_place(false).is_err());
    }
}

mod to_prompt {
    use gix_credentials::protocol::Context;

    #[test]
    fn no_scheme_means_no_url() {
        assert_eq!(Context::default().to_prompt("Username"), "Username: ");
    }

    #[test]
    fn any_scheme_means_url_is_included() {
        assert_eq!(
            Context {
                protocol: Some("https".into()),
                host: Some("host".into()),
                ..Default::default()
            }
            .to_prompt("Password"),
            "Password for 'https://host': "
        );
    }
}

mod to_url {
    use crate::TestResult;
    use gix_credentials::protocol::Context;

    #[test]
    fn component_delimiters_cannot_change_the_credential_identity() -> TestResult {
        for protocol in ["http", "https", "ssh", "git"] {
            for user in [
                "github.com/",
                "victim@trusted.example/",
                "victim@trusted.example?",
                "victim@trusted.example#",
                "user:password",
                "user%2Fname",
                "jörg",
            ] {
                let mut ctx = Context {
                    protocol: Some(protocol.into()),
                    host: Some("evil.example:8443".into()),
                    username: Some(user.into()),
                    password: Some("already-supplied".into()),
                    path: Some("repo%2Fwith space/?#@".into()),
                    ..Default::default()
                };
                let encoded = ctx.to_url().expect("a protocol is present");
                let parsed = gix_url::parse(&encoded)?;
                assert_eq!(parsed.user(), Some(user), "user delimiters remain part of the username");
                assert_eq!(parsed.host(), Some("evil.example"), "the requested host cannot change");
                assert_eq!(
                    parsed.port,
                    Some(8443),
                    "the explicit port remains an authority component"
                );
                assert_eq!(
                    parsed.password(),
                    None,
                    "the synthesized URL never contains the password"
                );
                assert_eq!(
                    parsed.path, "/repo%2Fwith space/?#@",
                    "decoded path bytes survive serialization"
                );

                let mut expected = ctx.clone();
                expected.url = Some(encoded);
                ctx.destructure_url_in_place(true)?;
                assert_eq!(
                    ctx, expected,
                    "constructing a URL does not reinterpret supplied credentials"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn hosts_keep_ports_ipv6_and_scheme_specific_percent_encoding() -> TestResult {
        for (protocol, host, encoded_host) in [
            ("https", "[::1]:8443", "[::1]:8443"),
            ("https", "[fe80::1%25eth0]:8443", "[fe80::1%25eth0]:8443"),
            ("https", "exa%20mple.example:8443", "exa%20mple.example:8443"),
            ("ssh", "[fe80::1%eth0]:8443", "[fe80::1%25eth0]:8443"),
            ("ssh", "exa%mple.example:8443", "exa%25mple.example:8443"),
            (
                "https",
                "trusted.example/path@evil.example",
                "trusted.example%2Fpath%40evil.example",
            ),
        ] {
            let mut ctx = Context {
                protocol: Some(protocol.into()),
                host: Some(host.into()),
                path: Some("repo".into()),
                ..Default::default()
            };
            assert_eq!(
                ctx.to_url().expect("a protocol is present"),
                format!("{protocol}://{encoded_host}/repo"),
                "host encoding preserves ports and IPv6 without introducing authority delimiters"
            );
            ctx.destructure_url_in_place(true)?;
            assert_eq!(
                ctx.host.as_deref(),
                Some(host),
                "the original host field reaches helpers intact"
            );
        }
        Ok(())
    }

    #[test]
    fn protocol_delimiters_cannot_introduce_an_authority() {
        let mut ctx = Context {
            protocol: Some("https://trusted.example/".into()),
            host: Some("evil.example".into()),
            ..Default::default()
        };
        assert!(
            ctx.destructure_url_in_place(false).is_err(),
            "a malformed protocol cannot select a different host"
        );
    }

    #[test]
    fn no_protocol_is_nothing() {
        assert_eq!(Context::default().to_url(), None);
    }
    #[test]
    fn protocol_alone_is_enough() {
        assert_eq!(
            Context {
                protocol: Some("https".into()),
                ..Default::default()
            }
            .to_url()
            .unwrap(),
            "https://"
        );
    }
    #[test]
    fn username_is_appended() {
        assert_eq!(
            Context {
                protocol: Some("https".into()),
                username: Some("user".into()),
                ..Default::default()
            }
            .to_url()
            .unwrap(),
            "https://user@"
        );
    }
    #[test]
    fn host_is_appended() {
        assert_eq!(
            Context {
                protocol: Some("https".into()),
                host: Some("host".into()),
                ..Default::default()
            }
            .to_url()
            .unwrap(),
            "https://host"
        );
    }
    #[test]
    fn path_is_appended_with_leading_slash_placed_as_needed() {
        assert_eq!(
            Context {
                protocol: Some("file".into()),
                path: Some("dir/git".into()),
                ..Default::default()
            }
            .to_url()
            .unwrap(),
            "file:///dir/git"
        );
        assert_eq!(
            Context {
                protocol: Some("file".into()),
                path: Some("/dir/git".into()),
                ..Default::default()
            }
            .to_url()
            .unwrap(),
            "file:///dir/git"
        );
    }

    #[test]
    fn all_fields_with_port_but_password_is_never_shown() {
        assert_eq!(
            Context {
                protocol: Some("https".into()),
                username: Some("user".into()),
                password: Some("secret".into()),
                host: Some("example.com:8080".into()),
                path: Some("GitoxideLabs/gitoxide".into()),
                ..Default::default()
            }
            .to_url()
            .unwrap(),
            "https://user@example.com:8080/GitoxideLabs/gitoxide"
        );
    }
}
