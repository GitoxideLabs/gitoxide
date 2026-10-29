#![cfg(unix)]

use gix_testtools::TestResult;
use std::{os::unix::fs::PermissionsExt, sync::atomic::AtomicBool};

#[test]
fn remote_environment_selects_askpass_and_reaches_native_receive_hooks() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let directory = gix_testtools::tempfile::tempdir()?;
    let askpass = directory.path().join("askpass");
    std::fs::write(
        &askpass,
        "#!/bin/sh
printf '%s\\n' \"$GIX_TEST_REMOTE_SECRET\"
",
    )?;
    std::fs::set_permissions(&askpass, std::fs::Permissions::from_mode(0o755))?;
    let env = vec![
        ("GIT_TERMINAL_PROMPT".into(), "0".into()),
        ("GIT_ASKPASS".into(), askpass.into_os_string()),
        ("GIX_TEST_REMOTE_SECRET".into(), "operation-secret".into()),
    ];
    let remote = repo
        .remote_at("ssh://example.com/repository")?
        .with_command_environment(env.clone());
    let connection = remote.connect(gix::remote::Direction::Fetch)?;
    let mut credentials = connection.configured_credentials(gix::url::parse("https://user@example.com")?)?;
    let identity = credentials(gix::credentials::helper::Action::get_for_url(
        "https://user@example.com",
    ))?
    .expect("askpass supplies password")
    .identity;
    assert_eq!(identity.password, "operation-secret");
    let mut credentials = connection.configured_credentials_for_current_url();
    assert_eq!(
        credentials(gix::credentials::helper::Action::get_for_url(
            "https://user@example.com"
        ))?
        .expect("dynamic credentials use same environment")
        .identity
        .password,
        "operation-secret"
    );

    let destination = directory.path().join("receiver");
    std::fs::create_dir(&destination)?;
    gix_testtools::git(&destination, "init --bare")?;
    let hook = destination.join("hooks/pre-receive");
    std::fs::write(
        &hook,
        "#!/bin/sh
test \"$GIX_TEST_REMOTE_SECRET\" = operation-secret
",
    )?;
    std::fs::set_permissions(hook, std::fs::Permissions::from_mode(0o755))?;
    let outcome = repo
        .remote_at(gix::path::into_bstr(&destination)?)?
        .with_command_environment(env)
        .push(
            [gix::push::Update {
                destination: "refs/heads/topic".try_into()?,
                new_id: Some(repo.head_id()?.detach()),
                lease: gix::push::Lease::Any,
                force: false,
            }],
            gix::push::Options::default(),
            gix::progress::Discard,
            |_, _| Ok(()),
            &AtomicBool::default(),
        )?;
    assert!(matches!(outcome.updates[0].status, gix::push::Status::Updated { .. }));
    assert!(
        std::env::var_os("GIX_TEST_REMOTE_SECRET").is_none(),
        "operation environment never changes the process"
    );
    Ok(())
}
