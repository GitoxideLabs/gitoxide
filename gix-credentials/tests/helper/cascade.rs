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
                        current_dir: None,
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
                    current_dir: None,
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
                    current_dir: None,
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
                current_dir: None,
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
    fn helpers_can_quit_before_prompting_for_incomplete_credentials() -> TestResult {
        use std::{io::Write, process::Stdio};

        let definition = "!f() { cat >/dev/null; echo quit=true; }; f";
        let askpass = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/askpass.sh");
        let temporary = gix_testtools::tempfile::tempdir()?;
        let mut git = gix_testtools::git_command(temporary.path())
            .args([
                "-c",
                "credential.helper=",
                "-c",
                &format!("credential.helper={definition}"),
                "credential",
                "fill",
            ])
            .env("GIT_ASKPASS", &askpass)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        git.stdin
            .take()
            .expect("piped input")
            .write_all(b"url=https://example.com\n\n")?;
        let output = git.wait_with_output()?;
        assert!(!output.status.success(), "Git honors a helper's request to quit");
        assert!(
            output.stdout.is_empty(),
            "Git returns no prompted identity after quitting"
        );
        assert!(
            output.stderr.contains_str("told us to quit"),
            "the reference failed due to the helper's quit request"
        );

        let err = Cascade::default()
            .extend([Program::from_custom_definition(definition)?])
            .invoke(
                Action::get_for_url("https://example.com"),
                gix_prompt::Options {
                    current_dir: None,
                    mode: gix_prompt::Mode::Hidden,
                    askpass: Some(askpass),
                },
            )
            .expect_err("a helper can stop authentication before the prompt supplies credentials");
        assert!(
            err.is_cancelled(),
            "a helper's quit request remains a cancellation: {err}"
        );
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
                    current_dir: None,
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
        }
    }

    fn invoke_cascade<'a>(
        names: impl IntoIterator<Item = &'a str>,
        action: Action,
    ) -> gix_error::Result<Option<protocol::Outcome>> {
        Cascade::default().use_http_path(true).extend(fixtures(names)).invoke(
            action,
            gix_prompt::Options {
                current_dir: None,
                mode: gix_prompt::Mode::Disable,
                askpass: None,
            },
        )
    }

    fn fixtures<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<Program> {
        names.into_iter().map(crate::helper::script_helper).collect()
    }
}

