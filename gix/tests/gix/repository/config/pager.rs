use std::{io::Write, process::Stdio};

use gix_testtools::TestResult;

#[test]
#[serial_test::serial]
fn pager_command_streams_input_and_honors_configuration_trust() -> TestResult {
    let _environment = gix_testtools::isolate_git_environment()?;
    let mut repo = crate::util::named_repo("make_basic_repo.sh")?;
    assert!(
        repo.pager_command()?.is_none(),
        "no configured pager leaves display to the caller"
    );
    for value in ["", "cat"] {
        repo.config_snapshot_mut().set_raw_value("core.pager", value)?;
        assert!(
            repo.pager_command()?.is_none(),
            "{value:?} explicitly disables the pager"
        );
    }
    repo.config_snapshot_mut()
        .set_raw_value("core.pager", r#"printf 'pager:%s\n' "$GIT_PAGER_IN_USE"; cat"#)?;
    let mut untrusted = gix_config::File::new(
        gix_config::file::Metadata::from(gix_config::Source::Local).with(gix_sec::Trust::Reduced),
    );
    untrusted.set_raw_value("core.pager", "exit 99")?;
    repo.config_snapshot_mut().append(untrusted)?;

    let mut command: std::process::Command = repo
        .pager_command()?
        .expect("the trusted pager remains selected")
        .try_into()?;
    let environment: Vec<_> = command
        .get_envs()
        .filter_map(|(key, value)| value.map(|value| (key.to_owned(), value.to_owned())))
        .collect();
    gix_testtools::configure_git_environment(&mut command, repo.current_dir()).envs(environment);
    let mut child = command.stdout(Stdio::piped()).spawn()?;
    child
        .stdin
        .take()
        .expect("pagers accept a pipe")
        .write_all(b"diff --git a/file b/file\n+\xff\n")?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "the trusted shell pager completes: {output:?}");
    assert_eq!(
        output.stdout, b"pager:true\ndiff --git a/file b/file\n+\xff\n",
        "the configured pager receives the original patch bytes"
    );
    Ok(())
}
