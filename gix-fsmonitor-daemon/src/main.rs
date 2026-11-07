//! Standalone entrypoint for the embeddable filesystem monitor.

use anyhow::{Context, bail};
use gix_fsmonitor::server::{self, ipc};
#[cfg(unix)]
use std::process::{Child, Command, Stdio};
use std::{
    ffi::OsString,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

mod endpoint;
#[cfg(windows)]
mod launch;

const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(5);

fn layout(path: &Path) -> anyhow::Result<server::Context> {
    let repo = gix::discover(path).context("opening the repository for filesystem monitoring")?;
    let worktree = repo
        .workdir()
        .context("filesystem monitoring requires a worktree")?
        .canonicalize()?;
    let endpoint = endpoint::for_repository(&repo, &worktree)?;
    let capabilities = repo.filesystem_options().context("reading filesystem capabilities")?;
    Ok(server::Context {
        worktree,
        git_dir: repo.git_dir().canonicalize()?,
        common_dir: repo.common_dir().canonicalize()?,
        endpoint,
        precompose_unicode: capabilities.precompose_unicode,
        ignore_case: capabilities.ignore_case,
    })
}

fn query(endpoint: &Path, request: &[u8]) -> anyhow::Result<Vec<u8>> {
    Ok(ipc::query(endpoint, request)?)
}

fn is_running(endpoint: &Path) -> anyhow::Result<bool> {
    Ok(ipc::is_running(endpoint)?)
}

#[cfg(unix)]
fn spawn(repo: &Path) -> anyhow::Result<Child> {
    use std::os::unix::process::CommandExt;

    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("run")
        .arg("--repo")
        .arg(repo)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: Only the async-signal-safe setsid system call runs between fork and exec.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    command.spawn().context("starting the filesystem monitor")
}

#[cfg(windows)]
fn spawn(repo: &Path) -> anyhow::Result<launch::Child> {
    launch::spawn(repo).context("starting the filesystem monitor")
}

fn start(layout: &server::Context) -> anyhow::Result<()> {
    if is_running(&layout.endpoint)? {
        bail!("a daemon already owns the native IPC endpoint");
    }
    let mut child = spawn(&layout.worktree)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait()? {
            bail!("filesystem monitor exited during startup: {status}");
        }
        if query(&layout.endpoint, b"").is_ok() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("filesystem monitor did not become ready before the startup deadline");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn main() -> anyhow::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let Some(command) = arguments.next() else {
        bail!("usage: gix-fsmonitor <run|start|stop|status|flush|query|hook|path> [--repo PATH]");
    };
    if command == "--help" || command == "-h" {
        println!("gix-fsmonitor <run|start|stop|status|flush|query [TOKEN]|hook 2 TOKEN|path> [--repo PATH]");
        return Ok(());
    }
    let mut repo = PathBuf::from(".");
    let mut positional = Vec::<OsString>::new();
    while let Some(argument) = arguments.next() {
        if argument == "--repo" {
            repo = arguments.next().context("--repo needs a path")?.into();
        } else {
            positional.push(argument);
        }
    }
    let layout = layout(&repo)?;
    match command.to_str() {
        Some("run") if positional.is_empty() => server::spawn(layout)?.wait().map_err(Into::into),
        Some("start") if positional.is_empty() => start(&layout),
        Some("status") if positional.is_empty() => {
            if !is_running(&layout.endpoint)? {
                bail!("filesystem monitor is not running");
            }
            Ok(())
        }
        Some("path") if positional.is_empty() => {
            println!("{}", layout.endpoint.display());
            Ok(())
        }
        Some("stop") if positional.is_empty() => {
            query(&layout.endpoint, b"quit")?;
            let deadline = Instant::now() + EXCHANGE_TIMEOUT;
            while is_running(&layout.endpoint)? {
                if Instant::now() >= deadline {
                    bail!("filesystem monitor did not stop");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(())
        }
        Some("flush") if positional.is_empty() => {
            io::stdout().write_all(&query(&layout.endpoint, b"flush")?)?;
            Ok(())
        }
        Some("query") if positional.len() <= 1 => {
            let token = positional
                .first()
                .map(|value| value.as_encoded_bytes())
                .unwrap_or_default();
            io::stdout().write_all(&query(&layout.endpoint, token)?)?;
            Ok(())
        }
        Some("hook") if positional.len() == 2 && positional[0] == "2" => {
            let token = positional[1].as_encoded_bytes();
            let response = match query(&layout.endpoint, token) {
                Ok(response) => response,
                Err(_) => {
                    // An autostart race may be won by either compatible daemon; query the winner.
                    let _ = start(&layout);
                    query(&layout.endpoint, token)?
                }
            };
            io::stdout().write_all(&response)?;
            Ok(())
        }
        _ => bail!("invalid command or arguments; use --help"),
    }
}
