#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::{fs, process::Command};

#[test]
fn fatal_errors_are_saved_after_command_logging_ends() -> gix::error::TestResult {
    for args in [vec!["show"], vec!["--trace", "show"], vec!["-ttt", "show"]] {
        for writable in [true, false] {
            let home = gix_testtools::tempfile::TempDir::new()?;
            #[cfg(target_os = "macos")]
            let directory = home.path().join("Library/Logs/org.GitoxideLabs.tix");
            #[cfg(target_os = "linux")]
            let directory = home.path().join("state/tix");
            if !writable {
                fs::create_dir_all(directory.parent().expect("the log directory has a parent"))?;
                fs::write(&directory, "a file prevents log directory creation")?;
            }
            let mut command = Command::new(env!("CARGO_BIN_EXE_tix"));
            gix_testtools::configure_git_environment(&mut command, home.path())
                .current_dir(home.path())
                .env("HOME", home.path())
                .env("XDG_STATE_HOME", home.path().join("state"))
                .args(&args);
            let output = command.output()?;
            assert_eq!(output.status.code(), Some(1), "fatal errors retain their exit status");
            assert!(output.stdout.is_empty(), "diagnostics leave primary output empty");
            let stderr = String::from_utf8(output.stderr)?;
            assert!(
                stderr.contains("could not discover repository"),
                "the terminal retains the error: {stderr}"
            );

            if !writable {
                continue;
            }
            let entry = fs::read_dir(directory)?
                .next()
                .expect("the invocation creates a daily log")?;
            let log = fs::read_to_string(entry.path())?;
            assert!(
                log.contains("tix invocation failed"),
                "fatal errors reach the daily log with {args:?}: {log}"
            );
            assert!(
                log.contains("could not discover repository"),
                "the log retains the full error context: {log}"
            );
            assert!(
                log.contains("gix-tix/src/command.rs"),
                "the log retains captured caller locations: {log}"
            );
            assert!(
                log.contains("process_id="),
                "the log identifies the failing process: {log}"
            );
            assert!(
                log.contains("invocation_directory="),
                "the log identifies the launch directory: {log}"
            );
        }
    }
    Ok(())
}
