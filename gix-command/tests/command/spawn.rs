use bstr::ByteSlice;

#[test]
fn environment_variables_are_passed_one_by_one() -> crate::Result {
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
fn disallow_shell() -> crate::Result {
    let out = gix_command::prepare("PATH= echo hi")
        .command_may_be_shell_script_disallow_manual_argument_splitting()
        .spawn()?
        .wait_with_output()?;
    assert_eq!(out.stdout.as_bstr(), "hi\n");

    let mut cmd: std::process::Command = gix_command::prepare("echo hi")
        .command_may_be_shell_script()
        .without_shell()
        .into();
    assert!(
        cmd.env_remove("PATH").spawn().is_err(),
        "no command named 'echo hi' exists"
    );
    Ok(())
}

#[test]
fn script_with_dollar_at() -> crate::Result {
    let out = std::process::Command::from(
        gix_command::prepare(r#"echo "$@""#)
            .command_may_be_shell_script()
            .arg("arg"),
    )
    .spawn()?
    .wait_with_output()?;
    assert_eq!(
        out.stdout.to_str_lossy().trim(),
        "arg",
        "the argument is just mentioned once"
    );
    Ok(())
}

#[test]
fn direct_command_execution_searches_in_path() -> crate::Result {
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
fn direct_command_with_absolute_command_path() -> crate::Result {
    assert!(gix_command::prepare("/usr/bin/env").spawn()?.wait()?.success());
    Ok(())
}

mod with_shell {
    use gix_testtools::bstr::ByteSlice;

    #[test]
    fn command_in_path_with_args() -> crate::Result {
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
    fn shell_builtin_or_command_in_path() -> crate::Result {
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
    fn shell_builtin_or_command_in_path_with_single_extra_arg() -> crate::Result {
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
    fn shell_builtin_or_command_in_path_with_multiple_extra_args() -> crate::Result {
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
    fn force_shell_builtin() -> crate::Result {
        let out = gix_command::prepare("echo").with_shell().spawn()?.wait_with_output()?;
        assert!(out.status.success());
        assert_eq!(out.stdout.as_bstr(), "\n");
        Ok(())
    }

    #[test]
    fn force_shell_builtin_with_single_extra_arg() -> crate::Result {
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
    fn force_shell_builtin_with_multiple_extra_args() -> crate::Result {
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
    fn sh_shell_specific_script_code() -> crate::Result {
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
    fn sh_shell_specific_script_code_with_single_extra_arg() -> crate::Result {
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
    fn sh_shell_specific_script_code_with_multiple_extra_args() -> crate::Result {
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
    fn dollar_zero_in_minus_c_is_basename_of_default_shell() -> crate::Result {
        let out = gix_command::prepare(r#"printf %s "$0""#)
            .command_may_be_shell_script()
            .spawn()?
            .wait_with_output()?;
        assert_eq!(
            out.stdout.as_bstr(),
            "sh",
            "with the default shell on Unix, $0 should be the shell's basename, \
                 since that is the command_name passed after the -c operand"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn dollar_zero_in_minus_c_reflects_with_shell_program() -> crate::Result {
        let out = std::process::Command::from(
            gix_command::prepare(r#"printf %s "$0""#)
                .command_may_be_shell_script()
                .with_shell_program(gix_testtools::bash_program()),
        )
        .spawn()?
        .wait_with_output()?;
        assert_eq!(
            out.stdout.as_bstr(),
            "bash",
            "$0 should be the basename of the shell selected via with_shell_program, \
                 so a configured bash identifies as 'bash' rather than as some other shell"
        );
        Ok(())
    }
}
