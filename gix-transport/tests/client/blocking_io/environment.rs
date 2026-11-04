#![cfg(unix)]

use gix_transport::{Protocol, Service, client::blocking_io::connect};

#[test]
fn operation_environment_reaches_ssh_probe_and_transport() -> gix_testtools::TestResult {
    use std::os::unix::fs::PermissionsExt;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _environment = gix_testtools::Env::new().set("GIX_TEST_SSH_VALUE", "process-value");
    let directory = gix_testtools::tempfile::tempdir()?;
    let program = directory.path().join("custom-ssh");
    let log = directory.path().join("invocations");
    std::fs::write(
        &program,
        "#!/bin/sh
set -eu
test \"$GIX_TEST_SSH_VALUE\" = operation-value
printf '%s\\n' \"$1\" >>\"$GIX_TEST_SSH_LOG\"
if test \"$1\" = -G; then exit 0; fi
printf '000eversion 2\\n0000'
",
    )?;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))?;
    let mut transport = connect::connect_with_environment(
        "ssh://host/repository",
        connect::Options {
            version: Protocol::V2,
            ssh: gix_transport::client::blocking_io::ssh::connect::Options {
                command: Some("./custom-ssh".into()),
                ..Default::default()
            },
            trace: false,
            working_dir: Some(directory.path().into()),
        },
        &[
            ("GIX_TEST_SSH_VALUE".into(), "operation-value".into()),
            ("GIX_TEST_SSH_LOG".into(), "invocations".into()),
        ],
    )?;
    assert_eq!(
        transport.handshake(Service::UploadPack, &[])?.actual_protocol,
        Protocol::V2
    );
    assert_eq!(
        std::fs::read_to_string(log)?,
        "-G
-o
"
    );
    assert_eq!(std::env::var("GIX_TEST_SSH_VALUE")?, "process-value");
    Ok(())
}

#[test]
fn operation_directory_resolves_relative_service_helpers() -> gix_testtools::TestResult {
    use std::os::unix::fs::PermissionsExt;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let directory = gix_testtools::tempfile::tempdir()?;
    let helpers = directory.path().join("helpers");
    std::fs::create_dir(&helpers)?;
    let program = helpers.join("git-upload-pack");
    std::fs::write(
        &program,
        "#!/bin/sh
printf '%s\\n' \"$1\" >invocation
printf '000eversion 2\\n0000'
",
    )?;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))?;
    let mut transport = connect::connect_with_environment(
        "../repository",
        connect::Options {
            version: Protocol::V2,
            working_dir: Some(directory.path().into()),
            ..Default::default()
        },
        &[("PATH".into(), "".into()), ("GIT_EXEC_PATH".into(), "helpers".into())],
    )?;
    assert_eq!(
        transport.handshake(Service::UploadPack, &[])?.actual_protocol,
        Protocol::V2
    );
    assert_eq!(
        std::fs::read_to_string(directory.path().join("invocation"))?,
        "../repository\n",
        "the relative helper runs in the operation directory and receives the original relative URL"
    );
    Ok(())
}
