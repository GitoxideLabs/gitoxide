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
                command: Some(program.into_os_string()),
                ..Default::default()
            },
            trace: false,
        },
        &[
            ("GIX_TEST_SSH_VALUE".into(), "operation-value".into()),
            ("GIX_TEST_SSH_LOG".into(), log.clone().into_os_string()),
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
