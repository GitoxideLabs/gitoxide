use crate::TestResult;
use bstr::ByteSlice;

#[test]
fn environment_variables_are_passed_one_by_one() -> TestResult {
    let out = gix_command::prepare("echo $FIRST $SECOND")
        .env("FIRST", "first")
        .env("SECOND", "second")
        .command_may_be_shell_script()
        .spawn()?
        .wait_with_output()?;
    assert_eq!(out.stdout.as_bstr(), "first second\n");
    Ok(())
}

#[test]
fn disallow_shell() -> TestResult {
    let out = gix_command::prepare("PATH= echo hi")
        .command_may_be_shell_script_disallow_manual_argument_splitting()
        .spawn()?
        .wait_with_output()?;
    assert_eq!(out.stdout.as_bstr(), "hi\n");

    let mut cmd: std::process::Command = gix_command::prepare("echo hi")
        .command_may_be_shell_script()
        .without_shell()
        .try_into()?;
    assert!(
        cmd.env_remove("PATH").spawn().is_err(),
        "no command named 'echo hi' exists"
    );
    Ok(())
}

#[test]
fn script_with_dollar_at() -> TestResult {
    let out = std::process::Command::try_from(
        gix_command::prepare(r#"echo "$@""#)
            .command_may_be_shell_script()
            .arg("arg"),
    )?
    .spawn()?
    .wait_with_output()?;
    assert_eq!(
        out.stdout.to_str_lossy().trim(),
        "arg arg",
        "Git appends arguments even when the script already expands them"
    );
    Ok(())
}

#[test]
fn direct_command_execution_searches_in_path() -> TestResult {
    assert!(
        gix_command::prepare(if cfg!(unix) { "ls" } else { "attrib.exe" })
            .spawn()?
            .wait()?
            .success()
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn direct_command_with_absolute_command_path() -> TestResult {
    assert!(gix_command::prepare("/usr/bin/env").spawn()?.wait()?.success());
    Ok(())
}

mod with_shell {
    use crate::TestResult;
    use gix_testtools::bstr::ByteSlice;

    #[test]
    fn command_in_path_with_args() -> TestResult {
        // `ls` is occasionaly a builtin, as in busybox ash, but it is usually external.
        assert!(
            gix_command::prepare(if cfg!(unix) { "ls -l" } else { "attrib.exe /d" })
                .command_may_be_shell_script()
                .spawn()?
                .wait()?
                .success()
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn shell_builtin_or_command_in_path() -> TestResult {
        let out = gix_command::prepare("echo")
            .command_may_be_shell_script()
            .spawn()?
            .wait_with_output()?;
        assert!(out.status.success());
        assert_eq!(out.stdout.as_bstr(), "\n");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn shell_builtin_or_command_in_path_with_single_extra_arg() -> TestResult {
        let out = gix_command::prepare("printf")
            .command_may_be_shell_script()
            .arg("1")
            .spawn()?
            .wait_with_output()?;
        assert!(out.status.success());
        assert_eq!(out.stdout.as_bstr(), "1");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn shell_builtin_or_command_in_path_with_multiple_extra_args() -> TestResult {
        let out = gix_command::prepare("printf")
            .command_may_be_shell_script()
            .arg("%s")
            .arg("arg")
            .spawn()?
            .wait_with_output()?;
        assert!(out.status.success());
        assert_eq!(out.stdout.as_bstr(), "arg");
        Ok(())
    }

    #[test]
    fn force_shell_builtin() -> TestResult {
        let out = gix_command::prepare("echo").with_shell().spawn()?.wait_with_output()?;
        assert!(out.status.success());
        assert_eq!(out.stdout.as_bstr(), "\n");
        Ok(())
    }

    #[test]
    fn force_shell_builtin_with_single_extra_arg() -> TestResult {
        let out = gix_command::prepare("printf")
            .with_shell()
            .arg("1")
            .spawn()?
            .wait_with_output()?;
        assert!(out.status.success());
        assert_eq!(out.stdout.as_bstr(), "1");
        Ok(())
    }

    #[test]
    fn force_shell_builtin_with_multiple_extra_args() -> TestResult {
        let out = gix_command::prepare("printf")
            .with_shell()
            .arg("%s")
            .arg("arg")
            .spawn()?
            .wait_with_output()?;
        assert!(out.status.success());
        assert_eq!(out.stdout.as_bstr(), "arg");
        Ok(())
    }

    #[test]
    fn sh_shell_specific_script_code() -> TestResult {
        assert!(
            gix_command::prepare(":;:;:")
                .command_may_be_shell_script()
                .spawn()?
                .wait()?
                .success()
        );
        Ok(())
    }

    #[test]
    fn sh_shell_specific_script_code_with_single_extra_arg() -> TestResult {
        let out = gix_command::prepare(":;printf")
            .command_may_be_shell_script()
            .arg("1")
            .spawn()?
            .wait_with_output()?;
        assert!(out.status.success());
        assert_eq!(out.stdout.as_bstr(), "1");
        Ok(())
    }

    #[test]
    fn sh_shell_specific_script_code_with_multiple_extra_args() -> TestResult {
        let out = gix_command::prepare(":;printf")
            .command_may_be_shell_script()
            .arg("%s")
            .arg("arg")
            .spawn()?
            .wait_with_output()?;
        assert!(out.status.success());
        assert_eq!(out.stdout.as_bstr(), "arg");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn dollar_zero_in_minus_c_is_the_original_command() -> TestResult {
        let out = gix_command::prepare(r#"printf %s "$0""#)
            .command_may_be_shell_script()
            .spawn()?
            .wait_with_output()?;
        assert_eq!(
            out.stdout.as_bstr(),
            r#"printf %s "$0""#,
            "Git passes the original command as the shell's argument zero"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn dollar_zero_in_minus_c_is_independent_of_the_shell_program() -> TestResult {
        let out = std::process::Command::try_from(
            gix_command::prepare(r#"printf %s "$0""#)
                .command_may_be_shell_script()
                .with_shell_program(gix_testtools::bash_program()),
        )?
        .spawn()?
        .wait_with_output()?;
        assert_eq!(
            out.stdout.as_bstr(),
            r#"printf %s "$0""#,
            "selecting a different shell does not change the command passed as argument zero"
        );
        Ok(())
    }
}

#[test]
fn shell_arguments_match_git() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only("shell-arguments.sh")?.join("repo");
    for script in [r#"printf '<%s>\n' "$0""#, r#"printf '<%s>\n' "$0" "$@""#] {
        for arguments in [&[][..], &["one argument", "literal $value"][..]] {
            let reference = gix_testtools::git_command(&root)
                .args(["-c", &format!("alias.inspect=!{script}"), "inspect"])
                .args(arguments)
                .output()?;
            assert!(reference.status.success(), "Git's shell alias runs: {reference:?}");
            let output = gix_command::prepare(script)
                .command_may_be_shell_script()
                .args(arguments.iter().copied())
                .spawn()?
                .wait_with_output()?;
            assert!(output.status.success(), "the prepared shell command runs");
            assert_eq!(
                output.stdout, reference.stdout,
                "$0 and automatic argument forwarding match Git for {script:?} with {arguments:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn child_directory_is_anchored_during_preparation() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only("shell-arguments.sh")?.canonicalize()?;
    let _cwd = gix_testtools::set_current_dir(&root)?;
    let script = "test -f .git/config && printf matched";
    let mut command = std::process::Command::try_from(
        gix_command::prepare(script)
            .command_may_be_shell_script()
            .current_dir("repo"),
    )?;
    let _moved = gix_testtools::set_current_dir(root.join("repo/.git"))?;
    assert_eq!(
        command.get_current_dir(),
        Some(root.join("repo").as_path()),
        "relative child directories are captured before the parent moves"
    );
    let actual = command.output()?;
    let expected = gix_testtools::git_command(root.join("repo"))
        .args(["-c", &format!("alias.inspect=!{script}"), "inspect"])
        .output()?;
    assert!(
        actual.status.success() && expected.status.success(),
        "both commands run from the selected repository"
    );
    assert_eq!(
        actual.stdout, expected.stdout,
        "child-directory shell execution matches Git -C"
    );
    assert_eq!(
        actual.stdout, b"matched",
        "the shell can read the repository config from its working directory"
    );
    Ok(())
}
