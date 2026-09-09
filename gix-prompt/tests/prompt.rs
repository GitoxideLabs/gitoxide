mod options;

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
            },
        )
        .expect_err("terminal prompting is disabled");
        assert_eq!(err.error().to_string(), "Terminal prompts are disabled");
        assert_eq!(err.iter().count(), 1, "there is no underlying operation error");
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

        let output = jq_proc
            .wait_with_output()
            .expect(".target_directory is a valid search path for manifest format version 1");
        manifest_proc.wait().unwrap();

        output
            .stdout
            .trim()
            .to_str()
            .expect("value of target_directory is valid UTF8")
            .to_owned()
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
    fn askpass_only() {
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
        cmd.spawn().unwrap().wait().expect("example builds OK");

        let mut p = expectrl::spawn(evaluate_target_dir() + "/debug/examples/use-askpass").unwrap();
        p.expect("Password: ").unwrap();
        p.send_line(" password with space ").unwrap();
        p.expect("\" password with space \"").unwrap();
        p.expect(expectrl::Eof).unwrap();
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
    fn username_password() {
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
        cmd.spawn().unwrap().wait().expect("example builds OK");

        let mut p = expectrl::spawn(evaluate_target_dir() + "/debug/examples/credentials").unwrap();
        p.expect("Username: ").unwrap();
        p.send_line(" user with space ").unwrap();
        p.expect("\" user with space\"").unwrap();
        p.expect("Password: ").unwrap();
        p.send_line(" password with space ").unwrap();
        p.expect("\" password with space \"").unwrap();
        p.expect(expectrl::Eof).unwrap();
    }
}
