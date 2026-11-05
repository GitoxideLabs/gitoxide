use bstr::{BString, ByteSlice};
use gix_credentials::{
    helper,
    protocol::{Capability, Context, ContextOptions},
};
use gix_sec::identity::Credentials;

use crate::helper::script_helper;

#[test]
fn follow_up_actions_discard_token_fields_rejected_in_wire_order() -> gix_testtools::TestResult {
    for caller in [false, true] {
        for accepted in [false, true] {
            for line_ending in ["\n", "\r\n"] {
                let response = format!(
                    "authtype=Rejected
credential=rejected-before-capability
ephemeral=1
capability[]=authtype
{}username=user
password=pass
quit=0
unknown=preserved

capability[]=authtype
authtype=Rejected
credential=rejected-after-record
ephemeral=1
",
                    if accepted {
                        "authtype=Bearer
credential=accepted-token
ephemeral=0
"
                    } else {
                        ""
                    }
                )
                .replace('\n', line_ending);
                let mut outcome = helper::invoke(
                    &mut gix_credentials::Program::from_custom_definition(format!(
                        "!f() {{ cat >/dev/null; printf '%s' '{response}'; }}; f"
                    ))?,
                    &helper::Action::Get(Context {
                        capability: if caller { vec![Capability::Authtype] } else { Vec::new() },
                        ..Default::default()
                    }),
                )?
                .expect("the helper supplies at least username and password");
                let credentials = outcome.consume_credentials().expect("complete credentials");
                assert_eq!(
                    credentials,
                    if caller && accepted {
                        Credentials::Encoded {
                            authtype: "Bearer".into(),
                            credential: "accepted-token".into(),
                            oauth_refresh_token: None,
                        }
                    } else {
                        Credentials::UsernamePassword {
                            username: "user".into(),
                            password: "pass".into(),
                            oauth_refresh_token: None,
                        }
                    },
                    "only fields accepted before the record terminator determine the credentials"
                );
                for action in [outcome.next.clone().store(), outcome.next.erase()] {
                    let payload = action.payload().expect("follow-up actions have a payload");
                    assert!(
                        !payload.contains_str("rejected-"),
                        "discarded secrets must not be forwarded, even if a later token was accepted"
                    );
                    assert!(
                        payload.contains_str("accepted-token") == (caller && accepted),
                        "only negotiated token fields survive"
                    );
                    assert!(
                        payload.contains_str("unknown=preserved") && payload.contains_str("quit=0"),
                        "unrelated legacy fields remain verbatim"
                    );
                    let context = Context::from_bytes(payload, Default::default())?;
                    assert_eq!(
                        context.credential.as_deref(),
                        (caller && accepted).then_some("accepted-token"),
                        "stored credentials match the accepted response"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn authtype_requires_caller_and_helper_negotiation_and_overrides_passwords() -> gix_testtools::TestResult {
    for caller in [false, true] {
        for responder in [false, true] {
            let response = format!(
                "{}authtype=Bearer
credential=access-token
ephemeral=1
username=user
password=pass
",
                if responder { "capability[]=authtype\n" } else { "" }
            );
            let mut outcome = helper::invoke(
                &mut gix_credentials::Program::from_custom_definition(format!(
                    "!f() {{ cat >/dev/null; printf '%s' '{response}'; }}; f"
                ))?,
                &helper::Action::Get(Context {
                    capability: if caller { vec![Capability::Authtype] } else { Vec::new() },
                    url: Some("https://example.com".into()),
                    ..Default::default()
                }),
            )?
            .expect("the helper always supplies a password");
            let credentials = outcome
                .consume_credentials()
                .expect("complete credentials are available");
            let negotiated = caller && responder;
            assert_eq!(
                credentials,
                if negotiated {
                    Credentials::Encoded {
                        authtype: "Bearer".into(),
                        credential: "access-token".into(),
                        oauth_refresh_token: None,
                    }
                } else {
                    Credentials::UsernamePassword {
                        username: "user".into(),
                        password: "pass".into(),
                        oauth_refresh_token: None,
                    }
                },
                "a complete token takes precedence over the password only when both parties announce support"
            );
            for action in [outcome.next.clone().store(), outcome.next.erase()] {
                let payload = action.payload().expect("follow-up payload");
                if !negotiated {
                    assert!(
                        !payload
                            .windows(b"credential=".len())
                            .any(|bytes| bytes == b"credential="),
                        "unnegotiated token fields must not be forwarded to helpers"
                    );
                }
                let context = Context::from_bytes(payload, Default::default())?;
                assert_eq!(
                    context.credential.as_deref(),
                    negotiated.then_some("access-token"),
                    "follow-up actions use only negotiated credentials"
                );
                assert_eq!(
                    context.ephemeral,
                    negotiated.then_some(true),
                    "the ephemeral hint accompanies the credential"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn follow_up_actions_discard_unsupported_capabilities_and_state() -> gix_testtools::TestResult {
    for capability in [
        Vec::new(),
        vec![
            Capability::Unsupported("state".into()),
            Capability::Unsupported("future".into()),
        ],
        vec![Capability::Authtype],
        vec![
            Capability::Unsupported("future".into()),
            Capability::Authtype,
            Capability::Unsupported("state".into()),
        ],
    ] {
        for announcements in [
            "",
            "capability[]=state
capability[]=future
capability[]=authtype-extra
capability[]=
",
            "capability[]=state
capability[]=future
capability[]=authtype
",
            "capability[]=authtype
capability[]=state
capability[]=future
",
            "capability[]=state
capability[]=authtype
capability[]=future
",
        ] {
            let negotiated =
                capability.contains(&Capability::Authtype) && announcements.contains("capability[]=authtype\n");
            for (line_ending, escaped_line_ending) in [("\n", r"\n"), ("\r\n", r"\r\n")] {
                let response = format!(
                    "state[]=discard-before-announcement
continue=1
{announcements}authtype=Bearer
credential=accepted-token
ephemeral=0
state[]=discard-after-announcement
continue=0
username=user
password=pass
oauth_refresh_token=refresh-token
quit=0
unknown=preserved

capability[]=authtype
credential=rejected-after-record
state[]=discard-after-record
unknown=discard-after-record
"
                )
                .replace('\n', escaped_line_ending);
                // Git for Windows' shell strips literal carriage returns from scripts.
                let outcome = helper::invoke(
                    &mut gix_credentials::Program::from_custom_definition(format!(
                        "!f() {{ cat >/dev/null; printf '%b' '{response}'; }}; f"
                    ))?,
                    &helper::Action::Get(Context {
                        capability: capability.clone(),
                        ..Default::default()
                    }),
                )?
                .expect("the helper supplies username and password even without authtype negotiation");
                assert_eq!(
                    outcome.credential.as_deref(),
                    negotiated.then_some("accepted-token"),
                    "unsupported announcements must neither enable nor disable authtype negotiation"
                );
                let expected = format!(
                    "{}username=user
password=pass
oauth_refresh_token=refresh-token
quit=0
unknown=preserved
",
                    if negotiated {
                        "capability[]=authtype
authtype=Bearer
credential=accepted-token
ephemeral=0
"
                    } else {
                        ""
                    }
                )
                .replace('\n', line_ending);
                for action in [outcome.next.clone().store(), outcome.next.erase()] {
                    let payload = action.payload().expect("follow-up actions have a payload");
                    assert_eq!(
                        payload,
                        expected.as_bytes().as_bstr(),
                        "only negotiated capabilities and unrelated legacy fields survive verbatim, up to the record terminator"
                    );
                    let context = Context::from_bytes(payload, Default::default())?;
                    assert_eq!(
                        context.capability,
                        if negotiated {
                            vec![Capability::Authtype]
                        } else {
                            Vec::new()
                        },
                        "follow-up actions must never announce unsupported capabilities"
                    );
                    assert_eq!(
                        context.credential.as_deref(),
                        outcome.credential.as_deref(),
                        "follow-up actions retain the accepted credential"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn get() -> gix_testtools::TestResult {
    let mut outcome = gix_credentials::helper::invoke(
        &mut script_helper("last-pass"),
        &helper::Action::get_for_url("https://github.com/byron/gitoxide"),
    )?
    .expect("mock provides credentials");
    assert_eq!(
        outcome.consume_credentials().expect("complete"),
        Credentials::UsernamePassword {
            username: "user".into(),
            password: "pass".into(),
            oauth_refresh_token: None
        }
    );
    assert_eq!(
        outcome.next.store().payload().expect("store action has a payload"),
        "username=user\npassword=pass\nquit=1\n"
    );
    Ok(())
}

#[test]
fn get_uses_context_options_for_the_entire_exchange() -> gix_testtools::TestResult {
    let action = helper::Action::Get(Context::from_url(
        "https://github.com/byron/gitoxide",
        ContextOptions {
            protect_protocol: false,
        },
    ));
    let outcome = gix_credentials::helper::invoke(&mut script_helper("carriage-return"), &action)?
        .expect("mock provides credentials");

    assert_eq!(outcome.username.as_deref(), Some("user\rname"));
    let context: Context = (&outcome.next).try_into()?;
    assert_eq!(context.options, action.context().expect("get action").options);
    assert_eq!(context.username.as_deref(), Some("user\rname"));
    Ok(())
}

#[test]
fn store_and_reject() {
    let ctx = Context {
        url: Some("https://github.com/byron/gitoxide".into()),
        ..Default::default()
    };
    let ctxbuf = || -> BString {
        let mut buf = Vec::<u8>::new();
        ctx.write_to(&mut buf).expect("cannot fail");
        buf.into()
    };
    for action in [helper::Action::Store(ctxbuf()), helper::Action::Erase(ctxbuf())] {
        let outcome = gix_credentials::helper::invoke(&mut script_helper("last-pass"), &action).unwrap();
        assert!(
            outcome.is_none(),
            "store and erase have no outcome, they just shouldn't fail"
        );
    }
}

mod program {
    use crate::TestResult;
    use gix_credentials::{Program, helper, program::Kind};

    use crate::helper::script_helper;

    #[test]
    fn builtin_negotiates_tokens_with_a_url_and_forwards_them() -> TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        if gix_testtools::should_skip_as_git_version_is_smaller_than(2, 46, 0) {
            return Ok(());
        }
        let temp = gix_testtools::tempfile::tempdir()?;
        let _cwd = gix_testtools::set_current_dir(temp.path())?;
        let _env = gix_testtools::Env::new()
            .set("GIT_CONFIG_COUNT", "1")
            .set("GIT_CONFIG_KEY_0", "credential.helper")
            .set(
                "GIT_CONFIG_VALUE_0",
                r#"!f() {
                    capability= scheme= token=
                    while IFS= read -r line; do
                        case "$line" in
                            'capability[]=authtype') capability=yes ;;
                            authtype=Bearer) scheme=yes ;;
                            credential=access-token) token=yes ;;
                        esac
                    done
                    if test "$1" = get; then
                        if test "$capability" = yes; then
                            printf 'capability[]=authtype\nauthtype=Bearer\ncredential=access-token\nephemeral=1\n'
                        else
                            echo quit=1
                        fi
                    elif test "$capability$scheme$token" = yesyesyes; then
                        printf '%s\n' "$1" >> callbacks
                    fi
                }; f"#,
            );
        let mut context = gix_credentials::protocol::Context::from_url("https://example.com/repo", Default::default());
        context.capability.push(gix_credentials::protocol::Capability::Authtype);
        let outcome = gix_credentials::builtin(helper::Action::Get(context))?
            .expect("Git forwards the capability to its configured helper");
        assert_eq!(
            outcome.credentials,
            gix_sec::identity::Credentials::Encoded {
                authtype: "Bearer".into(),
                credential: "access-token".into(),
                oauth_refresh_token: None,
            },
            "parsing the URL must not clear the capability needed for token lookup"
        );
        let mut context = gix_credentials::protocol::Context::try_from(&outcome.next)?;
        context.url = Some("https://example.com/repo".into());
        let next: helper::NextAction = context.into();
        for action in [next.clone().store(), next.erase()] {
            gix_credentials::builtin(action)?;
        }
        assert_eq!(
            std::fs::read_to_string(temp.path().join("callbacks"))?,
            "store\nerase\n",
            "restoring the URL must preserve token fields in Git-backed follow-up callbacks"
        );
        Ok(())
    }

    #[test]
    fn builtin() -> TestResult {
        // Other tests resolve fixture paths relative to the working directory, so change it only in a child.
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let temp = gix_testtools::tempfile::tempdir()?;
        let _cwd = gix_testtools::set_current_dir(temp.path())?;
        let err = gix_credentials::helper::invoke(
            &mut Program::from_kind(Kind::Builtin).suppress_stderr(),
            &helper::Action::get_for_url("/path/without/scheme/fails/with/error"),
        )
        .expect_err("the builtin helper rejects a URL without a scheme");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[("exit code:", "exit status:")]), "this failure indicates we could launch the helper, even though it wasn't happy which is fine. It doesn't like the URL", @r#"
        Credentials helper failed

        Caused by:
            0: I/O error (Other)
            1: Credentials helper program failed, exit_code=128, exit_status="exit status: 128"
        "#);
        assert!(
            err.classify().next().is_none(),
            "an unknown helper exit status must not promise that retrying will help"
        );
        Ok(())
    }

    #[test]
    fn script() -> gix_testtools::TestResult {
        assert_eq!(
            gix_credentials::helper::invoke(
                &mut Program::from_custom_definition(
                    "!f() { test \"$1\" = get && echo \"password=pass\" && echo \"username=user\"; }; f"
                )?,
                &helper::Action::get_for_url("/does/not/matter"),
            )?
            .expect("present")
            .consume_credentials()
            .expect("complete"),
            gix_sec::identity::Credentials::UsernamePassword {
                username: "user".into(),
                password: "pass".into(),
                oauth_refresh_token: None
            }
        );
        Ok(())
    }

    #[cfg(unix)] // needs executable bits to work
    #[test]
    fn path_to_helper_script() -> gix_testtools::TestResult {
        assert_eq!(
            gix_credentials::helper::invoke(
                &mut Program::from_custom_definition(
                    gix_path::into_bstr(gix_path::realpath(gix_testtools::fixture_path("custom-helper.sh"))?)?
                        .into_owned(),
                )?,
                &helper::Action::get_for_url("/does/not/matter"),
            )?
            .expect("present")
            .consume_credentials()
            .expect("complete"),
            gix_sec::identity::Credentials::UsernamePassword {
                username: "user-script".into(),
                password: "pass-script".into(),
                oauth_refresh_token: None
            }
        );
        Ok(())
    }

    #[test]
    fn path_to_helper_as_script_to_workaround_executable_bits() -> TestResult {
        assert_eq!(
            gix_credentials::helper::invoke(
                &mut script_helper("custom-helper"),
                &helper::Action::Get(gix_credentials::protocol::Context {
                    url: Some("https://example.com/repo".into()),
                    // Exceed the pipe buffer before the capability write to detect unread input reliably.
                    path: Some(vec![b'x'; 1024 * 1024].into()),
                    capability: vec![gix_credentials::protocol::Capability::Authtype],
                    ..Default::default()
                }),
            )?
            .expect("present")
            .consume_credentials()
            .expect("complete"),
            gix_sec::identity::Credentials::UsernamePassword {
                username: "user-script".into(),
                password: "pass-script".into(),
                oauth_refresh_token: None
            },
            "the fixture consumes the full request before returning credentials"
        );
        Ok(())
    }
}