#[test]
fn operation_environment_reaches_helpers_without_changing_the_process() -> gix_testtools::TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _environment = gix_testtools::Env::new().set("GIX_TEST_CREDENTIAL_VALUE", "process-value");
    let mut cascade =
        gix_credentials::helper::Cascade::default().extend([gix_credentials::Program::from_custom_definition(
            "!f() { printf 'username=user\\npassword=%s\\n' \"$GIX_TEST_CREDENTIAL_VALUE\"; }; f",
        )?]);
    let action = gix_credentials::helper::Action::get_for_url("https://example.com");
    let prompt = gix_prompt::Options {
        current_dir: None,
        mode: gix_prompt::Mode::Disable,
        ..Default::default()
    };
    let outcome = cascade
        .invoke_with_environment(
            action.clone(),
            prompt.clone(),
            &[("GIX_TEST_CREDENTIAL_VALUE".into(), "operation-value".into())],
        )?
        .expect("helper supplies identity");
    assert_eq!(outcome.identity.password, "operation-value");
    assert_eq!(std::env::var("GIX_TEST_CREDENTIAL_VALUE")?, "process-value");
    let outcome = cascade.invoke(action, prompt)?.expect("helper supplies identity");
    assert_eq!(
        outcome.identity.password, "process-value",
        "per-operation values do not persist in reused cascades"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn operation_environment_reaches_askpass() -> gix_testtools::TestResult {
    use std::os::unix::fs::PermissionsExt;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let directory = gix_testtools::tempfile::tempdir()?;
    let askpass = directory.path().join("askpass");
    std::fs::write(
        &askpass,
        "#!/bin/sh
test \"$GIX_TEST_ASKPASS_VALUE\" != fail || exit 1
printf '%s\\n' \"$GIX_TEST_ASKPASS_VALUE\"
",
    )?;
    std::fs::set_permissions(&askpass, std::fs::Permissions::from_mode(0o755))?;
    let outcome = gix_credentials::helper::Cascade::default()
        .invoke_with_environment(
            gix_credentials::helper::Action::get_for_url("https://user@example.com"),
            gix_prompt::Options {
                current_dir: None,
                askpass: Some(askpass.clone()),
                mode: gix_prompt::Mode::Disable,
            },
            &[("GIX_TEST_ASKPASS_VALUE".into(), "per-operation-secret".into())],
        )?
        .expect("askpass supplies password");
    assert_eq!(outcome.identity.password, "per-operation-secret");
    assert!(
        gix_credentials::helper::Cascade::default()
            .invoke_with_environment(
                gix_credentials::helper::Action::get_for_url("https://user@example.com"),
                gix_prompt::Options {
                    current_dir: None,
                    askpass: Some(askpass),
                    mode: gix_prompt::Mode::Disable
                },
                &[("GIX_TEST_ASKPASS_VALUE".into(), "fail".into())],
            )
            .is_err(),
        "a failing askpass cannot enable a disabled terminal prompt or supply empty credentials"
    );
    Ok(())
}

mod invoke_with_prompt {
    use gix_credentials::{
        Program,
        helper::{Action, Cascade},
        protocol::Context,
    };
    use gix_prompt::{Mode, Options};
    use gix_testtools::TestResult;

    #[test]
    fn supplies_missing_fields_with_their_visibility() -> TestResult {
        let mut prompts = Vec::new();
        let outcome = Cascade::default()
            .invoke_with_prompt(
                Action::get_for_url("https://example.com/repo"),
                Options::default(),
                |message, options| {
                    prompts.push((message.to_owned(), options.mode));
                    Ok(match options.mode {
                        Mode::Visible => "user",
                        Mode::Hidden => "password",
                        Mode::Disable => unreachable!("disabled prompts must not invoke the callback"),
                    }
                    .into())
                },
            )?
            .expect("the callback supplies a complete identity");
        assert_eq!(
            prompts,
            [
                ("Username for https://example.com: ".into(), Mode::Visible),
                ("Password for https://user@example.com: ".into(), Mode::Hidden),
            ],
            "prompts use the normalized context, including the username just obtained"
        );
        assert_eq!(
            outcome.identity.username, "user",
            "the username comes from the visible prompt"
        );
        assert_eq!(
            outcome.identity.password, "password",
            "the password comes from the hidden prompt"
        );
        let saved = Context::try_from(&outcome.next)?;
        assert_eq!(
            saved.username,
            Some("user".into()),
            "follow-up actions retain the identity"
        );
        assert_eq!(
            saved.password,
            Some("password".into()),
            "follow-up actions retain the password"
        );
        Ok(())
    }

    #[test]
    fn helpers_run_before_the_prompt() -> TestResult {
        let outcome = Cascade::default()
            .extend([Program::from_custom_definition(
                "!f() { cat >/dev/null; echo username=helper; }; f",
            )?])
            .invoke_with_prompt(
                Action::get_for_url("https://example.com/repo"),
                Options::default(),
                |message, options| {
                    assert_eq!(
                        message, "Password for https://helper@example.com: ",
                        "only the missing password is requested"
                    );
                    assert_eq!(options.mode, Mode::Hidden, "password input is secret");
                    Ok("password".into())
                },
            )?
            .expect("the helper and callback jointly supply an identity");
        assert_eq!(outcome.identity.username, "helper", "the helper's username is retained");
        Ok(())
    }

    #[test]
    fn disabled_complete_cancelled_and_followup_actions_do_not_prompt() -> TestResult {
        let never_prompt =
            |_: &str, _: &Options| -> gix_error::Result<String> { panic!("this action must not prompt") };
        let err = Cascade::default()
            .invoke_with_prompt(
                Action::get_for_url("https://example.com"),
                Options {
                    current_dir: None,
                    mode: Mode::Disable,
                    askpass: None,
                },
                never_prompt,
            )
            .expect_err("disabled prompting leaves credentials incomplete");
        assert!(
            err.is_unauthenticated(),
            "disabled prompting reports the missing identity"
        );
        let outcome = Cascade::default()
            .invoke_with_prompt(
                Action::get_for_url("https://user:password@example.com"),
                Options::default(),
                never_prompt,
            )?
            .expect("the URL contains complete credentials");
        for action in [outcome.next.clone().store(), outcome.next.erase()] {
            assert!(
                Cascade::default()
                    .invoke_with_prompt(action, Options::default(), never_prompt)?
                    .is_none(),
                "follow-up actions have no identity outcome"
            );
        }
        let err = Cascade::default()
            .extend([Program::from_custom_definition(
                "!f() { cat >/dev/null; echo quit=true; }; f",
            )?])
            .invoke_with_prompt(
                Action::get_for_url("https://example.com"),
                Options::default(),
                never_prompt,
            )
            .expect_err("the helper cancels authentication");
        assert!(err.is_cancelled(), "the helper's cancellation remains classified");
        Ok(())
    }

    #[test]
    fn callback_errors_and_invalid_responses_do_not_produce_an_identity() {
        let err = Cascade::default()
            .invoke_with_prompt(
                Action::get_for_url("https://example.com"),
                Options::default(),
                |_, _| Err(gix_error::message("user cancelled the prompt").cancelled_error()),
            )
            .expect_err("a cancelled prompt aborts authentication");
        assert!(err.is_cancelled(), "the callback error keeps its classification");
        let err = Cascade::default()
            .invoke_with_prompt(
                Action::get_for_url("https://user@example.com"),
                Options::default(),
                |_, _| Ok("secret\npassword".into()),
            )
            .expect_err("responses cannot inject fields into the credential protocol");
        assert!(
            err.is_validation(),
            "invalid prompt responses are rejected before constructing follow-up actions"
        );
        assert!(
            !format!("{err:?}").contains("secret"),
            "diagnostics do not expose the rejected password"
        );
    }
}

#[test]
fn operation_environment_combines_with_custom_prompt() -> gix_testtools::TestResult {
    use gix_credentials::{
        Program,
        helper::{Action, Cascade},
    };
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _environment = gix_testtools::Env::new().set("GIX_TEST_CREDENTIAL_USER", "process-user");
    let mut cascade = Cascade::default().extend([Program::from_custom_definition(
        "!f() { printf 'username=%s\\n' \"$GIX_TEST_CREDENTIAL_USER\"; }; f",
    )?]);
    let outcome = cascade
        .invoke_with_environment_and_prompt(
            Action::get_for_url("https://example.com"),
            gix_prompt::Options::default(),
            &[("GIX_TEST_CREDENTIAL_USER".into(), "operation-user".into())],
            |message, options| {
                assert_eq!(
                    message, "Password for https://operation-user@example.com: ",
                    "the helper uses operation environment before prompting"
                );
                assert_eq!(
                    options.mode,
                    gix_prompt::Mode::Hidden,
                    "only the missing password is requested"
                );
                Ok("secret".into())
            },
        )?
        .expect("helper and callback supply the identity");
    assert_eq!(
        outcome.identity.username, "operation-user",
        "native helper output reaches the result"
    );
    assert_eq!(
        std::env::var("GIX_TEST_CREDENTIAL_USER")?,
        "process-user",
        "operation environment stays local"
    );
    let error = Cascade::default()
        .invoke_with_environment_and_prompt(
            Action::get_for_url("https://example.com"),
            gix_prompt::Options {
                current_dir: None,
                mode: gix_prompt::Mode::Disable,
                askpass: Some("unused-askpass".into()),
            },
            &[],
            |_, _| panic!("a configured askpass does not enable a disabled custom prompt"),
        )
        .expect_err("disabled custom prompting leaves the identity incomplete");
    assert!(
        error.is_unauthenticated(),
        "the missing identity retains its classification"
    );
    Ok(())
}
