mod from_custom_definition;
mod main;

#[test]
#[cfg(unix)]
fn named_helpers_run_without_git_and_preserve_operation_environment() -> gix_testtools::TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_named_helpers.sh")?.canonicalize()?;
    let core = fixture.join("installation with spaces/libexec/git-core");
    for environment in [
        vec![("PATH".into(), fixture.join("standalone").into_os_string())],
        vec![(
            "PATH".into(),
            fixture.join("installation with spaces/bin").into_os_string(),
        )],
        vec![
            ("PATH".into(), "".into()),
            ("GIT_EXEC_PATH".into(), core.into_os_string()),
        ],
        vec![("PATH".into(), "standalone".into())],
        vec![
            ("PATH".into(), "".into()),
            (
                "GIT_EXEC_PATH".into(),
                "installation with spaces/libexec/git-core".into(),
            ),
        ],
    ] {
        for definition in [r#"sample --label="a b""#, r#"sample --label="$GIX_HELPER_ARGUMENT""#] {
            let mut environment = environment.clone();
            environment.extend([
                ("GIX_HELPER_USER".into(), "operation-user".into()),
                ("GIX_HELPER_ARGUMENT".into(), "a b".into()),
            ]);
            let mut helper = gix_credentials::Program::from_custom_definition(definition)?;
            helper.current_dir = Some(fixture.clone());
            let output = helper
                .to_command_with_environment(&gix_credentials::helper::Action::Get(Default::default()), &environment)?
                .output()?;
            assert!(output.status.success());
            assert_eq!(output.stdout, b"username=operation-user\npassword=--label=a b:get\n");
        }
    }
    Ok(())
}
