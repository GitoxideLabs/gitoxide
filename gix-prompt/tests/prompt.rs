mod options;

#[test]
#[cfg(unix)]
fn relative_askpass_uses_the_operation_directory_like_git() -> gix_testtools::TestResult {
    use std::{io::Write, process::Stdio};

    let fixture = gix_testtools::scripted_fixture_read_only("make_relative_askpass.sh")?;
    let working_dir = fixture.join("working directory").canonicalize()?;
    let mut reference = gix_testtools::git_command(&working_dir)
        .current_dir(&working_dir)
        .args(["-c", "credential.helper=", "credential", "fill"])
        .env("GIT_ASKPASS", "./askpass")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    reference
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(b"url=https://example.com\n")?;
    let reference = reference.wait_with_output()?;
    assert!(
        reference.status.success(),
        "Git's relative askpass succeeds: {reference:?}"
    );
    let reference = String::from_utf8(reference.stdout)?;
    let expected = reference
        .lines()
        .find_map(|line| line.strip_prefix("password="))
        .expect("Git prompted for a password");
    let process_dir = std::env::current_dir()?;
    let actual = gix_prompt::ask(
        "Password: ",
        &gix_prompt::Options {
            askpass: Some("./askpass".into()),
            current_dir: Some(working_dir),
            mode: gix_prompt::Mode::Disable,
        },
    )?;
    assert_eq!(
        actual, expected,
        "askpass resolves its executable and files from the operation directory"
    );
    assert_eq!(
        std::env::current_dir()?,
        process_dir,
        "askpass does not change the process directory"
    );
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
mod ask {
    use gix_testtools::bstr::ByteSlice;

    #[test]
    #[cfg(unix)]
    fn disabled_is_a_message_without_a_source() {
        let err = gix_prompt::ask(
            "ignored",
            &gix_prompt::Options {
                mode: gix_prompt::Mode::Disable,
                askpass: None,
                current_dir: None,
            },
        )
        .expect_err("terminal prompting is disabled");
        insta::assert_debug_snapshot!(err.error(), "disabled is a message without a source", @r#"
        Message {
            message: "Terminal prompts are disabled",
        }
        "#);
        assert_eq!(err.iter_errors().count(), 1, "there is no underlying operation error");
    }

    /// Evaluates Cargo's target directory for this project at runtime to adjust for the concrete
    /// execution environment. This is necessary because certain environment variables and
    /// configuration options can change its location (e.g. CARGO_TARGET_DIR).
    fn evaluate_target_dir() -> String {
        let mut manifest_proc = std::process::Command::new(env!("CARGO"))
            .args(["metadata", "--format-version", "1"])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let jq_proc = std::process::Command::new("jq")
            .args(["-r", ".target_directory"]) // -r makes it output raw strings
            .stdin(manifest_proc.stdout.take().unwrap())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("jq utility is available in PATH");

        let output = jq_proc.wait_with_output().expect("jq finishes reading Cargo metadata");
        assert!(
            manifest_proc
                .wait()
                .expect("Cargo metadata process can be waited on")
                .success(),
            "Cargo metadata must succeed before resolving the example executable"
        );
        assert!(output.status.success(), "jq extracts the target directory successfully");

        output
            .stdout
            .trim()
            .to_str()
            .expect("value of target_directory is valid UTF8")
            .to_owned()
    }

    fn spawn_example(name: &str) -> gix_testtools::Result<(gix_testtools::tempfile::TempDir, expectrl::Session)> {
        let temp = gix_testtools::tempfile::tempdir()?;
        let example = std::path::PathBuf::from(evaluate_target_dir())
            .join("debug/examples")
            .join(name);
        let mut cmd = std::process::Command::new(example);
        cmd.env_clear().envs(std::env::vars_os());
        gix_testtools::configure_git_environment(&mut cmd, temp.path());
        let session = expectrl::Session::spawn(cmd)?;
        Ok((temp, session))
    }

    #[test]
    fn askpass_only() -> gix_testtools::TestResult {
        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.args([
            "build",
            "-p",
            "gix-prompt",
            "-p",
            "gix-hash",
            "--features",
            "sha1",
            "--example",
            "use-askpass",
            "--example",
            "askpass",
        ]);
        assert!(
            cmd.status()?.success(),
            "prompt examples must build successfully before they run"
        );

        let (_temp, mut p) = spawn_example("use-askpass")?;
        p.expect("Password: ")?;
        p.send_line(" password with space ")?;
        p.expect("\" password with space \"")?;
        p.expect(expectrl::Eof)?;
        Ok(())
    }

    #[test]
    fn username_password() -> gix_testtools::TestResult {
        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.args([
            "build",
            "-p",
            "gix-prompt",
            "-p",
            "gix-hash",
            "--features",
            "sha1",
            "--example",
            "credentials",
        ]);
        assert!(
            cmd.status()?.success(),
            "prompt examples must build successfully before they run"
        );

        let (_temp, mut p) = spawn_example("credentials")?;
        p.expect("Username: ")?;
        p.send_line(" user with space ")?;
        p.expect("\" user with space\"")?;
        p.expect("Password: ")?;
        p.send_line(" password with space ")?;
        p.expect("\" password with space \"")?;
        p.expect(expectrl::Eof)?;
        Ok(())
    }
}
