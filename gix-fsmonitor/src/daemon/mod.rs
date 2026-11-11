//! Shared blocking daemon control for Git's hook v2 and native Simple IPC protocols.
//!
//! [`endpoint()`](crate::daemon::endpoint) selects a native IPC endpoint from caller-resolved repository paths.
//! [`execute()`](crate::daemon::execute) runs foreground serving, detached startup, and client commands from a server context.
//! Detached children run the current executable with the foreground argument prefix supplied by the
//! caller; they do not require the standalone `gix-fsmonitor` binary to be installed.

use std::{
    ffi::OsString,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use bstr::BString;
use gix_error::{ErrorExt, Result, ResultExt, bail, message, validation};

use crate::server::{self, ipc};

mod endpoint;
mod launch;

pub use endpoint::endpoint;

/// A foreground, detached-startup, or client command for a repository's filesystem monitor.
pub enum Command {
    /// Serve the worktree until interruption, idle shutdown, or endpoint/root loss.
    Run {
        /// Seconds without a client connection before shutdown; zero disables idle shutdown.
        idle_timeout: u64,
        /// An internal startup channel to receive the PID once native registration is complete.
        ready_file: Option<PathBuf>,
    },
    /// Start a detached child and wait for its native registration to complete.
    Start {
        /// The idle timeout passed to the child, in seconds; zero disables idle shutdown.
        idle_timeout: u64,
    },
    /// Request shutdown and wait for endpoint release.
    Stop,
    /// Succeed if a compatible daemon owns the endpoint.
    Status,
    /// Reset token history and write a fresh baseline response.
    Flush,
    /// Write the binary response for a token without starting a daemon.
    Query {
        /// The client's previous token; an empty or unknown token requests a full baseline.
        token: BString,
    },
    /// Write a hook response, starting a daemon if needed.
    Hook {
        /// The hook protocol version; only version 2 is supported.
        version: u8,
        /// The client's previous token.
        token: BString,
        /// The idle timeout passed to a new child, in seconds; zero disables idle shutdown.
        idle_timeout: u64,
    },
    /// Print the native endpoint followed by a newline.
    Path,
}

/// Execute `command`, writing only its requested output to `out`.
///
/// The caller resolves `context` and releases repository handles before serving or starting a child.
/// `reopen_git_dir` is the absolute Git-directory spelling to preserve when reopening in a child,
/// including symlink aliases used by conditional configuration. Poll `should_interrupt` during
/// foreground serving and startup so callers can supply their own cancellation or signal handling.
///
/// `child_arguments` is called at most once, only when detached startup is needed. It receives the
/// resolved context and must return arguments invoking the **current executable's** foreground
/// `run` command, including any repository/configuration options needed by that executable.
/// The shared launcher appends `--idle-timeout` and `--ready-file`; the foreground entrypoint must
/// accept both and pass them back through [`Command::Run`]. Repository redirection variables are
/// replaced with this repository's layout while other environment variables are inherited.
///
/// Startup waits for the child's PID readiness notification and kills and reaps an unsuccessful
/// child. An occupied endpoint is an error for explicit startup; hook autostart can query a daemon
/// that wins a concurrent startup race.
pub fn execute(
    context: server::Context,
    reopen_git_dir: &Path,
    command: Command,
    mut out: impl Write,
    mut should_interrupt: impl FnMut() -> bool,
    child_arguments: impl FnOnce(&server::Context) -> Result<Vec<OsString>>,
) -> Result<()> {
    if matches!(command, Command::Hook { version, .. } if version != 2) {
        bail!(validation("filesystem monitor hooks support only protocol version 2"));
    }

    match command {
        Command::Run {
            idle_timeout,
            ready_file,
        } => {
            let handle = server::spawn(
                context,
                server::Options {
                    idle_timeout: (idle_timeout != 0).then(|| Duration::from_secs(idle_timeout)),
                    ..Default::default()
                },
            )?;
            if let Some(path) = ready_file {
                std::fs::write(path, format!("{}\n", std::process::id()))
                    .or_raise(|| message("reporting filesystem monitor startup to the parent"))?;
            }
            // Only signal/exit observation is polled; the server's client and event loop is event-driven.
            while !handle.is_finished() {
                if should_interrupt() {
                    return handle.shutdown();
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            handle.wait()
        }
        Command::Start { idle_timeout } => start(
            &context,
            reopen_git_dir,
            child_arguments,
            idle_timeout,
            &mut should_interrupt,
        ),
        Command::Status => {
            if !ipc::is_running(&context.endpoint)? {
                bail!("filesystem monitor is not running".conflict());
            }
            Ok(())
        }
        Command::Path => writeln!(out, "{}", context.endpoint.display()).or_error(),
        Command::Stop => {
            ipc::query(&context.endpoint, ipc::Request::Quit)?;
            let deadline = Instant::now() + Duration::from_secs(5);
            while ipc::is_running(&context.endpoint)? {
                if Instant::now() >= deadline {
                    bail!("filesystem monitor did not stop before the shutdown deadline".conflict());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(())
        }
        Command::Flush => out
            .write_all(&ipc::query(&context.endpoint, ipc::Request::Flush)?)
            .or_error(),
        Command::Query { token } => out.write_all(&query(&context.endpoint, &token)?).or_error(),
        Command::Hook {
            token, idle_timeout, ..
        } => {
            let response = match query(&context.endpoint, &token) {
                Ok(response) => response,
                Err(_) => {
                    let started = start(
                        &context,
                        reopen_git_dir,
                        child_arguments,
                        idle_timeout,
                        &mut should_interrupt,
                    );
                    // A compatible daemon can win an autostart race; query that winner.
                    query(&context.endpoint, &token).map_err(|query_error| match started {
                        Ok(()) => query_error.and_raise(message("querying the filesystem monitor after startup")),
                        Err(start_error) => query_error
                            .and_raise(start_error)
                            .and_raise(message("starting and querying the filesystem monitor")),
                    })?
                }
            };
            out.write_all(&response).or_error()
        }
    }
}

fn query(endpoint: &Path, token: &[u8]) -> Result<Vec<u8>> {
    ipc::query(endpoint, ipc::Request::Message(token))
}

fn start(
    context: &server::Context,
    reopen_git_dir: &Path,
    child_arguments: impl FnOnce(&server::Context) -> Result<Vec<OsString>>,
    idle_timeout: u64,
    should_interrupt: &mut impl FnMut() -> bool,
) -> Result<()> {
    if ipc::is_running(&context.endpoint)? {
        bail!("a daemon already owns the native IPC endpoint".conflict());
    }
    let arguments = child_arguments(context)?;
    let notification = tempfile::Builder::new()
        .prefix("gix-fsmonitor-start-")
        .tempdir()
        .or_raise(|| message("creating a private filesystem monitor startup channel"))?;
    let ready_file = notification.path().join("ready");
    let mut child = launch::spawn(context, reopen_git_dir, &arguments, idle_timeout, &ready_file)?;
    let expected = format!("{}\n", child.id()).into_bytes();
    let deadline = Instant::now() + Duration::from_secs(10);
    let result = (|| {
        loop {
            if let Some(status) = child
                .try_wait()
                .or_raise(|| message("checking filesystem monitor startup"))?
            {
                bail!(message!("filesystem monitor exited during startup: {status}"));
            }
            match std::fs::read(&ready_file) {
                Ok(notification) if notification == expected => return Ok(()),
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error).or_raise(|| message("reading filesystem monitor startup notification"));
                }
            }
            if should_interrupt() {
                bail!("filesystem monitor startup interrupted".conflict());
            }
            if Instant::now() >= deadline {
                bail!("filesystem monitor did not become ready before the startup deadline".conflict());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    })();
    if result.is_err() {
        if let Err(error) = child.kill()
            && child
                .try_wait()
                .or_raise(|| message("checking the failed daemon child"))?
                .is_none()
        {
            return Err(error).or_raise(|| message("stopping the daemon after unsuccessful startup"));
        }
        child
            .wait()
            .or_raise(|| message("reaping the daemon after unsuccessful startup"))?;
    }
    result
}
