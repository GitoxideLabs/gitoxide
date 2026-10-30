mod invoke {
    use crate::TestResult;
    use bstr::ByteSlice;
    use gix_credentials::{
        Program,
        helper::{Action, Cascade},
        protocol,
        protocol::Context,
    };
    use gix_sec::identity::Account;

    #[test]
    fn reconstructed_urls_cannot_change_the_host_sent_to_helpers() -> TestResult {
        for user in ["victim@trusted.example?", "victim@trusted.example/", "github.com/"] {
            let outcome = Cascade::default()
                .extend([Program::from_custom_definition(
                    r#"!f() {
                        while IFS= read -r line; do
                            case "$line" in host=*) printf 'password=%s\n' "${line#host=}" ;; esac
                        done
                    }; f"#,
                )?])
                .invoke(
                    Action::Get(Context {
                        protocol: Some("https".into()),
                        host: Some("evil.example".into()),
                        username: Some(user.into()),
                        ..Default::default()
                    }),
                    gix_prompt::Options {
                        mode: gix_prompt::Mode::Disable,
                        askpass: None,
                    },
                )?
                .expect("the helper reports the host for which it was asked to obtain credentials");
            assert_eq!(
                outcome.identity,
                identity(user, "evil.example"),
                "the helper sees the requested host and the complete username; otherwise URL delimiters in \
                 the username could redirect credential lookup to trusted.example or github.com, exposing \
                 that host's credentials when the caller authenticates to evil.example"
            );
        }
        Ok(())
    }

    #[test]
    fn invalid_authentication_challenges_fail_without_helpers() {
        let mut error_snapshots = Vec::new();
        for value in [
            b"Basic realm=\"a\rb\"".as_slice(),
            b"Basic\nusername=other",
            b"Basic\0realm=example",
        ] {
            let err = invoke_cascade(
                [],
                Action::Get(Context {
                    url: Some("https://example.com/repo".into()),
                    www_authenticate: vec![value.into()],
                    ..Default::default()
                }),
            )
            .expect_err("malformed authentication challenges must fail without panicking");
            error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
            assert!(
                err.downcast_any_ref::<std::io::Error>().is_some(),
                "protocol validation must run even when no helper is configured and prompting is disabled"
            );
        }
        insta::assert_debug_snapshot!(error_snapshots, "invalid authentication challenges fail without helpers", @r#"
        [
            I/O error (Other)
            
            Caused by:
                0: "wwwauth[]"="Basic realm=\"a\rb\"" must not contain null bytes or newlines neither in key nor in value., input="Basic realm=\"a\rb\"",
            I/O error (Other)
            
            Caused by:
                0: "wwwauth[]"="Basic\nusername=other" must not contain null bytes or newlines neither in key nor in value., input="Basic\nusername=other",
            I/O error (Other)
            
            Caused by:
                0: "wwwauth[]"="Basic\0realm=example" must not contain null bytes or newlines neither in key nor in value., input="Basic\0realm=example",
        ]
        "#);
    }

    #[test]
    fn a_helper_closing_its_input_does_not_prevent_fallback_with_challenges() -> TestResult {
        let outcome = Cascade::default()
            .extend([
                Program::from_custom_definition("!f() { exit 1; }; f")?,
                Program::from_custom_definition(
                    "!f() { cat >/dev/null; printf 'username=user\\npassword=pass\\n'; }; f",
                )?,
            ])
            .invoke(
                Action::Get(Context {
                    url: Some("https://example.com/repo".into()),
                    // Exceed the pipe buffer so the first helper's early exit is observed while writing.
                    www_authenticate: vec![vec![b'x'; 1024 * 1024].into()],
                    ..Default::default()
                }),
                gix_prompt::Options {
                    mode: gix_prompt::Mode::Disable,
                    askpass: None,
                },
            )?
            .expect("the fallback helper supplies a complete credential");
        assert_eq!(
            outcome.identity,
            identity("user", "pass"),
            "a helper that closes its input cannot prevent the next helper from supplying credentials"
        );
        Ok(())
    }

    #[test]
    fn authentication_challenges_reach_all_helpers_until_credentials_are_complete() -> TestResult {
        let outcome = Cascade::default()
            .extend([
                Program::from_custom_definition("!f() { cat >/dev/null; echo username=user; }; f")?,
                Program::from_custom_definition(
                    r#"!f() {
                        while IFS= read -r line; do
                            if test "$line" = 'wwwauth[]=Basic realm="example"'; then
                                echo password=pass
                            fi
                        done
                    }; f"#,
                )?,
            ])
            .invoke(
                Action::Get(Context {
                    url: Some("https://example.com/repo".into()),
                    www_authenticate: vec![r#"Basic realm="example""#.into()],
                    ..Default::default()
                }),
                gix_prompt::Options {
                    mode: gix_prompt::Mode::Disable,
                    askpass: None,
                },
            )?
            .expect("both helpers contribute to the credential");
        assert_eq!(
            outcome.identity,
            identity("user", "pass"),
            "the second helper receives the challenge"
        );
        let context = Context::try_from(&outcome.next)?;
        assert!(
            context.www_authenticate.is_empty(),
            "completed credentials do not carry authentication challenges into store or erase"
        );
        Ok(())
    }

    #[test]
    fn authtype_credentials_are_used_if_the_capability_was_announced() -> TestResult {
        let outcome = Cascade::default()
            .extend([
                Program::from_custom_definition(
                    r#"!f() {
                        while IFS= read -r line; do
                            if test "$line" = 'capability[]=authtype'; then
                                printf 'capability[]=authtype\nauthtype=Bearer\ncredential=token\nephemeral=1\n'
                            fi
                        done
                    }; f"#,
                )?,
                Program::from_custom_definition("!f() { cat >/dev/null; echo username=user; echo password=pass; }; f")?,
            ])
            .invoke(
                Action::Get(Context {
                    capability: vec!["authtype".into()],
                    url: Some("https://example.com/repo".into()),
                    www_authenticate: vec!["Bearer".into()],
                    ..Default::default()
                }),
                gix_prompt::Options {
                    mode: gix_prompt::Mode::Disable,
                    askpass: None,
                },
            )?
            .expect("the first helper supplies a complete credential");
        assert_eq!(
            outcome.identity,
            Account {
                username: "".into(),
                password: "token".into(),
                oauth_refresh_token: None,
                authtype: Some("Bearer".into()),
            },
            "a credential with its authtype is complete, so the second helper isn't asked"
        );
        let context = Context::try_from(&outcome.next)?;
        assert_eq!(
            (context.authtype.as_deref(), context.ephemeral),
            (Some("Bearer"), Some(true)),
            "store and erase tell helpers which credential was used and that it should not be stored"
        );
        assert!(
            context.www_authenticate.is_empty(),
            "completed credentials do not carry authentication challenges into store or erase"
        );
        Ok(())
    }

    #[test]
    fn authtype_credentials_are_ignored_if_the_capability_was_not_announced() -> TestResult {
        let outcome = Cascade::default()
            .extend([
                Program::from_custom_definition(
                    "!f() { cat >/dev/null; printf 'authtype=Bearer\\ncredential=token\\n'; }; f",
                )?,
                Program::from_custom_definition("!f() { cat >/dev/null; echo username=user; echo password=pass; }; f")?,
            ])
            .invoke(
                Action::get_for_url("https://example.com/repo"),
                gix_prompt::Options {
                    mode: gix_prompt::Mode::Disable,
                    askpass: None,
                },
            )?
            .expect("the second helper supplies a complete credential");
        assert_eq!(
            outcome.identity,
            identity("user", "pass"),
            "callers that can't send an `authtype` don't receive one"
        );
        Ok(())
    }

    #[test]
    fn credentials_are_filled_in_one_by_one_and_stop_when_complete() -> TestResult {
        let actual = invoke_cascade(["username", "password", "custom-helper"], action_get())?.expect("credentials");
        assert_eq!(actual.identity, identity("user", "pass"));
        Ok(())
    }

    #[test]
    fn disabled_protocol_protection_is_preserved_for_the_next_action() -> TestResult {
        let actual = Cascade {
            context_options: protocol::ContextOptions {
                protect_protocol: false,
            },
            ..Default::default()
        }
        .extend(fixtures(["carriage-return"]))
        .invoke(
            action_get(),
            gix_prompt::Options {
                mode: gix_prompt::Mode::Disable,
                askpass: None,
            },
        )?
        .expect("credentials are complete");

        assert_eq!(actual.identity, identity("user\rname", "pass"));
        let context: Context = (&actual.next).try_into()?;
        assert_eq!(context.username.as_deref(), Some("user\rname"));
        let mut serialized = Vec::new();
        actual.next.store().send(&mut serialized)?;
        assert!(serialized.contains_str("username=user\rname"));
        Ok(())
    }

    #[test]
    fn usernames_in_urls_are_kept_if_the_helper_does_not_overwrite_it() -> TestResult {
        let actual = invoke_cascade(
            ["password", "custom-helper"],
            Action::get_for_url("ssh://git@host.org/path"),
        )?
        .expect("credentials");
        assert_eq!(actual.identity, identity("git", "pass"));
        Ok(())
    }

    #[test]
    fn partial_credentials_can_be_overwritten_by_complete_ones() -> TestResult {
        let actual = invoke_cascade(["username", "custom-helper"], action_get())?.expect("credentials");
        assert_eq!(actual.identity, identity("user-script", "pass-script"));
        Ok(())
    }

    #[test]
    fn failing_helpers_for_filling_dont_interrupt() -> TestResult {
        let actual = invoke_cascade(["fail", "custom-helper"], action_get())?.expect("credentials");
        assert_eq!(actual.identity, identity("user-script", "pass-script"));
        Ok(())
    }

    #[test]
    fn urls_are_split_in_get_to_support_scripts() -> TestResult {
        let actual = invoke_cascade(
            ["reflect", "custom-helper"],
            Action::get_for_url("https://example.com:8080/path/git/"),
        )?
        .expect("credentials");

        let ctx: Context = (&actual.next).try_into()?;
        assert_eq!(ctx.protocol.as_deref().expect("protocol"), "https");
        assert_eq!(ctx.host.as_deref().expect("host"), "example.com:8080");
        assert_eq!(ctx.path.as_deref().expect("path").as_bstr(), "path/git");
        Ok(())
    }

    #[test]
    fn urls_are_split_in_get_but_can_skip_the_path_in_host_only_urls() -> TestResult {
        let actual = invoke_cascade(["reflect", "custom-helper"], Action::get_for_url("http://example.com"))?
            .expect("credentials");

        let ctx: Context = (&actual.next).try_into()?;
        assert_eq!(ctx.protocol.as_deref().expect("protocol"), "http");
        assert_eq!(ctx.host.as_deref().expect("host"), "example.com");
        assert_eq!(ctx.path, None);
        Ok(())
    }

    #[test]
    fn helpers_can_set_any_context_value() -> TestResult {
        let actual = invoke_cascade(
            ["all-but-credentials", "custom-helper"],
            Action::get_for_url("http://github.com"),
        )?
        .expect("credentials");

        let ctx: Context = (&actual.next).try_into()?;
        assert_eq!(ctx.protocol.as_deref().expect("protocol"), "ftp");
        assert_eq!(ctx.host.as_deref().expect("host"), "example.com:8080");
        assert_eq!(
            ctx.path.expect("set by helper"),
            "/path/to/git/",
            "values are passed verbatim even if they would otherwise look different"
        );
        Ok(())
    }

    #[test]
    fn helpers_can_set_any_context_value_using_the_url_only() -> TestResult {
        let actual =
            invoke_cascade(["url", "custom-helper"], Action::get_for_url("http://github.com"))?.expect("credentials");

        let ctx: Context = (&actual.next).try_into()?;
        assert_eq!(
            ctx.protocol.as_deref().expect("protocol"),
            "http",
            "url is processed last, it overwrites what came before"
        );
        assert_eq!(ctx.host.as_deref().expect("host"), "example.com:8080");
        assert_eq!(
            ctx.path.expect("set by helper"),
            "path/to/git",
            "the url is processed like any other"
        );
        Ok(())
    }

    #[test]
    fn helpers_can_quit_and_their_creds_are_taken_if_complete() -> TestResult {
        let actual = invoke_cascade(["last-pass", "custom-helper"], Action::get_for_url("http://github.com"))?
            .expect("credentials");

        assert_eq!(actual.identity, identity("user", "pass"));
        Ok(())
    }

    #[test]
    fn expired_credentials_are_not_returned() -> TestResult {
        let actual = invoke_cascade(
            ["expired", "oauth-token", "custom-helper"],
            Action::get_for_url("http://github.com"),
        )?
        .expect("credentials");

        assert_eq!(
            actual.identity,
            Account {
                oauth_refresh_token: Some("oauth-token".into()),
                ..identity("user-script", "pass-script")
            },
            "it ignored the expired password, which otherwise would have come first"
        );
        Ok(())
    }

    #[test]
    fn bogus_password_overrides_any_helper_and_helper_overrides_username_in_url() -> TestResult {
        let actual = Cascade::default()
            .query_user_only(true)
            .extend(fixtures(["username", "password"]))
            .invoke(
                Action::get_for_url("ssh://git@host/repo"),
                gix_prompt::Options {
                    mode: gix_prompt::Mode::Disable,
                    askpass: None,
                },
            )?
            .expect("credentials");
        assert_eq!(actual.identity, identity("user", ""));
        Ok(())
    }

    fn action_get() -> Action {
        Action::get_for_url("does/not/matter")
    }

    fn identity(user: &str, pass: &str) -> Account {
        Account {
            username: user.into(),
            password: pass.into(),
            oauth_refresh_token: None,
            authtype: None,
        }
    }

    fn invoke_cascade<'a>(
        names: impl IntoIterator<Item = &'a str>,
        action: Action,
    ) -> gix_error::Result<Option<protocol::Outcome>> {
        Cascade::default().use_http_path(true).extend(fixtures(names)).invoke(
            action,
            gix_prompt::Options {
                mode: gix_prompt::Mode::Disable,
                askpass: None,
            },
        )
    }

    fn fixtures<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<Program> {
        names.into_iter().map(crate::helper::script_helper).collect()
    }
}
