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

#[test]
fn configured_service_commands_run_for_file_and_ssh_remotes() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_read_only("make_native_local_fetch_repo.sh")?.canonicalize()?;
    for scheme in ["file", "ssh"] {
        let tmp = gix_testtools::tempfile::tempdir()?;
        gix_testtools::git(tmp.path(), "init --bare client")?;
        gix_testtools::git(tmp.path(), "init --bare receiver")?;
        let service = tmp.path().join("service helper");
        std::fs::write(
            &service,
            "#!/bin/sh\nservice=$1\nshift\nprintf '%s:%s\\n' \"$service\" \"$GIX_SERVICE_TEST_VALUE\" >>\"$GIX_SERVICE_TEST_MARKER\"\nexec git \"$service\" \"$@\"\n",
        )?;
        std::fs::set_permissions(&service, std::fs::Permissions::from_mode(0o755))?;
        let ssh = tmp.path().join("ssh-helper");
        std::fs::write(&ssh, "#!/bin/sh\nshift\nexec /bin/sh -c \"$*\"\n")?;
        std::fs::set_permissions(&ssh, std::fs::Permissions::from_mode(0o755))?;
        let url = |path: std::path::PathBuf| {
            if scheme == "ssh" {
                format!("ssh://host{}", path.display())
            } else {
                path.display().to_string()
            }
        };
        let repo = gix::open_opts(
            tmp.path().join("client"),
            gix::open::Options::isolated().config_overrides([
                format!("remote.origin.url={}", url(fixture.join("source"))),
                format!("remote.origin.pushurl={}", url(tmp.path().join("receiver"))),
                "remote.origin.fetch=refs/heads/main:refs/remotes/origin/main".into(),
                format!("remote.origin.uploadpack='{}' upload-pack", service.display()),
                format!("remote.origin.receivepack='{}' receive-pack", service.display()),
                format!("core.sshCommand={}", ssh.display()),
                "ssh.variant=simple".into(),
            ]),
        )?;
        let marker = tmp.path().join("commands");
        let remote = repo.find_remote("origin")?.with_command_environment([
            ("GIX_SERVICE_TEST_MARKER".into(), marker.clone().into_os_string()),
            ("GIX_SERVICE_TEST_VALUE".into(), "operation-value".into()),
        ]);
        remote.fetch(Default::default(), gix::progress::Discard, &AtomicBool::new(false))?;
        assert_eq!(
            std::fs::read_to_string(&marker)?,
            "upload-pack:operation-value\n",
            "{scheme} fetch honors the configured command and operation environment"
        );
        let head_id = repo.find_reference("refs/remotes/origin/main")?.peel_to_id()?.detach();
        let outcome = remote.push(
            [gix::push::Update {
                destination: "refs/heads/main".try_into()?,
                new_id: Some(head_id),
                lease: gix::push::Lease::Any,
                force: false,
            }],
            Default::default(),
            gix::progress::Discard,
            |_, _| Ok(()),
            &AtomicBool::new(false),
        )?;
        assert!(
            matches!(outcome.updates[0].status, gix::push::Status::Updated { .. }),
            "the configured receiver accepts the push"
        );
        assert_eq!(
            std::fs::read_to_string(&marker)?,
            "upload-pack:operation-value\nreceive-pack:operation-value\n",
            "{scheme} push honors its configured receive command"
        );
    }
    Ok(())
}
