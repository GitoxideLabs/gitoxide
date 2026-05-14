mod invoke {
    use bstr::ByteSlice;
    use gix_credentials::{
        Program,
        helper::{Action, Cascade},
        protocol,
        protocol::Context,
    };
    use gix_sec::identity::Account;

    #[test]
    fn credentials_are_filled_in_one_by_one_and_stop_when_complete() {
        let actual = invoke_cascade(["username", "password", "custom-helper"], action_get())
            .unwrap()
            .expect("credentials");
        assert_eq!(actual.identity, identity("user", "pass"));
    }

    #[test]
    fn usernames_in_urls_are_kept_if_the_helper_does_not_overwrite_it() {
        let actual = invoke_cascade(
            ["password", "custom-helper"],
            Action::get_for_url("ssh://git@host.org/path"),
        )
        .unwrap()
        .expect("credentials");
        assert_eq!(actual.identity, identity("git", "pass"));
    }

    #[test]
    fn partial_credentials_can_be_overwritten_by_complete_ones() {
        let actual = invoke_cascade(["username", "custom-helper"], action_get())
            .unwrap()
            .expect("credentials");
        assert_eq!(actual.identity, identity("user-script", "pass-script"));
    }

    #[test]
    fn failing_helpers_for_filling_dont_interrupt() {
        let actual = invoke_cascade(["fail", "custom-helper"], action_get())
            .unwrap()
            .expect("credentials");
        assert_eq!(actual.identity, identity("user-script", "pass-script"));
    }

    #[test]
    fn urls_are_split_in_get_to_support_scripts() {
        let actual = invoke_cascade(
            ["reflect", "custom-helper"],
            Action::get_for_url("https://example.com:8080/path/git/"),
        )
        .unwrap()
        .expect("credentials");

        let ctx: Context = (&actual.next).try_into().unwrap();
        assert_eq!(ctx.protocol.as_deref().expect("protocol"), "https");
        assert_eq!(ctx.host.as_deref().expect("host"), "example.com:8080");
        assert_eq!(ctx.path.as_deref().expect("path").as_bstr(), "path/git");
    }

    #[test]
    fn urls_are_split_in_get_but_can_skip_the_path_in_host_only_urls() {
        let actual = invoke_cascade(["reflect", "custom-helper"], Action::get_for_url("http://example.com"))
            .unwrap()
            .expect("credentials");

        let ctx: Context = (&actual.next).try_into().unwrap();
        assert_eq!(ctx.protocol.as_deref().expect("protocol"), "http");
        assert_eq!(ctx.host.as_deref().expect("host"), "example.com");
        assert_eq!(ctx.path, None);
    }

    #[test]
    fn helpers_can_set_any_context_value() {
        let actual = invoke_cascade(
            ["all-but-credentials", "custom-helper"],
            Action::get_for_url("http://github.com"),
        )
        .unwrap()
        .expect("credentials");

        let ctx: Context = (&actual.next).try_into().unwrap();
        assert_eq!(ctx.protocol.as_deref().expect("protocol"), "ftp");
        assert_eq!(ctx.host.as_deref().expect("host"), "example.com:8080");
        assert_eq!(
            ctx.path.expect("set by helper"),
            "/path/to/git/",
            "values are passed verbatim even if they would otherwise look different"
        );
    }

    #[test]
    fn helpers_can_set_any_context_value_using_the_url_only() {
        let actual = invoke_cascade(["url", "custom-helper"], Action::get_for_url("http://github.com"))
            .unwrap()
            .expect("credentials");

        let ctx: Context = (&actual.next).try_into().unwrap();
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
    }

    #[test]
    fn helpers_can_quit_and_their_creds_are_taken_if_complete() {
        let actual = invoke_cascade(["last-pass", "custom-helper"], Action::get_for_url("http://github.com"))
            .unwrap()
            .expect("credentials");

        assert_eq!(actual.identity, identity("user", "pass"));
    }

    #[test]
    fn expired_credentials_are_not_returned() {
        let actual = invoke_cascade(
            ["expired", "oauth-token", "custom-helper"],
            Action::get_for_url("http://github.com"),
        )
        .unwrap()
        .expect("credentials");

        assert_eq!(
            actual.identity,
            Account {
                oauth_refresh_token: Some("oauth-token".into()),
                ..identity("user-script", "pass-script")
            },
            "it ignored the expired password, which otherwise would have come first"
        );
    }

    #[test]
    fn bogus_password_overrides_any_helper_and_helper_overrides_username_in_url() {
        let actual = Cascade::default()
            .query_user_only(true)
            .extend(fixtures(["username", "password"]))
            .invoke(
                Action::get_for_url("ssh://git@host/repo"),
                gix_prompt::Options {
                    mode: gix_prompt::Mode::Disable,
                    askpass: None,
                },
            )
            .unwrap()
            .expect("credentials");
        assert_eq!(actual.identity, identity("user", ""));
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

    #[allow(clippy::result_large_err)]
    fn invoke_cascade<'a>(names: impl IntoIterator<Item = &'a str>, action: Action) -> protocol::Result {
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
