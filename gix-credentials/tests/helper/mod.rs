mod cascade;
mod context;
mod invoke;

mod invoke_outcome_to_helper_result {
    use gix_credentials::{helper, protocol, protocol::helper_outcome_to_result};

    #[test]
    fn missing_username_or_password_causes_failure_with_get_action() {
        let action = helper::Action::get_for_url("does/not/matter");
        let err = helper_outcome_to_result(
            Some(helper::Outcome {
                username: None,
                password: None,
                oauth_refresh_token: None,
                quit: false,
                next: protocol::Context::default().into(),
            }),
            action,
        )
        .unwrap_err();
        insta::assert_debug_snapshot!(err, "missing username or password causes failure with get action", @"Could not obtain identity for context: url=does/not/matter");
        assert!(err.is_not_found());
    }

    #[test]
    fn invalid_context_still_reports_missing_identity() {
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
                    err.is_not_found(),
                    "Invalid context must not replace the missing-credentials classification"
                );
            }
        }
        insta::assert_debug_snapshot!(error_snapshots, "invalid context still reports missing identity", @"
        [
            Could not obtain identity for context: ,
            Could not obtain identity for context: ,
            Could not obtain identity for context: ,
            Could not obtain identity for context: ,
            Could not obtain identity for context: ,
            Could not obtain identity for context: ,
        ]
        ");
    }

    #[test]
    fn quit_message_in_context_causes_special_error_ignoring_missing_identity() {
        let action = helper::Action::get_for_url("does/not/matter");
        let err = helper_outcome_to_result(
            Some(helper::Outcome {
                username: None,
                password: None,
                oauth_refresh_token: None,
                quit: true,
                next: protocol::Context::default().into(),
            }),
            action,
        )
        .unwrap_err();
        insta::assert_debug_snapshot!(err, "quit message in context causes special error ignoring missing identity", @"The handler asked to stop trying to obtain credentials");
    }
}

use bstr::{BString, ByteVec};
use gix_credentials::Program;
use gix_testtools::fixture_path;
use std::{borrow::Cow, path::Path};

pub fn script_helper(name: &str) -> Program {
    fn to_arg<'a>(path: impl Into<Cow<'a, Path>>) -> BString {
        let utf8_encoded = gix_path::into_bstr(path);
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
