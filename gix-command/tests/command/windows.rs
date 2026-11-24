use bstr::ByteSlice;
use gix_testtools::TestResult;

#[test]
fn git_style_quoting_preserves_msys_ssh_arguments() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let printf = gix_path::env::installation_program("printf").expect("Git for Windows provides MSYS printf");
    let root = gix_testtools::tempfile::tempdir()?;
    let _cwd = gix_testtools::set_current_dir(root.path())?;
    std::fs::write(root.path().join("hostname.invalid"), [])?;
    let args = [
        "{first,second}",
        "host*.invalid",
        "host?ame.invalid",
        "host[n]ame.invalid",
        "O'Brien.invalid",
        "git-upload-pack 'team/repo with spaces.git'",
        r"git-upload-pack 'team/O'\''Brien.git'",
    ];
    let expected = format!("<{}>", args.join("><"));
    for mode in ["glob", "noglob"] {
        let output = gix_command::prepare(&printf)
            .env("MSYS", mode)
            .arg("<%s>")
            .args(args)
            .into_command_with_git_quoting()?
            .output()?;
        assert!(
            output.status.success(),
            "the MSYS argument recorder completes in mode {mode}"
        );
        assert_eq!(
            output.stdout.as_bstr(),
            expected.as_bytes().as_bstr(),
            "the SSH destination stays literal and the remote command arrives in one argument"
        );
    }
    Ok(())
}

#[test]
fn explicit_msys_ssh_path_can_omit_exe_suffix() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let ssh = gix_path::env::installation_program("ssh").expect("Git for Windows provides MSYS ssh");
    let mut cmd = gix_command::prepare(ssh.with_extension(""))
        .env("MSYS", "glob")
        .args(["-F", "/dev/null", "-G", "{first,second}"])
        .stderr(std::process::Stdio::piped())
        .into_command_with_git_quoting()?;
    assert_eq!(cmd.get_program(), ssh, "the .exe sibling selected at launch is used");
    let output = cmd.output()?;
    assert!(
        !output.status.success(),
        "the invalid hostname is rejected during the configuration probe"
    );
    assert!(
        output.stderr.contains_str("hostname contains invalid characters"),
        "OpenSSH sees the literal brace argument without a connection attempt"
    );
    Ok(())
}

#[test]
fn bundled_msys_shell_preserves_positional_arguments() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let args = ["{first,second}", "'repo name'", r"C:\example directory\", r#"a"b"#];
    let expected = format!("<{}>", args.join("><"));
    for mode in ["glob", "noglob"] {
        let output = gix_command::prepare(r#"printf '<%s>' "$@""#)
            .with_shell()
            .env("MSYS", mode)
            .args(args)
            .spawn()?
            .wait_with_output()?;
        assert!(output.status.success(), "the bundled shell completes in mode {mode}");
        assert_eq!(
            output.stdout.as_bstr(),
            expected.as_bytes().as_bstr(),
            "the shell preserves its positional arguments"
        );
    }
    Ok(())
}

#[test]
fn native_fallback_shell_retains_standard_argument_encoding() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::tempfile::tempdir()?;
    let _env = gix_testtools::Env::new().set("GIT_EXEC_PATH", root.path().to_string_lossy());
    std::fs::copy(std::env::current_exe()?, root.path().join("sh.exe"))?;
    assert_eq!(
        gix_path::env::shell(),
        "sh.exe",
        "the fixture disables bundled shell discovery"
    );

    let arg = r"C:\example directory\";
    let script = r#"printf '%s' "$1" "$@""#;
    let cmd = std::process::Command::try_from(
        gix_command::prepare(script)
            .with_shell()
            .env("PATH", root.path())
            .arg(arg),
    )?;
    assert_eq!(cmd.get_program(), "sh.exe", "Rust resolves the fallback shell at spawn");
    assert_eq!(
        cmd.get_args().collect::<Vec<_>>(),
        ["-c", script, "sh.exe", arg],
        "the native fallback receives ordinary Windows arguments"
    );
    Ok(())
}

#[test]
fn explicit_shell_does_not_discover_git_shell() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::tempfile::tempdir()?;
    let shell = std::env::current_exe()?;
    {
        let _env = gix_testtools::Env::new().set("GIT_EXEC_PATH", root.path().to_string_lossy());
        let cmd = std::process::Command::try_from(
            gix_command::prepare("echo hello")
                .with_shell()
                .with_shell_program(&shell),
        )?;
        assert_eq!(cmd.get_program(), shell, "the explicit absolute shell is used directly");
    }
    assert!(
        std::path::Path::new(gix_path::env::shell()).is_absolute(),
        "Git discovery still sees the normal installation after the temporary override is gone"
    );
    Ok(())
}

#[test]
fn git_style_quoting_preserves_rust_batch_escaping() -> TestResult {
    let root = gix_testtools::tempfile::tempdir()?;
    let file = root.path().join("helper.CmD");
    std::fs::write(&file, b"@exit /b 0\r\n")?;
    let arg = "'literal'";
    let cmd = gix_command::prepare(file)
        .env("PATH", "")
        .arg(arg)
        .into_command_with_git_quoting()?;
    assert_eq!(
        cmd.get_args().collect::<Vec<_>>(),
        [arg],
        "batch files retain Rust's regular arguments"
    );
    Ok(())
}

#[test]
fn quoted_git_transport_arguments_require_compatible_glob_mode() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let printf = gix_path::env::installation_program("printf").expect("Git for Windows provides MSYS printf");
    let result = gix_command::prepare(printf)
        .env("MSYS", "noglob")
        .args(["<%s>", r#"git-upload-pack 'team/repo with "quotes".git'"#])
        .into_command_with_git_quoting();
    assert!(
        result
            .expect_err("the parser would lose literal quotes")
            .is_validation()
    );
    Ok(())
}
