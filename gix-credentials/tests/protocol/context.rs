mod destructure_url_in_place {
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
    fn passwords_are_placed_in_context_too() -> crate::Result {
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
    fn protocol_and_host_with_path_without_url_constructs_full_url() {
        let mut ctx = Context {
            protocol: Some("https".into()),
            host: Some("github.com".into()),
            path: Some("org/repo".into()),
            username: Some("user".into()),
            password: Some("pass-to-be-ignored".into()),
            ..Default::default()
        };
        ctx.destructure_url_in_place(false)
            .expect("should work with protocol, host and path");

        assert_eq!(
            ctx.url.unwrap(),
            "https://user@github.com/org/repo",
            "URL should be constructed from all provided fields, except password"
        );
        // Original fields should be preserved
        assert_eq!(ctx.protocol.as_deref(), Some("https"));
        assert_eq!(ctx.host.as_deref(), Some("github.com"));
        assert_eq!(ctx.path.unwrap(), "org/repo");
    }

    #[test]
    fn missing_protocol_or_host_without_url_fails() {
        let mut ctx_no_protocol = Context {
            host: Some("github.com".into()),
            ..Default::default()
        };
        assert_eq!(
            ctx_no_protocol.destructure_url_in_place(false).unwrap_err().to_string(),
            "Either 'url' field or both 'protocol' and 'host' fields must be provided"
        );

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
            "Password for https://host: "
        );
    }
}

mod to_url {
    use gix_credentials::protocol::Context;

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
