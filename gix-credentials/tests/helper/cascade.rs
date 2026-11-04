mod invoke {
    use crate::TestResult;
    use bstr::ByteSlice;
    use gix_credentials::{
        Program,
        helper::{Action, Cascade},
        protocol,
        protocol::{Capability, Context},
    };
    use gix_sec::identity::Credentials;

    #[test]
    fn negotiated_authtype_supplies_credentials_without_a_username() -> TestResult {
        let context = Context::from_bytes(
            b"capability[]=state
state[]=caller:opaque-state
continue=1
capability[]=authtype
capability[]=future
protocol=https
host=example.com
wwwauth[]=Bearer
",
            Default::default(),
        )?;
        let outcome = Cascade::default()
            .extend([
                Program::from_custom_definition(
                    r#"!f() {
                        while IFS= read -r line; do
                            case "$line" in
                                'capability[]='*) test "$line" = 'capability[]=authtype' || exit 1 ;;
                                'state[]='*|continue=*) exit 1 ;;
                            esac
                            if test "$line" = 'capability[]=authtype'; then
                                printf 'capability[]=state\nstate[]=helper:opaque-state\ncontinue=1\ncapability[]=authtype\nauthtype=Bearer\ncredential=access-token\nephemeral=1\ncapability[]=future\n'
                            fi
                        done
                    }; f"#,
                )?,
                crate::helper::script_helper("custom-helper"),
            ])
            .invoke(
                Action::Get(context),
                gix_prompt::Options {
                    mode: gix_prompt::Mode::Hidden,
                    askpass: Some(gix_testtools::fixture_path("username.sh")),
                },
            )?
            .expect("a negotiated bearer credential provides complete credentials");
        assert_eq!(
            outcome.credentials,
            Credentials::Encoded {
                authtype: "Bearer".into(),
                credential: "access-token".into(),
                oauth_refresh_token: None,
            },
            "the helper selects the scheme and supplies a pre-encoded credential without a username"
        );
        let context = Context::try_from(&outcome.next)?;
        assert_eq!(
            context.capability,
            [Capability::Authtype],
            "cascade follow-up actions advertise only supported capabilities"
        );
        assert!(
            context.www_authenticate.is_empty(),
            "challenges are not returned for storage"
        );
        for action in [outcome.next.clone().store(), outcome.next.erase()] {
            let payload = action.payload().expect("store and erase have a payload");
            assert!(
                payload.starts_with_str("url=https://example.com\ncapability[]=authtype\n"),
                "the capability follows the URL and precedes its fields so Git retains it"
            );
            assert!(
                !payload.contains_str("state") && !payload.contains_str("continue=") && !payload.contains_str("future"),
                "unsupported capabilities and state fields are omitted from store and erase"
            );
            assert!(
                payload.contains_str(
                    "authtype=Bearer
credential=access-token
ephemeral=1
"
                ),
                "helpers receive ephemeral credentials for both follow-up actions"
            );
        }
        Ok(())
    }

    #[test]
    fn incomplete_unnegotiated_or_expired_authtype_allows_password_fallback() -> TestResult {
        for (announce, response) in [
            (
                false,
                "capability[]=authtype
authtype=Bearer
credential=ignored
",
            ),
            (
                true,
                "authtype=Bearer
credential=ignored
",
            ),
            (
                true,
                "capability[]=authtype
authtype=Bearer
",
            ),
            (
                true,
                "capability[]=authtype
credential=ignored
",
            ),
            (
                true,
                "capability[]=authtype
authtype=Bearer
credential=expired
ephemeral=1
password_expiry_utc=1
",
            ),
        ] {
            let outcome = Cascade::default()
                .extend([
                    Program::from_custom_definition(format!(
                        "!f() {{ cat >/dev/null; printf '%s' '{response}'; }}; f"
                    ))?,
                    crate::helper::script_helper("custom-helper"),
                ])
                .invoke(
                    Action::Get(Context {
                        url: Some("https://example.com/repo".into()),
                        capability: if announce {
                            vec![Capability::Authtype]
                        } else {
                            Vec::new()
                        },
                        ..Default::default()
                    }),
                    gix_prompt::Options {
                        mode: gix_prompt::Mode::Disable,
                        askpass: None,
                    },
                )?
                .expect("the password helper remains available");
            assert_eq!(
                outcome.credentials,
                credentials("user-script", "pass-script"),
                "only a complete negotiated credential can replace a password: {response}"
            );
            if !announce || response.contains("password_expiry_utc") {
                let next = Context::try_from(&outcome.next)?;
                assert_eq!(
                    next.credential, None,
                    "unnegotiated and expired credentials must not be stored"
                );
                assert_eq!(
                    next.ephemeral, None,
                    "the fallback password has no inherited lifetime flag"
                );
            }
        }
        Ok(())
    }

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
                outcome.credentials,
                credentials(user, "evil.example"),
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
            outcome.credentials,
            credentials("user", "pass"),
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
            outcome.credentials,
            credentials("user", "pass"),
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
    fn credentials_are_filled_in_one_by_one_and_stop_when_complete() -> TestResult {
        let actual = invoke_cascade(["username", "password", "custom-helper"], action_get())?.expect("credentials");
        assert_eq!(actual.credentials, credentials("user", "pass"));
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

        assert_eq!(actual.credentials, credentials("user\rname", "pass"));
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
        assert_eq!(actual.credentials, credentials("git", "pass"));
        Ok(())
    }

    #[test]
    fn partial_credentials_can_be_overwritten_by_complete_ones() -> TestResult {
        let actual = invoke_cascade(["username", "custom-helper"], action_get())?.expect("credentials");
        assert_eq!(actual.credentials, credentials("user-script", "pass-script"));
        Ok(())
    }

    #[test]
    fn failing_helpers_for_filling_dont_interrupt() -> TestResult {
        let actual = invoke_cascade(["fail", "custom-helper"], action_get())?.expect("credentials");
        assert_eq!(actual.credentials, credentials("user-script", "pass-script"));
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

        assert_eq!(actual.credentials, credentials("user", "pass"));
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
            actual.credentials,
            Credentials::UsernamePassword {
                username: "user-script".into(),
                password: "pass-script".into(),
                oauth_refresh_token: Some("oauth-token".into()),
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
        assert_eq!(actual.credentials, credentials("user", ""));
        Ok(())
    }

    #[test]
    fn username_only_queries_ignore_tokens_and_reach_the_username_helper() -> TestResult {
        for prefilled in [false, true] {
            let actual = Cascade::default()
                .query_user_only(true)
                .extend([
                    Program::from_custom_definition(
                        r#"!f() {
                            while IFS= read -r line; do
                                case "$line" in
                                    'capability[]=authtype'|authtype=*|credential=*|ephemeral=*) echo quit=1 ;;
                                esac
                            done
                            printf 'capability[]=authtype\nauthtype=Bearer\ncredential=helper-token\n'
                        }; f"#,
                    )?,
                    crate::helper::script_helper("username"),
                ])
                .invoke(
                    Action::Get(Context {
                        url: Some("https://example.com/repo".into()),
                        capability: vec![Capability::Authtype],
                        authtype: prefilled.then(|| "Bearer".into()),
                        credential: prefilled.then(|| "input-token".into()),
                        ephemeral: prefilled.then_some(true),
                        ..Default::default()
                    }),
                    gix_prompt::Options {
                        mode: gix_prompt::Mode::Disable,
                        askpass: None,
                    },
                )?
                .expect("tokens cannot complete a query for a username");
            assert_eq!(
                actual.credentials,
                credentials("user", ""),
                "the username helper supplies the user and the bogus password is retained"
            );
            for action in [actual.next.clone().store(), actual.next.erase()] {
                let payload = action.payload().expect("follow-up actions have a payload");
                assert!(
                    !payload.contains_str("authtype")
                        && !payload.contains_str("credential=")
                        && !payload.contains_str("ephemeral="),
                    "username-only queries neither negotiate nor forward token credentials"
                );
            }
        }
        Ok(())
    }

    fn action_get() -> Action {
        Action::get_for_url("does/not/matter")
    }

    fn credentials(user: &str, pass: &str) -> Credentials {
        Credentials::UsernamePassword {
            username: user.into(),
            password: pass.into(),
            oauth_refresh_token: None,
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
