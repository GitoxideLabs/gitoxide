#![cfg(feature = "pretty-cli")]

use std::{path::Path, process::Command};

use gix_testtools::TestResult;

fn command(directory: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_gix"));
    gix_testtools::configure_git_environment(&mut command, directory);
    command.current_dir(directory);
    command
}

#[test]
fn notify_help_and_validation_do_not_require_a_repository() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let output = command(directory.path()).args(["notify", "--help"]).output()?;
    assert!(
        output.status.success(),
        "raw notification help succeeds outside a repository: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8(output.stdout)?;
    for option in [
        "--non-recursive",
        "--max-events",
        "--max-bytes",
        "--batch-events",
        "--batch-bytes",
        "--root-check-ms",
    ] {
        assert!(
            help.contains(option),
            "notify exposes its coverage and bounded queue/drain options: {option}"
        );
    }
    for option in ["--max-events", "--max-bytes", "--batch-events", "--batch-bytes"] {
        let output = command(directory.path()).args(["notify", option, "0"]).output()?;
        assert_eq!(
            output.status.code(),
            Some(2),
            "zero notification queue and batch budgets fail during CLI parsing: {option}"
        );
    }
    let missing = directory.path().join("missing notification directory");
    let output = command(directory.path())
        .args(["notify", "--root-check-ms", "0"])
        .arg(&missing)
        .output()?;
    assert!(
        !output.status.success() && output.status.code() != Some(2),
        "zero disables periodic root checks and reaches path validation"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("missing notification directory"),
        "a missing notification path retains path context: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod native {
    use super::*;
    use std::{
        io::{self, BufRead, BufReader},
        path::PathBuf,
        process::{Child, Stdio},
        sync::mpsc::{Receiver, sync_channel},
        thread::JoinHandle,
        time::{Duration, Instant},
    };

    struct Cleanup(Child);

    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    struct Session {
        process: Cleanup,
        receive: Receiver<io::Result<String>>,
        reader: JoinHandle<()>,
        stderr: PathBuf,
        transcript: String,
    }

    impl Session {
        fn start(mut command: Command, stderr: PathBuf) -> TestResult<Self> {
            command
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::from(std::fs::File::create(&stderr)?));
            let mut process = Cleanup(command.spawn()?);
            let stdout = process.0.stdout.take().ok_or("notify pipes its output")?;
            let (send, receive) = sync_channel(128);
            let reader = std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines() {
                    let failed = line.is_err();
                    if send.send(line).is_err() || failed {
                        break;
                    }
                }
            });
            let mut session = Self {
                process,
                receive,
                reader,
                stderr,
                transcript: String::new(),
            };
            session.read_until(|line| line.contains("Rescan required: WatchSetChanged"))?;
            assert!(
                session.process.0.try_wait()?.is_none(),
                "registration publishes initial loss while the raw watcher remains active"
            );
            assert!(
                session.transcript.lines().any(|line| {
                    line.contains("Batch: generation=") && line.contains("sequence=") && line.contains("more=")
                }),
                "raw batches expose coverage generation, publication sequence, and pending work"
            );
            Ok(session)
        }

        fn read_until(&mut self, matches: impl Fn(&str) -> bool) -> TestResult {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let line = self
                    .receive
                    .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                    .map_err(|error| {
                        format!(
                            "notify does not stream its next update: {error}; output: {}; stderr: {}",
                            self.transcript,
                            std::fs::read_to_string(&self.stderr).unwrap_or_default()
                        )
                    })??;
                self.transcript.push_str(&line);
                self.transcript.push('\n');
                if matches(&line) {
                    return Ok(());
                }
            }
        }

        fn interrupt(mut self, directory: &Path) -> TestResult<String> {
            let mut interrupt = Command::new("kill");
            gix_testtools::configure_git_environment(&mut interrupt, directory);
            assert!(
                interrupt
                    .args(["-INT", &self.process.0.id().to_string()])
                    .output()?
                    .status
                    .success(),
                "Ctrl-C is delivered to the owned raw watcher"
            );
            let deadline = Instant::now() + Duration::from_secs(5);
            let status = loop {
                if let Some(status) = self.process.0.try_wait()? {
                    break status;
                }
                assert!(
                    Instant::now() < deadline,
                    "Ctrl-C promptly releases native resources: {}; stderr: {}",
                    self.transcript,
                    std::fs::read_to_string(&self.stderr)?
                );
                std::thread::sleep(Duration::from_millis(10));
            };
            assert!(
                status.success(),
                "Ctrl-C follows graceful raw watcher shutdown: {status}; stderr: {}",
                std::fs::read_to_string(&self.stderr)?
            );
            self.reader
                .join()
                .expect("the output reader finishes when notify closes stdout");
            for line in self.receive {
                self.transcript.push_str(&line?);
                self.transcript.push('\n');
            }
            Ok(self.transcript)
        }
    }

    fn is_event_for(line: &str, filename: &str) -> bool {
        line.contains(filename) && line.contains("): [")
    }

    #[test]
    fn default_directory_streams_raw_events_without_git_discovery_and_handles_ctrl_c() -> TestResult {
        let directory = gix_testtools::tempfile::tempdir()?;
        let watched = directory.path().join("ordinary directory");
        std::fs::create_dir(&watched)?;
        let mut command = command(&watched);
        command
            .arg("-v")
            .arg("-r")
            .arg(directory.path().join("not a repository"))
            .arg("notify");
        let mut session = Session::start(command, directory.path().join("stderr"))?;
        let child = watched.join("new child");
        std::fs::create_dir(&child)?;
        session.read_until(|line| is_event_for(line, "new child"))?;
        std::fs::write(child.join("covered-write"), "after registration\n")?;
        session.read_until(|line| is_event_for(line, "covered-write"))?;
        session.interrupt(&watched)?;
        Ok(())
    }

    #[test]
    fn nonrecursive_multiple_roots_report_direct_children() -> TestResult {
        let directory = gix_testtools::tempfile::tempdir()?;
        let first = directory.path().join("first ordinary directory");
        let second = directory.path().join("second ordinary directory");
        std::fs::create_dir(&first)?;
        std::fs::create_dir(&second)?;
        let mut command = command(directory.path());
        command
            .args(["-v", "notify", "--non-recursive"])
            .arg(&first)
            .arg(&second);
        let mut session = Session::start(command, directory.path().join("stderr"))?;
        let child = first.join("new-direct-child");
        std::fs::create_dir(&child)?;
        session.read_until(|line| is_event_for(line, "new-direct-child"))?;
        std::fs::write(child.join("uncovered-descendant"), "outside selected depth\n")?;
        std::fs::write(first.join("first-covered-write"), "direct child\n")?;
        session.read_until(|line| is_event_for(line, "first-covered-write"))?;
        std::fs::write(second.join("second-covered-write"), "another registered root\n")?;
        session.read_until(|line| is_event_for(line, "second-covered-write"))?;
        let transcript = session.interrupt(directory.path())?;
        assert!(
            !transcript.contains("uncovered-descendant"),
            "nonrecursive roots exclude descendant file paths while reporting later direct writes: {transcript}"
        );
        Ok(())
    }
}
