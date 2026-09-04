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
        assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
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
        assert_eq!(
            err.to_string(),
            "The handler asked to stop trying to obtain credentials"
        );
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
