#![cfg(all(feature = "pretty-cli", unix))]

use std::{
    io::{self, BufRead, BufReader},
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc::{Receiver, sync_channel},
    time::{Duration, Instant},
};

use gix_testtools::TestResult;

struct Cleanup(Child);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn verbose_watch_streams_invalidations_and_exits_successfully_on_ctrl_c() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let repository = directory.path().join("watched repository");
    std::fs::create_dir(&repository)?;
    gix_testtools::git(&repository, "init --initial-branch=main")?;
    std::fs::write(repository.join("watched"), "before\n")?;
    let stderr = directory.path().join("stderr");
    let mut command = Command::new(env!("CARGO_BIN_EXE_gix"));
    gix_testtools::configure_git_environment(&mut command, &repository);
    command
        .current_dir(&repository)
        .arg("-v")
        .arg("-r")
        .arg(&repository)
        .args(["watch", "--no-metadata", "--worktree-coverage", "all"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(std::fs::File::create(&stderr)?));
    let mut process = Cleanup(command.spawn()?);
    let stdout = process.0.stdout.take().ok_or("the watch command pipes its output")?;
    let (send, receive) = sync_channel(128);
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let failed = line.is_err();
            if send.send(line).is_err() || failed {
                break;
            }
        }
    });
    let mut transcript = String::new();
    read_until(&receive, &stderr, &mut transcript, |line| {
        line.contains("Raw watcher statistics")
    })?;
    assert!(
        process.0.try_wait()?.is_none(),
        "verbose watch publishes its initial service before the command finishes"
    );
    std::fs::write(repository.join("watched"), "after\n")?;
    read_until(&receive, &stderr, &mut transcript, |line| {
        line.contains("Selected worktree invalidations")
    })?;
    read_until(&receive, &stderr, &mut transcript, |line| {
        line.contains("watched") || line.trim() == "All,"
    })?;

    let mut interrupt = Command::new("kill");
    gix_testtools::configure_git_environment(&mut interrupt, &repository);
    let signalled = interrupt.args(["-INT", &process.0.id().to_string()]).output()?;
    assert!(
        signalled.status.success(),
        "Ctrl-C is delivered to the owned watch process"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = process.0.try_wait()? {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "Ctrl-C releases native watchers promptly; output: {transcript}; stderr: {}",
            std::fs::read_to_string(&stderr)?
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        status.success(),
        "Ctrl-C follows graceful watch shutdown; status: {status}; stderr: {}",
        std::fs::read_to_string(&stderr)?
    );
    reader
        .join()
        .expect("the output reader finishes when the watch process closes stdout");
    Ok(())
}

fn read_until(
    receive: &Receiver<io::Result<String>>,
    stderr: &Path,
    transcript: &mut String,
    matches: impl Fn(&str) -> bool,
) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let line = receive
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|error| {
                format!(
                    "watch does not stream its next update: {error}; output: {transcript}; stderr: {}",
                    std::fs::read_to_string(stderr).unwrap_or_default()
                )
            })??;
        transcript.push_str(&line);
        transcript.push('\n');
        if matches(&line) {
            return Ok(());
        }
    }
}
