mod options;

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
mod ask {
    use gix_testtools::bstr::ByteSlice;

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
    fn askpass_only() -> gix_testtools::Result {
        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.args(["build", "--example", "use-askpass", "--example", "askpass"]);
        assert!(
            cmd.status().expect("Cargo can build prompt examples").success(),
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
    fn username_password() -> gix_testtools::Result {
        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.args(["build", "--example", "credentials"]);
        assert!(
            cmd.status().expect("Cargo can build prompt examples").success(),
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
