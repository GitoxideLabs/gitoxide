mod options;

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

    #[test]
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
    fn askpass_only() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.args(["build", "--example", "use-askpass", "--example", "askpass"]);
        assert!(
            cmd.status().expect("Cargo can build prompt examples").success(),
            "prompt examples must build successfully before they run"
        );

        let mut p = expectrl::spawn(evaluate_target_dir() + "/debug/examples/use-askpass").unwrap();
        p.expect("Password: ").unwrap();
        p.send_line(" password with space ").unwrap();
        p.expect("\" password with space \"").unwrap();
        p.expect(expectrl::Eof).unwrap();
        Ok(())
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
    fn username_password() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.args(["build", "--example", "credentials"]);
        assert!(
            cmd.status().expect("Cargo can build prompt examples").success(),
            "prompt examples must build successfully before they run"
        );

        let mut p = expectrl::spawn(evaluate_target_dir() + "/debug/examples/credentials").unwrap();
        p.expect("Username: ").unwrap();
        p.send_line(" user with space ").unwrap();
        p.expect("\" user with space\"").unwrap();
        p.expect("Password: ").unwrap();
        p.send_line(" password with space ").unwrap();
        p.expect("\" password with space \"").unwrap();
        p.expect(expectrl::Eof).unwrap();
        Ok(())
    }
}
