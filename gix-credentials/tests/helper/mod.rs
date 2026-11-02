mod cascade;
mod context;
mod invoke;

#[test]
fn consume_credentials_prefers_encoded_credentials_and_preserves_username_password() {
    use gix_credentials::{helper::Outcome, protocol::Context};
    use gix_sec::identity::Credentials;

    let mut outcome = Outcome {
        username: Some("user".into()),
        password: Some("pass".into()),
        authtype: Some("Bearer".into()),
        credential: Some("access-token".into()),
        oauth_refresh_token: Some("refresh-token".into()),
        quit: false,
        next: Context::default().into(),
    };
    assert_eq!(
        outcome.consume_credentials(),
        Some(Credentials::Encoded {
            authtype: "Bearer".into(),
            credential: "access-token".into(),
            oauth_refresh_token: Some("refresh-token".into()),
        }),
        "a complete encoded credential takes precedence and carries the refresh token"
    );
    assert_eq!(
        outcome.consume_credentials(),
        Some(Credentials::UsernamePassword {
            username: "user".into(),
            password: "pass".into(),
            oauth_refresh_token: None,
        }),
        "the unused username/password pair remains intact, but the refresh token is consumed only once"
    );
    assert_eq!(outcome.consume_credentials(), None, "both complete pairs were consumed");
}

#[test]
fn consume_credentials_preserves_incomplete_pairs_and_falls_back_to_username_password() {
    use gix_credentials::{helper::Outcome, protocol::Context};
    use gix_sec::identity::Credentials;

    for (authtype, credential) in [(None, None), (Some("Bearer"), None), (None, Some("access-token"))] {
        for (username, password) in [(None, None), (Some("user"), None), (None, Some("pass"))] {
            let mut outcome = Outcome {
                username: username.map(String::from),
                password: password.map(String::from),
                authtype: authtype.map(String::from),
                credential: credential.map(String::from),
                oauth_refresh_token: Some("refresh-token".into()),
                quit: false,
                next: Context::default().into(),
            };
            let original = outcome.clone();
            assert_eq!(
                outcome.consume_credentials(),
                None,
                "neither authentication pair is complete"
            );
            assert_eq!(outcome, original, "an unsuccessful conversion consumes no fields");

            outcome.username = Some("user".into());
            outcome.password = Some("pass".into());
            assert_eq!(
                outcome.consume_credentials(),
                Some(Credentials::UsernamePassword {
                    username: "user".into(),
                    password: "pass".into(),
                    oauth_refresh_token: Some("refresh-token".into()),
                }),
                "an incomplete encoded credential does not prevent username/password authentication"
            );
            assert_eq!(
                outcome,
                Outcome {
                    username: None,
                    password: None,
                    oauth_refresh_token: None,
                    ..original
                },
                "only the selected credential pair and refresh token are consumed"
            );
        }
    }
}

mod invoke_outcome_to_helper_result {
    use gix_credentials::{helper, protocol, protocol::helper_outcome_to_result};

    #[test]
    fn missing_username_or_password_causes_failure_with_get_action() {
        let action = helper::Action::get_for_url("does/not/matter");
        let err = helper_outcome_to_result(
            Some(helper::Outcome {
                username: None,
                password: None,
                authtype: None,
                credential: None,
                oauth_refresh_token: None,
                quit: false,
                next: protocol::Context::default().into(),
            }),
            action,
        )
        .unwrap_err();
        insta::assert_debug_snapshot!(err, "missing username or password causes failure with get action", @"Could not obtain credentials for context: url=does/not/matter");
        assert!(
            err.is_unauthenticated(),
            "missing credentials require obtaining credentials"
        );
    }

    #[test]
    fn invalid_context_still_reports_missing_credentials() {
        let mut error_snapshots = Vec::new();
        for value in ["invalid\nvalue", "invalid\0value", "invalid\rvalue"] {
            for context in [
                protocol::Context::from_url(value, Default::default()),
                protocol::Context {
                    path: Some(value.into()),
                    ..Default::default()
                },
            ] {
                let err = helper_outcome_to_result(None, helper::Action::Get(context))
                    .expect_err("Missing credentials must return an error even when the context is invalid");
                error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
                assert!(
                    err.is_unauthenticated(),
                    "Invalid context must not replace the missing-credentials classification"
                );
            }
        }
        insta::assert_debug_snapshot!(error_snapshots, "invalid context still reports missing credentials", @"
        [
            Could not obtain credentials for context: ,
            Could not obtain credentials for context: ,
            Could not obtain credentials for context: ,
            Could not obtain credentials for context: ,
            Could not obtain credentials for context: ,
            Could not obtain credentials for context: ,
        ]
        ");
    }

    #[test]
    fn quit_message_in_context_causes_special_error_ignoring_missing_credentials() {
        let action = helper::Action::get_for_url("does/not/matter");
        let err = helper_outcome_to_result(
            Some(helper::Outcome {
                username: None,
                password: None,
                authtype: None,
                credential: None,
                oauth_refresh_token: None,
                quit: true,
                next: protocol::Context::default().into(),
            }),
            action,
        )
        .unwrap_err();
        insta::assert_debug_snapshot!(err, "quit message in context causes special error ignoring missing credentials", @"The handler asked to stop trying to obtain credentials");
        assert!(err.is_cancelled(), "helper quit requests stop the credential cascade");
        assert!(!err.is_retryable(), "cancellation must not invite a retry");
        assert!(!err.can_retry(), "cancellation vetoes conservative retries");
        assert!(!err.can_retry_lenient(), "cancellation vetoes lenient retries");
        assert!(
            !err.is_unauthenticated(),
            "quit takes precedence over missing credentials"
        );
        use gix_error::ErrorExt;
        let err = err.and_raise(gix_error::retryable("outer context would otherwise permit retrying"));
        assert!(!err.can_retry(), "nested cancellation vetoes an outer retry marker");
        assert!(
            !err.can_retry_lenient(),
            "nested cancellation vetoes an outer lenient retry marker"
        );
    }
}

use bstr::{BString, ByteVec};
use gix_credentials::Program;
use gix_testtools::fixture_path;
use std::{borrow::Cow, path::Path};

pub fn script_helper(name: &str) -> Program {
    fn to_arg<'a>(path: impl Into<Cow<'a, Path>>) -> BString {
        let utf8_encoded = gix_path::into_bstr(path).expect("fixture path is representable");
        let slash_separated = gix_path::to_unix_separators_on_windows(utf8_encoded);
        gix_quote::single(slash_separated.as_ref())
    }

    let shell = gix_path::env::shell();
    let fixture = gix_path::realpath(fixture_path(format!("{name}.sh"))).unwrap();

    let mut script = to_arg(Path::new(shell));
    script.push_char(' ');
    script.push_str(to_arg(fixture));
    Program::from_kind(gix_credentials::program::Kind::ExternalShellScript(script))
}
